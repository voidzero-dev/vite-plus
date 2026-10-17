import { execFile } from 'node:child_process';
import { copyFile, mkdir, mkdtemp, readFile, readdir, rm, stat, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs, promisify } from 'node:util';

const exec = promisify(execFile);
const repo = process.cwd();
const { findMaxSatisfying } = createRequire(join(repo, 'packages/tools/package.json'))('verkit');
const nativeName = '@voidzero-dev/vite-plus-linux-x64-gnu';
const nativeFile = 'vite-plus.linux-x64-gnu.node';
const json = async (file) => JSON.parse(await readFile(file, 'utf8'));
const saveJson = (file, value) => writeFile(file, `${JSON.stringify(value, null, 2)}\n`);
const run = (file, args, cwd = repo) =>
  exec(file, args, { cwd, timeout: 300_000, maxBuffer: 128 * 1024 ** 2 });

async function removeBindings(directory) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const file = join(directory, entry.name);
    if (entry.isDirectory()) {
      await removeBindings(file);
    } else if (entry.name.endsWith('.node')) {
      await rm(file);
    }
  }
}

// Run only after the binary measurements. This prepares the disposable CI
// checkout like the release pack job, which ships native addons separately.
async function packReleasePackages(destination) {
  const cliPath = join(repo, 'packages/cli/package.json');
  const corePath = join(repo, 'packages/core/package.json');
  const original = await json(cliPath);
  const require = createRequire(cliPath);
  const { parseTriple } = require('@napi-rs/cli');
  const artifacts = join(repo, 'packages/cli/artifacts');
  await mkdir(artifacts, { recursive: true });
  await copyFile(join(repo, 'packages/cli/binding', nativeFile), join(artifacts, nativeFile));

  // This job builds only Linux x64. Use the normal release preparation for
  // that target, then restore all platform declarations before packing.
  await saveJson(cliPath, {
    ...original,
    napi: { ...original.napi, targets: ['x86_64-unknown-linux-gnu'] },
  });
  await run(process.execPath, ['packages/cli/publish-native-addons.ts', '--mode', 'pkg-pr-new']);
  const pins = Object.fromEntries(
    original.napi.targets.map((target) => [
      `${original.napi.packageName}-${parseTriple(target).platformArchABI}`,
      original.version,
    ]),
  );
  const cli = await json(cliPath);
  await saveJson(cliPath, {
    ...cli,
    napi: original.napi,
    optionalDependencies: { ...cli.optionalDependencies, ...pins },
  });
  const core = await json(corePath);
  await saveJson(corePath, {
    ...core,
    optionalDependencies: { ...core.optionalDependencies, ...pins },
  });
  await removeBindings(join(repo, 'packages/cli/binding'));
  await removeBindings(join(repo, 'packages/core/dist'));
  await run('pnpm', [
    '--filter',
    'vite-plus',
    '--filter',
    '@voidzero-dev/vite-plus-core',
    'pack',
    '--pack-destination',
    destination,
  ]);
  await run(
    'pnpm',
    ['pack', '--pack-destination', destination],
    join(repo, 'packages/cli/npm/linux-x64-gnu'),
  );
}

// Same metric as npmx: sum dist.unpackedSize for the package and its
// production/optional dependency graph on linux-x64-glibc. Resolve npm aliases
// to their real package names so the bundled core package is included.
// https://github.com/npmx-dev/npmx.dev/blob/eafa727d13b1296938fb9f6d1ca0d8899f6bdf93/server/utils/dependency-resolver.ts
export function matchesPlatform(manifest) {
  return Object.entries({ os: 'linux', cpu: 'x64', libc: 'glibc' }).every(([key, value]) => {
    const allowed = manifest[key];
    if (!Array.isArray(allowed) || allowed.length === 0) {
      return true;
    }
    return (
      !allowed.includes(`!${value}`) &&
      (allowed.every((item) => item.startsWith('!')) || allowed.includes(value))
    );
  });
}

function dependencySpec(name, range) {
  if (!range.startsWith('npm:')) {
    return { name, range };
  }
  const spec = range.slice(4);
  const separator = spec.lastIndexOf('@');
  return separator > 0
    ? { name: spec.slice(0, separator), range: spec.slice(separator + 1) }
    : { name: spec, range: '*' };
}

export async function calculateUnpackedSize(name, version, loadPackument) {
  const packages = new Map();
  const seen = new Set();
  let pending = [{ name, range: version, optional: false }];
  while (pending.length > 0) {
    const next = new Map();
    for (const item of pending) {
      seen.add(item.name);
    }
    // Keep traversal deterministic while limiting concurrent registry requests.
    for (let index = 0; index < pending.length; index += 20) {
      const batch = pending.slice(index, index + 20);
      const packuments = await Promise.all(batch.map((item) => loadPackument(item.name)));
      for (const [offset, item] of batch.entries()) {
        const packument = packuments[offset];
        const versions = Object.keys(packument.versions);
        const resolved = versions.includes(item.range)
          ? item.range
          : (packument['dist-tags']?.[item.range] ?? findMaxSatisfying(versions, item.range));
        if (!resolved) {
          // Locally packed previews declare optional binaries for other targets
          // that this Linux-only job does not build or publish.
          if (item.optional) {
            continue;
          }
          throw new Error(`Cannot resolve ${item.name}@${item.range}`);
        }
        const manifest = packument.versions[resolved];
        if (!matchesPlatform(manifest)) {
          continue;
        }
        packages.set(item.name, {
          name: item.name,
          version: resolved,
          size: manifest.dist?.unpackedSize ?? 0,
          missingSize: manifest.dist?.unpackedSize === undefined,
        });
        const optional = manifest.optionalDependencies ?? {};
        for (const [dependency, range] of Object.entries({
          ...manifest.dependencies,
          ...optional,
        })) {
          const spec = dependencySpec(dependency, range);
          if (!seen.has(spec.name) && !next.has(spec.name)) {
            next.set(spec.name, { ...spec, optional: Object.hasOwn(optional, dependency) });
          }
        }
      }
    }
    pending = [...next.values()];
  }
  const self = packages.get(name);
  if (!self) {
    throw new Error(`Root package ${name}@${version} does not support Linux x64 glibc`);
  }
  const dependencies = [...packages.values()]
    .filter((item) => item.name !== name)
    .toSorted((a, b) => b.size - a.size || a.name.localeCompare(b.name));
  return {
    package: name,
    version,
    platform: 'linux-x64-glibc',
    selfSize: self.size,
    totalSize: self.size + dependencies.reduce((total, dependency) => total + dependency.size, 0),
    dependencyCount: dependencies.length,
    dependencies,
  };
}

// Tarball payload bytes match npm's unpackedSize. Directory allocation, tar
// headers, install scripts, and package-manager files do not contribute.
async function unpackedSize(directory) {
  let size = 0;
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      size += await unpackedSize(path);
    } else if (entry.isFile()) {
      size += (await stat(path)).size;
    }
  }
  return size;
}

async function main() {
  const { values } = parseArgs({
    options: { output: { type: 'string' }, 'packages-dir': { type: 'string' } },
  });
  if (process.platform !== 'linux' || process.arch !== 'x64' || !values.output) {
    throw new Error('Run on Linux x64 with --output <size-json>');
  }
  const temporary = await mkdtemp(join(tmpdir(), 'vp-unpacked-size-'));
  try {
    const directory = values['packages-dir']
      ? resolve(values['packages-dir'])
      : join(temporary, 'packages');
    if (!values['packages-dir']) {
      await mkdir(directory);
      await packReleasePackages(directory);
    }
    const local = new Map();
    for (const file of await readdir(directory)) {
      if (!file.endsWith('.tgz')) {
        continue;
      }
      const extracted = await mkdtemp(join(temporary, 'unpacked-'));
      const archive = join(directory, file);
      await run('tar', ['-xzf', archive, '-C', extracted]);
      const root = join(extracted, 'package');
      const manifest = await json(join(root, 'package.json'));
      local.set(manifest.name, {
        name: manifest.name,
        versions: {
          [manifest.version]: { ...manifest, dist: { unpackedSize: await unpackedSize(root) } },
        },
      });
    }
    const cli = local.get('vite-plus');
    if (!cli || !local.has('@voidzero-dev/vite-plus-core') || !local.has(nativeName)) {
      throw new Error('Expected packed CLI, core, and Linux x64 native packages');
    }
    const cache = new Map();
    const load = (name) => {
      if (local.has(name)) {
        return local.get(name);
      }
      if (!cache.has(name)) {
        cache.set(
          name,
          (async () => {
            const response = await fetch(`https://registry.npmjs.org/${encodeURIComponent(name)}`, {
              signal: AbortSignal.timeout(60_000),
            });
            if (!response.ok) {
              throw new Error(`Registry request failed for ${name}: ${response.status}`);
            }
            return response.json();
          })(),
        );
      }
      return cache.get(name);
    };
    const measurement = await calculateUnpackedSize(
      'vite-plus',
      Object.keys(cli.versions)[0],
      load,
    );
    for (const name of ['@voidzero-dev/vite-plus-core', nativeName]) {
      if (!measurement.dependencies.some((dependency) => dependency.name === name)) {
        throw new Error(`Unpacked total omitted ${name}`);
      }
    }
    const result = await json(values.output);
    delete result.artifacts.cliInstalled;
    result.artifacts.cliUnpacked = measurement;
    await saveJson(values.output, result);
    console.log(JSON.stringify(measurement, null, 2));
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main();
}

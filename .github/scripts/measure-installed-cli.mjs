import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { copyFile, mkdir, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { parseArgs, promisify } from 'node:util';

const exec = promisify(execFile);
const { values } = parseArgs({
  options: { output: { type: 'string' }, 'packages-dir': { type: 'string' } },
});
if (process.platform !== 'linux' || process.arch !== 'x64' || !values.output) {
  throw new Error('Run on Linux x64 with --output <size-json>');
}

const repo = process.cwd();
const temporary = await mkdtemp(join(tmpdir(), 'vp-installed-size-'));
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

async function diskSize(directory, apparent = false) {
  const { stdout } = await run('du', [
    '--summarize',
    '--block-size=1',
    ...(apparent ? ['--apparent-size'] : []),
    directory,
  ]);
  return Number(stdout.split(/\s/)[0]);
}

try {
  const packages = values['packages-dir']
    ? resolve(values['packages-dir'])
    : join(temporary, 'packages');
  if (!values['packages-dir']) {
    await mkdir(packages);
    await packReleasePackages(packages);
  }
  const manifests = new Map();
  for (const file of await readdir(packages)) {
    if (file.endsWith('.tgz')) {
      const archive = join(packages, file);
      const { stdout } = await run('tar', ['-xzOf', archive, 'package/package.json']);
      const manifest = JSON.parse(stdout);
      manifests.set(manifest.name, { manifest, archive });
    }
  }
  const cli = manifests.get('vite-plus');
  const core = manifests.get('@voidzero-dev/vite-plus-core');
  const native = manifests.get(nativeName);
  if (!cli || !core || !native) {
    throw new Error('Expected packed CLI, core, and Linux x64 native packages');
  }
  const project = join(temporary, 'project');
  await mkdir(project);
  await saveJson(join(project, 'package.json'), {
    private: true,
    dependencies: { 'vite-plus': cli.manifest.version },
  });
  // Use the repository's registry fixture so aliases and native dependencies
  // resolve to this build. The npm cache lives outside the measured tree.
  await run(
    process.execPath,
    [
      join(repo, 'packages/tools/src/local-npm-registry.ts'),
      '--packages-dir',
      packages,
      '--',
      'npm',
      'install',
      '--omit=dev',
      '--legacy-peer-deps',
      '--no-audit',
      '--no-fund',
      '--cache',
      join(temporary, 'npm-cache'),
    ],
    project,
  );
  const modules = join(project, 'node_modules');
  const installedCli = join(modules, 'vite-plus');
  const installedCore = await json(join(modules, 'vite/package.json'));
  if (
    installedCore.name !== core.manifest.name ||
    installedCore.version !== core.manifest.version
  ) {
    throw new Error('The installation did not use the packed core package');
  }
  const { stdout: packedNative } = await exec(
    'tar',
    ['-xzOf', native.archive, `package/${nativeFile}`],
    {
      encoding: 'buffer',
      maxBuffer: 128 * 1024 ** 2,
    },
  );
  const installedNative = await readFile(join(modules, nativeName, nativeFile));
  if (!installedNative.equals(packedNative)) {
    throw new Error('The installation did not use the packed native addon');
  }
  const [raw, disk, cliRaw, cliDisk, npm] = await Promise.all([
    diskSize(modules, true),
    diskSize(modules),
    diskSize(installedCli, true),
    diskSize(installedCli),
    run('npm', ['--version']),
  ]);
  const result = await json(values.output);
  result.artifacts.cliInstalled = {
    raw,
    disk,
    cliRaw,
    cliDisk,
    dependenciesRaw: raw - cliRaw,
    dependenciesDisk: disk - cliDisk,
    nodeVersion: process.version,
    npmVersion: npm.stdout.trim(),
    nativeSha256: createHash('sha256').update(installedNative).digest('hex'),
  };
  await saveJson(values.output, result);
  console.log(JSON.stringify(result.artifacts.cliInstalled, null, 2));
} finally {
  await rm(temporary, { recursive: true, force: true });
}

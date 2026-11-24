import {
  copyFileSync,
  existsSync,
  chmodSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { readdir } from 'node:fs/promises';
import { join } from 'node:path';

import { NapiCli, parseTriple } from '@napi-rs/cli';

import { publishNpmPackageFromEnv } from '../../.github/scripts/publish-npm-package.ts';
import type { NpmPackageVersion } from '../../.github/scripts/wait-for-npm-packages.ts';
import { waitForNpmPackagesFromEnv } from '../../.github/scripts/wait-for-npm-packages.ts';
import { INSTALL_LOCKFILE, validateInstallLockfile } from './generate-install-lockfile.ts';
import pkg from './package.json' with { type: 'json' };
import { isObjectValue } from './src/utils/json-value.ts';
import { editJsonFile, readJsonFile } from './src/utils/json.ts';

const cli = new NapiCli();

const currentDir = import.meta.dirname;
const repoRoot = join(currentDir, '..', '..');
const nodeVersion = readFileSync(join(repoRoot, '.node-version'), 'utf8').trim();
if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(nodeVersion)) {
  throw new Error('.node-version must contain an exact stable Node.js version');
}

const args = process.argv.slice(2);
const modeIdx = args.indexOf('--mode');
const mode = modeIdx >= 0 ? args[modeIdx + 1] : null;
if (mode !== 'npm' && mode !== 'pkg-pr-new') {
  console.error(`Usage: publish-native-addons.ts --mode <npm|pkg-pr-new>`);
  process.exit(1);
}
const skipNpmPublish = mode === 'pkg-pr-new';
const phaseIdx = args.indexOf('--phase');
const phase = phaseIdx >= 0 ? args[phaseIdx + 1] : 'all';
if (phase !== 'native' && phase !== 'cli' && phase !== 'all') {
  throw new Error('--phase must be native, cli, or all');
}
const lockfile = join(repoRoot, 'binary-release', INSTALL_LOCKFILE);

const VERSION = process.env.VERSION;
if (!VERSION) {
  console.error('VERSION env var must be set');
  process.exit(1);
}

// Move downloaded Rust CLI binaries into target/<triple>/release/ where the
// rest of this script (and napi-cli) expects them.
const rustCliArtifactsDir = join(repoRoot, 'rust-cli-artifacts');
if (existsSync(rustCliArtifactsDir)) {
  for (const dir of await readdir(rustCliArtifactsDir)) {
    if (!dir.startsWith('vp-global-cli-')) {
      continue;
    }
    const target = dir.slice('vp-global-cli-'.length);
    const releaseDir = join(repoRoot, 'target', target, 'release');
    mkdirSync(releaseDir, { recursive: true });
    for (const file of await readdir(join(rustCliArtifactsDir, dir))) {
      copyFileSync(join(rustCliArtifactsDir, dir, file), join(releaseDir, file));
    }
  }
}

const npmDir = join(currentDir, 'npm');
if (phase !== 'cli') {
  // Create npm directories for NAPI bindings
  await cli.createNpmDirs({
    cwd: currentDir,
    packageJsonPath: './package.json',
  });

  // Copy NAPI artifacts
  await cli.artifacts({
    cwd: currentDir,
    packageJsonPath: './package.json',
  });

  // Pre-publish (Update package.json and copy addons into per platform packages)
  await cli.prePublish({
    cwd: currentDir,
    packageJsonPath: './package.json',
    tagStyle: 'npm',
    ghRelease: false,
    skipOptionalPublish: true,
  });

  const platformDirs = await readdir(npmDir);

  // The native binding's true ABI floor is Node 20, well below the product
  // support policy copied into `engines.node` (e.g. `^20.19.0 || ^22.18.0 ||
  // >=24.11.0`). Declaring that policy on the platform packages makes engine-strict
  // package managers (pnpm) skip the optional native dependency whenever a
  // consumer's declared Node floor lands in one of the policy's gaps (20.0-20.18,
  // 22.0-22.17, 24.0-24.10), surfacing as "Cannot find native binding". Rewrite each
  // platform package to its real ABI floor of `>=20.0.0` so the native dep is never
  // skipped. `packages/cli/package.json` and `packages/core/package.json` keep the
  // product policy unchanged.
  for (const dir of platformDirs) {
    editJsonFile(join(npmDir, dir, 'package.json'), (pkgJson) => ({
      ...pkgJson,
      engines: { ...(isObjectValue(pkgJson.engines) && pkgJson.engines), node: '>=20.0.0' },
    }));
  }
}

// Fresh read: napi-rs prePublish rewrote this package.json on disk, so the
// top-level `pkg` import is stale for injected fields.
const cliPackageJson = readJsonFile(join(currentDir, 'package.json'));
// Lockstep versioning: every generated platform package uses the CLI version.
if (typeof cliPackageJson.version !== 'string') {
  throw new TypeError('Expected a version in packages/cli/package.json');
}
const cliVersion = cliPackageJson.version;

// napi-rs prePublish injects the platform packages into this package's
// `optionalDependencies`. Release builds of core rewrite bundled Rolldown's
// binding requires to the same platform packages (see
// packages/core/build-support/rewrite-rolldown-binding.ts), so core must
// declare them too; napi-rs manages a single package, so mirror the injected
// entries into core with identical pins. Like the CLI's entries, these live
// only in the publish working tree, never in the committed package.json.
const nativePlatformPins: Record<string, string> = {};
for (const target of pkg.napi.targets) {
  const packageName = `${pkg.napi.packageName}-${parseTriple(target).platformArchABI}`;
  const optional = cliPackageJson.optionalDependencies;
  const pin = isObjectValue(optional) ? optional[packageName] : undefined;
  if (typeof pin !== 'string' || !pin) {
    console.error(
      `napi prePublish did not inject ${packageName} into packages/cli/package.json optionalDependencies`,
    );
    process.exit(1);
  }
  nativePlatformPins[packageName] = pin;
}
editJsonFile(join(repoRoot, 'packages', 'core', 'package.json'), (corePkgJson) => ({
  ...corePkgJson,
  optionalDependencies: {
    ...(isObjectValue(corePkgJson.optionalDependencies) && corePkgJson.optionalDependencies),
    ...nativePlatformPins,
  },
}));
const platformPackages: Array<NpmPackageVersion> = [];

// Publish each NAPI platform package (without vp binary)
const npmTag = process.env.NPM_TAG || 'latest';
const publishArgs = ['publish', '--tag', npmTag, '--access', 'public'];
if (!skipNpmPublish && phase !== 'cli') {
  for (const file of await readdir(npmDir)) {
    const platformDir = join(npmDir, file);
    const { name, version } = readJsonFile(join(platformDir, 'package.json'));
    if (typeof name !== 'string' || typeof version !== 'string') {
      throw new TypeError(`Expected package name and version in ${platformDir}/package.json`);
    }
    const platformPackage = { name, version };
    await publishNpmPackageFromEnv(platformPackage, 'npm', publishArgs, platformDir);
    platformPackages.push(platformPackage);
  }
}

// Create and publish separate @voidzero-dev/vite-plus-cli-{platform} packages
const cliNpmDir = join(currentDir, 'cli-npm');
if (phase !== 'native') {
  for (const napiTarget of pkg.napi.targets) {
    await prepareAndPublishCliPackage(napiTarget);
  }
}

// npm can accept uploads before scanning makes them installable. Wait for the
// platform packages before publishing core and the CLI, which pin their versions.
if (!skipNpmPublish) {
  await waitForNpmPackagesFromEnv(platformPackages);

  // Preview releases still need the prepared directories.
  rmSync(cliNpmDir, { recursive: true, force: true });
}

async function prepareAndPublishCliPackage(napiTarget: string): Promise<void> {
  const { platform, arch, abi, platformArchABI } = parseTriple(napiTarget);
  const isWindows = platform === 'win32';
  const binaryName = isWindows ? 'vp.exe' : 'vp';
  const rustBinarySource = join(repoRoot, 'target', napiTarget, 'release', binaryName);

  if (!existsSync(rustBinarySource)) {
    // eslint-disable-next-line no-console
    console.warn(
      `Warning: Rust binary not found at ${rustBinarySource}, skipping CLI package for ${platformArchABI}`,
    );
    return;
  }

  const platformCliDir = join(cliNpmDir, platformArchABI);
  mkdirSync(platformCliDir, { recursive: true });

  copyFileSync(rustBinarySource, join(platformCliDir, binaryName));
  if (!isWindows) {
    chmodSync(join(platformCliDir, binaryName), 0o755);
  }

  const files = [binaryName];
  if (!skipNpmPublish) {
    validateInstallLockfile(readFileSync(lockfile, 'utf8'), cliVersion);
    copyFileSync(lockfile, join(platformCliDir, INSTALL_LOCKFILE));
    files.push(INSTALL_LOCKFILE);
  }
  if (isWindows) {
    // The trampoline replaces .cmd wrappers to avoid "Terminate batch job (Y/N)?"
    // on Ctrl+C (see issue #835).
    const shimName = 'vp-shim.exe';
    const shimSource = join(repoRoot, 'target', napiTarget, 'release', shimName);
    if (!existsSync(shimSource)) {
      console.error(
        `Error: ${shimName} does not exist at ${shimSource}. Run "node packages/tools/src/build-trampoline.ts --release --target ${napiTarget}" first.`,
      );
      process.exit(1);
    }
    copyFileSync(shimSource, join(platformCliDir, shimName));
    files.push(shimName);
  }

  // Generate package.json
  const cliPackage = {
    name: `@voidzero-dev/vite-plus-cli-${platformArchABI}`,
    version: cliVersion,
    os: [platform],
    cpu: [arch],
    libc: abi ? [abi] : undefined,
    files,
    vpNodeVersion: nodeVersion,
    vpInstallLockfile: skipNpmPublish ? undefined : true,
    description: `Vite+ CLI binary for ${platformArchABI}`,
    repository: cliPackageJson.repository,
  };
  writeFileSync(join(platformCliDir, 'package.json'), JSON.stringify(cliPackage, null, 2) + '\n');
  platformPackages.push(cliPackage);

  if (skipNpmPublish) {
    // eslint-disable-next-line no-console
    console.log(
      `Prepared CLI package: @voidzero-dev/vite-plus-cli-${platformArchABI}@${cliVersion}`,
    );
    return;
  }

  const result = await publishNpmPackageFromEnv(cliPackage, 'npm', publishArgs, platformCliDir);

  if (result === 'published') {
    // eslint-disable-next-line no-console
    console.log(`Published CLI package: @voidzero-dev/vite-plus-cli-${platform}@${cliVersion}`);
  }
}

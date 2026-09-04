import { copyFileSync, existsSync, chmodSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { readdir } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { NapiCli, parseTriple } from '@napi-rs/cli';

import { publishNpmPackageFromEnv } from '../../.github/scripts/publish-npm-package.ts';
import { waitForNpmPackagesFromEnv } from '../../.github/scripts/wait-for-npm-packages.ts';
import pkg from './package.json' with { type: 'json' };
import { editJsonFile, readJsonFile } from './src/utils/json.ts';

const cli = new NapiCli();

const currentDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(currentDir, '..', '..');

const args = process.argv.slice(2);
const modeIdx = args.indexOf('--mode');
const mode = modeIdx >= 0 ? args[modeIdx + 1] : null;
if (mode !== 'npm' && mode !== 'pkg-pr-new') {
  console.error(`Usage: publish-native-addons.ts --mode <npm|pkg-pr-new>`);
  process.exit(1);
}
const skipNpmPublish = mode === 'pkg-pr-new';

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

const npmDir = join(currentDir, 'npm');
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
    engines: { ...(pkgJson.engines as Record<string, unknown>), node: '>=20.0.0' },
  }));
}

// Fresh read: napi-rs prePublish rewrote this package.json on disk, so the
// top-level `pkg` import is stale for injected fields.
const cliPackageJson = readJsonFile(join(currentDir, 'package.json')) as {
  version: string;
  repository?: unknown;
  optionalDependencies?: Record<string, string>;
};
// Lockstep versioning: every generated platform package uses the CLI version.
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
  const pin = cliPackageJson.optionalDependencies?.[packageName];
  if (!pin) {
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
    ...(corePkgJson.optionalDependencies as Record<string, string> | undefined),
    ...nativePlatformPins,
  },
}));
const publishedPlatformPackages = Object.keys(nativePlatformPins).map((name) => ({
  name,
  version: cliVersion,
}));

// Publish each NAPI platform package (without vp binary)
const npmTag = process.env.NPM_TAG || 'latest';
if (!skipNpmPublish) {
  for (const file of platformDirs) {
    const platformDir = join(currentDir, 'npm', file);
    const platformPackageJson = readJsonFile(join(platformDir, 'package.json')) as {
      name: string;
      version: string;
    };
    await publishNpmPackageFromEnv(
      { name: platformPackageJson.name, version: platformPackageJson.version },
      'npm',
      ['publish', '--tag', npmTag, '--access', 'public'],
      platformDir,
    );
  }
}

// Create and publish separate @voidzero-dev/vite-plus-cli-{platform} packages
const cliNpmDir = join(currentDir, 'cli-npm');
for (const napiTarget of pkg.napi.targets) {
  const { platform, arch, abi, platformArchABI } = parseTriple(napiTarget);
  const isWindows = platform === 'win32';
  const binaryName = isWindows ? 'vp.exe' : 'vp';
  const rustBinarySource = join(repoRoot, 'target', napiTarget, 'release', binaryName);

  if (!existsSync(rustBinarySource)) {
    // eslint-disable-next-line no-console
    console.warn(
      `Warning: Rust binary not found at ${rustBinarySource}, skipping CLI package for ${platformArchABI}`,
    );
    continue;
  }

  // Create temp directory for CLI package
  const platformCliDir = join(cliNpmDir, platformArchABI);
  mkdirSync(platformCliDir, { recursive: true });

  // Copy binary
  copyFileSync(rustBinarySource, join(platformCliDir, binaryName));
  if (!isWindows) {
    chmodSync(join(platformCliDir, binaryName), 0o755);
  }

  // Copy trampoline shim binary for Windows (required)
  // The trampoline is a small exe that replaces .cmd wrappers to avoid
  // "Terminate batch job (Y/N)?" on Ctrl+C (see issue #835)
  const shimName = 'vp-shim.exe';
  const files = [binaryName];
  if (isWindows) {
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
    ...(abi ? { libc: [abi] } : {}),
    files,
    description: `Vite+ CLI binary for ${platformArchABI}`,
    repository: cliPackageJson.repository,
  };
  writeFileSync(join(platformCliDir, 'package.json'), JSON.stringify(cliPackage, null, 2) + '\n');
  publishedPlatformPackages.push({
    name: cliPackage.name,
    version: cliVersion,
  });

  if (skipNpmPublish) {
    // eslint-disable-next-line no-console
    console.log(
      `Prepared CLI package: @voidzero-dev/vite-plus-cli-${platformArchABI}@${cliVersion}`,
    );
    continue;
  }

  // Publish CLI package
  const result = await publishNpmPackageFromEnv(
    { name: cliPackage.name, version: cliVersion },
    'npm',
    ['publish', '--tag', npmTag, '--access', 'public'],
    platformCliDir,
  );

  if (result === 'published') {
    // eslint-disable-next-line no-console
    console.log(`Published CLI package: @voidzero-dev/vite-plus-cli-${platform}@${cliVersion}`);
  }
}

// `npm publish` returns when npm accepts an upload, before publish-time scanning
// necessarily makes that version installable. Core and the main CLI pin the
// native packages at this exact version, while the installers fetch the CLI
// platform packages directly. Do not continue the release until every
// platform packument and tarball can be fetched.
if (!skipNpmPublish) {
  await waitForNpmPackagesFromEnv(publishedPlatformPackages);
}

// Clean up cli-npm directory (skipped when caller still needs the prepared dirs).
if (!skipNpmPublish) {
  rmSync(cliNpmDir, { recursive: true, force: true });
}

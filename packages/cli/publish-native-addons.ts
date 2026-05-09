import { execSync } from 'node:child_process';
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
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { NapiCli, parseTriple } from '@napi-rs/cli';

import pkg from './package.json' with { type: 'json' };

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
    if (!dir.startsWith('vite-global-cli-')) {
      continue;
    }
    const target = dir.slice('vite-global-cli-'.length);
    const releaseDir = join(repoRoot, 'target', target, 'release');
    mkdirSync(releaseDir, { recursive: true });
    for (const file of await readdir(join(rustCliArtifactsDir, dir))) {
      copyFileSync(join(rustCliArtifactsDir, dir, file), join(releaseDir, file));
    }
  }
}

// Stamp VERSION into the publishable package.json files. napi prePublish and
// the cli-binary packages below both read packages/cli/package.json#version.
for (const p of ['core', 'test', 'cli']) {
  const pkgPath = join(repoRoot, 'packages', p, 'package.json');
  const content = readFileSync(pkgPath, 'utf-8');
  writeFileSync(pkgPath, content.replace('"version": "0.0.0"', `"version": "${VERSION}"`));
}

// Build test package against the just-stamped versions.
execSync('pnpm --filter=@voidzero-dev/vite-plus-test build', {
  cwd: repoRoot,
  stdio: 'inherit',
});

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

// Publish each NAPI platform package (without vp binary)
const npmTag = process.env.NPM_TAG || 'latest';
if (!skipNpmPublish) {
  for (const file of platformDirs) {
    try {
      const output = execSync(`npm publish --tag ${npmTag} --access public`, {
        cwd: join(currentDir, 'npm', file),
        env: process.env,
        stdio: 'pipe',
      });
      process.stdout.write(output);
    } catch (e) {
      if (
        e instanceof Error &&
        e.message.includes('You cannot publish over the previously published versions')
      ) {
        // eslint-disable-next-line no-console
        console.info(e.message);
        // eslint-disable-next-line no-console
        console.warn(`${file} has been published, skipping`);
      } else {
        throw e;
      }
    }
  }
}

// Read version from packages/cli/package.json for lockstep versioning
const cliPackageJson = JSON.parse(readFileSync(join(currentDir, 'package.json'), 'utf-8'));
const cliVersion = cliPackageJson.version;

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
        `Error: ${shimName} not found at ${shimSource}. Run "cargo build -p vite_trampoline --release --target ${napiTarget}" first.`,
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

  if (skipNpmPublish) {
    // eslint-disable-next-line no-console
    console.log(
      `Prepared CLI package: @voidzero-dev/vite-plus-cli-${platformArchABI}@${cliVersion}`,
    );
    continue;
  }

  // Publish CLI package
  execSync(`npm publish --tag ${npmTag} --access public`, {
    cwd: platformCliDir,
    env: process.env,
    stdio: 'inherit',
  });

  // eslint-disable-next-line no-console
  console.log(`Published CLI package: @voidzero-dev/vite-plus-cli-${platform}@${cliVersion}`);
}

// Clean up cli-npm directory (skipped when caller still needs the prepared dirs).
if (!skipNpmPublish) {
  rmSync(cliNpmDir, { recursive: true, force: true });
}

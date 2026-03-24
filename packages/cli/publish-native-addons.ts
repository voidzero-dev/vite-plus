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

import { NapiCli } from '@napi-rs/cli';

const cli = new NapiCli();

const currentDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(currentDir, '..', '..');

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

// Mapping from npm platform directory names to Rust target triples
const RUST_TARGETS: Record<string, string> = {
  'darwin-arm64': 'aarch64-apple-darwin',
  'darwin-x64': 'x86_64-apple-darwin',
  'linux-arm64-gnu': 'aarch64-unknown-linux-gnu',
  'linux-arm64-musl': 'aarch64-unknown-linux-musl',
  'linux-x64-gnu': 'x86_64-unknown-linux-gnu',
  'linux-x64-musl': 'x86_64-unknown-linux-musl',
  'win32-arm64-msvc': 'aarch64-pc-windows-msvc',
  'win32-x64-msvc': 'x86_64-pc-windows-msvc',
};
const npmDir = join(currentDir, 'npm');
const platformDirs = await readdir(npmDir);

// Publish each NAPI platform package (without vp binary)
const npmTag = process.env.NPM_TAG || 'latest';
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
      console.info(e.message);
      console.warn(`${file} has been published, skipping`);
    } else {
      throw e;
    }
  }
}

// Platform metadata for CLI packages
const PLATFORM_META: Record<string, { os: string; cpu: string; libc?: string }> = {
  'darwin-arm64': { os: 'darwin', cpu: 'arm64' },
  'darwin-x64': { os: 'darwin', cpu: 'x64' },
  'linux-arm64-gnu': { os: 'linux', cpu: 'arm64', libc: 'glibc' },
  'linux-arm64-musl': { os: 'linux', cpu: 'arm64', libc: 'musl' },
  'linux-x64-gnu': { os: 'linux', cpu: 'x64', libc: 'glibc' },
  'linux-x64-musl': { os: 'linux', cpu: 'x64', libc: 'musl' },
  'win32-arm64-msvc': { os: 'win32', cpu: 'arm64' },
  'win32-x64-msvc': { os: 'win32', cpu: 'x64' },
};

// Read version from packages/cli/package.json for lockstep versioning
const cliPackageJson = JSON.parse(readFileSync(join(currentDir, 'package.json'), 'utf-8'));
const cliVersion = cliPackageJson.version;

// Create and publish separate @voidzero-dev/vite-plus-cli-{platform} packages
const cliNpmDir = join(currentDir, 'cli-npm');
for (const [platform, rustTarget] of Object.entries(RUST_TARGETS)) {
  const meta = PLATFORM_META[platform];
  if (!meta) {
    // eslint-disable-next-line no-console
    console.log(`Skipping CLI package for ${platform}: no platform metadata`);
    continue;
  }

  const isWindows = platform.startsWith('win32');
  const binaryName = isWindows ? 'vp.exe' : 'vp';
  const rustBinarySource = join(repoRoot, 'target', rustTarget, 'release', binaryName);

  if (!existsSync(rustBinarySource)) {
    // eslint-disable-next-line no-console
    console.warn(
      `Warning: Rust binary not found at ${rustBinarySource}, skipping CLI package for ${platform}`,
    );
    continue;
  }

  // Create temp directory for CLI package
  const platformCliDir = join(cliNpmDir, platform);
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
    const shimSource = join(repoRoot, 'target', rustTarget, 'release', shimName);
    if (!existsSync(shimSource)) {
      console.error(
        `Error: ${shimName} not found at ${shimSource}. Run "cargo build -p vite_trampoline --release --target ${rustTarget}" first.`,
      );
      process.exit(1);
    }
    copyFileSync(shimSource, join(platformCliDir, shimName));
    files.push(shimName);
  }

  // Generate package.json
  const cliPackage = {
    name: `@voidzero-dev/vite-plus-cli-${platform}`,
    version: cliVersion,
    os: [meta.os],
    cpu: [meta.cpu],
    ...(meta.libc ? { libc: [meta.libc] } : {}),
    files,
    description: `Vite+ CLI binary for ${platform}`,
    repository: cliPackageJson.repository,
  };
  writeFileSync(join(platformCliDir, 'package.json'), JSON.stringify(cliPackage, null, 2) + '\n');

  // Publish CLI package
  execSync(`npm publish --tag ${npmTag} --access public`, {
    cwd: platformCliDir,
    env: process.env,
    stdio: 'inherit',
  });

  // eslint-disable-next-line no-console
  console.log(`Published CLI package: @voidzero-dev/vite-plus-cli-${platform}@${cliVersion}`);
}

// Clean up cli-npm directory
rmSync(cliNpmDir, { recursive: true, force: true });

import { execSync } from 'node:child_process';
import { copyFileSync, existsSync, chmodSync } from 'node:fs';
import { readdir } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { NapiCli } from '@napi-rs/cli';

const cli = new NapiCli();

const currentDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(currentDir, '..', '..');

// Mapping from npm platform directory names to Rust target triples
const RUST_TARGETS: Record<string, string> = {
  'darwin-arm64': 'aarch64-apple-darwin',
  'darwin-x64': 'x86_64-apple-darwin',
  'linux-arm64-gnu': 'aarch64-unknown-linux-gnu',
  'linux-x64-gnu': 'x86_64-unknown-linux-gnu',
  'win32-arm64-msvc': 'aarch64-pc-windows-msvc',
  'win32-x64-msvc': 'x86_64-pc-windows-msvc',
};

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

// Copy Rust binaries to each platform package
const npmDir = join(currentDir, 'npm');
const platformDirs = await readdir(npmDir);

for (const platformDir of platformDirs) {
  const rustTarget = RUST_TARGETS[platformDir];
  if (!rustTarget) {
    // eslint-disable-next-line no-console
    console.log(`Skipping ${platformDir}: no Rust target mapping`);
    continue;
  }

  const isWindows = platformDir.startsWith('win32');
  const binaryName = isWindows ? 'vp.exe' : 'vp';
  const rustBinarySource = join(repoRoot, 'target', rustTarget, 'release', binaryName);
  const rustBinaryDest = join(npmDir, platformDir, binaryName);

  if (!existsSync(rustBinarySource)) {
    throw new Error(`Rust binary not found at ${rustBinarySource}`);
  }

  copyFileSync(rustBinarySource, rustBinaryDest);
  // Make the binary executable on Unix
  if (!isWindows) {
    chmodSync(rustBinaryDest, 0o755);
  }
  // eslint-disable-next-line no-console
  console.log(`Copied Rust binary: ${rustBinarySource} -> ${rustBinaryDest}`);
}

// Pre-publish (updates package.json files in npm directories)
await cli.prePublish({
  cwd: currentDir,
  packageJsonPath: './package.json',
  tagStyle: 'npm',
  ghRelease: false,
  skipOptionalPublish: true,
});

// Publish each platform package
const npmTag = process.env.NPM_TAG || 'latest';
for (const file of platformDirs) {
  execSync(`npm publish --tag ${npmTag} --access public --no-git-checks`, {
    cwd: join(currentDir, 'npm', file),
    env: process.env,
    stdio: 'inherit',
  });
}

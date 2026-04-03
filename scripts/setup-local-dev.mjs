#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync, lstatSync, readFileSync, renameSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const isWindows = process.platform === 'win32';
const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const gitBin = isWindows ? 'git.exe' : 'git';
const pnpmBin = isWindows ? 'pnpm.cmd' : 'pnpm';
const pnpmLockfilePath = path.join(repoRoot, 'pnpm-lock.yaml');
const rolldownRepoDir = path.join(repoRoot, 'rolldown');
const rolldownPackageJsonRelativePath = 'packages/rolldown/package.json';
const upstreamVersions = JSON.parse(
  readFileSync(path.join(repoRoot, 'packages', 'tools', '.upstream-versions.json'), 'utf-8'),
);

function log(message) {
  process.stdout.write(`[setup-dev] ${message}\n`);
}

function fail(message) {
  console.error(`[setup-dev] ${message}`);
  process.exit(1);
}

function canonicalRemote(url) {
  return url
    .trim()
    .replace(/^git@github\.com:/, 'https://github.com/')
    .replace(/^ssh:\/\/git@github\.com\//, 'https://github.com/')
    .replace(/\.git$/, '')
    .replace(/\/$/, '');
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? repoRoot,
    stdio: options.stdio ?? 'inherit',
    encoding: options.encoding ?? 'utf-8',
  });

  if (result.error) {
    fail(result.error.message);
  }

  if (result.status !== 0) {
    const rendered = [command, ...args].join(' ');
    fail(`Command failed (${result.status}): ${rendered}`);
  }

  return result;
}

function capture(command, args, cwd) {
  return run(command, args, {
    cwd,
    stdio: 'pipe',
    encoding: 'utf-8',
  }).stdout.trim();
}

function stringifyJson(value) {
  return JSON.stringify(value, null, 2) + '\n';
}

function rolldownPackageJsonPath(rootDir = rolldownRepoDir) {
  return path.join(rootDir, rolldownPackageJsonRelativePath);
}

function isGitRepo(dir) {
  const result = spawnSync(gitBin, ['rev-parse', '--git-dir'], {
    cwd: dir,
    stdio: 'ignore',
  });
  return result.status === 0;
}

function isDirty(dir) {
  return capture(gitBin, ['status', '--porcelain'], dir) !== '';
}

function ensureExpectedRemote(name, dir, repoUrl) {
  const actual = canonicalRemote(capture(gitBin, ['remote', 'get-url', 'origin'], dir));
  const expected = canonicalRemote(repoUrl);
  if (actual !== expected) {
    fail(
      `Unexpected remote for ${name}: ${actual}. Expected ${expected}. Please fix the checkout or remove ${dir} and rerun this command.`,
    );
  }
}

function cloneCheckout(name, repoUrl, branch, hash) {
  log(`Cloning ${name} from ${repoUrl} (${branch})...`);
  run(gitBin, ['clone', '--branch', branch, repoUrl, name]);
  if (hash) {
    run(gitBin, ['reset', '--hard', hash], {
      cwd: path.join(repoRoot, name),
    });
  }
}

function rolldownBindingCandidates() {
  switch (process.platform) {
    case 'android':
      if (process.arch === 'arm64') {
        return ['@rolldown/binding-android-arm64'];
      }
      if (process.arch === 'arm') {
        return ['@rolldown/binding-android-arm-eabi'];
      }
      return [];
    case 'darwin':
      if (process.arch === 'arm64') {
        return ['@rolldown/binding-darwin-universal', '@rolldown/binding-darwin-arm64'];
      }
      if (process.arch === 'x64') {
        return ['@rolldown/binding-darwin-universal', '@rolldown/binding-darwin-x64'];
      }
      return [];
    case 'freebsd':
      if (process.arch === 'arm64') {
        return ['@rolldown/binding-freebsd-arm64'];
      }
      if (process.arch === 'x64') {
        return ['@rolldown/binding-freebsd-x64'];
      }
      return [];
    case 'linux':
      if (process.arch === 'arm') {
        return ['@rolldown/binding-linux-arm-gnueabihf', '@rolldown/binding-linux-arm-musleabihf'];
      }
      if (process.arch === 'arm64') {
        return ['@rolldown/binding-linux-arm64-gnu', '@rolldown/binding-linux-arm64-musl'];
      }
      if (process.arch === 'loong64') {
        return ['@rolldown/binding-linux-loong64-gnu', '@rolldown/binding-linux-loong64-musl'];
      }
      if (process.arch === 'ppc64') {
        return ['@rolldown/binding-linux-ppc64-gnu'];
      }
      if (process.arch === 'riscv64') {
        return ['@rolldown/binding-linux-riscv64-gnu', '@rolldown/binding-linux-riscv64-musl'];
      }
      if (process.arch === 's390x') {
        return ['@rolldown/binding-linux-s390x-gnu'];
      }
      if (process.arch === 'x64') {
        return ['@rolldown/binding-linux-x64-gnu', '@rolldown/binding-linux-x64-musl'];
      }
      return [];
    case 'win32':
      if (process.arch === 'arm64') {
        return ['@rolldown/binding-win32-arm64-msvc'];
      }
      if (process.arch === 'ia32') {
        return ['@rolldown/binding-win32-ia32-msvc'];
      }
      if (process.arch === 'x64') {
        return ['@rolldown/binding-win32-x64-msvc', '@rolldown/binding-win32-x64-gnu'];
      }
      return [];
    default:
      return [];
  }
}

function withRolldownHostBindings(pkg) {
  const candidates = rolldownBindingCandidates();
  if (candidates.length === 0) {
    return { changed: false, pkg };
  }

  const optionalDependencies = {
    ...pkg.optionalDependencies,
  };

  let changed = false;
  for (const candidate of candidates) {
    if (!optionalDependencies[candidate]) {
      optionalDependencies[candidate] = pkg.version;
      changed = true;
    }
  }

  if (!changed) {
    return { changed: false, pkg };
  }

  return {
    changed: true,
    pkg: {
      ...pkg,
      optionalDependencies,
    },
  };
}

function ensureRolldownHostBindings() {
  const packageJsonPath = rolldownPackageJsonPath();
  const { changed, pkg } = withRolldownHostBindings(
    JSON.parse(readFileSync(packageJsonPath, 'utf-8')),
  );
  if (!changed) {
    return;
  }

  writeFileSync(packageJsonPath, stringifyJson(pkg), 'utf-8');
  log(`Added host rolldown bindings to ${packageJsonPath}`);
}

function hasOnlyManagedRolldownBindingsChange(dir) {
  const statusEntries = capture(gitBin, ['status', '--porcelain'], dir).split('\n').filter(Boolean);
  if (statusEntries.length !== 1 || statusEntries[0].slice(3) !== rolldownPackageJsonRelativePath) {
    return false;
  }

  const { changed, pkg } = withRolldownHostBindings(
    JSON.parse(capture(gitBin, ['show', `HEAD:${rolldownPackageJsonRelativePath}`], dir)),
  );
  if (!changed) {
    return false;
  }

  return readFileSync(rolldownPackageJsonPath(dir), 'utf-8') === stringifyJson(pkg);
}

function syncCleanCheckout(name, config) {
  const dir = path.join(repoRoot, name);

  if (!existsSync(dir)) {
    cloneCheckout(name, config.repo, config.branch, config.hash);
    return;
  }

  if (lstatSync(dir).isSymbolicLink()) {
    log(`Using existing symlinked ${name} checkout at ${dir}`);
    return;
  }

  if (!isGitRepo(dir)) {
    fail(`${dir} exists but is not a git repository.`);
  }

  ensureExpectedRemote(name, dir, config.repo);

  const hasManagedRolldownBindingsChange =
    name === 'rolldown' && isDirty(dir) && hasOnlyManagedRolldownBindingsChange(dir);
  if (isDirty(dir) && !hasManagedRolldownBindingsChange) {
    log(`Keeping existing dirty ${name} checkout at ${dir}`);
    return;
  }
  if (hasManagedRolldownBindingsChange) {
    log(`Ignoring managed rolldown host binding diff at ${dir}`);
  }

  log(`Updating clean ${name} checkout...`);
  run(gitBin, ['fetch', 'origin', '--tags'], { cwd: dir });
  run(gitBin, ['checkout', config.branch], { cwd: dir });

  if (config.hash) {
    run(gitBin, ['reset', '--hard', config.hash], { cwd: dir });
  } else {
    run(gitBin, ['reset', '--hard', `origin/${config.branch}`], { cwd: dir });
  }
}

function migrateLegacyViteCheckout() {
  const viteDir = path.join(repoRoot, 'vite');
  const legacyDir = path.join(repoRoot, 'rolldown-vite');

  if (existsSync(viteDir) || !existsSync(legacyDir)) {
    return;
  }

  if (lstatSync(legacyDir).isSymbolicLink()) {
    fail(`Found legacy symlinked checkout at ${legacyDir}. Remove it and rerun this command.`);
  }

  if (!isGitRepo(legacyDir)) {
    fail(`Found legacy directory ${legacyDir}, but it is not a git repository.`);
  }

  ensureExpectedRemote('rolldown-vite', legacyDir, upstreamVersions.vite.repo);

  if (isDirty(legacyDir)) {
    fail(
      `Found legacy checkout at ${legacyDir} with local changes. Rename it to ./vite or clean it before rerunning this command.`,
    );
  }

  log(`Migrating legacy ${legacyDir} checkout to ${viteDir}...`);
  renameSync(legacyDir, viteDir);
}

function main() {
  migrateLegacyViteCheckout();

  syncCleanCheckout('rolldown', upstreamVersions.rolldown);
  syncCleanCheckout('vite', upstreamVersions.vite);
  ensureRolldownHostBindings();

  const originalLockfile = existsSync(pnpmLockfilePath)
    ? readFileSync(pnpmLockfilePath, 'utf-8')
    : null;
  log('Installing workspace dependencies...');
  try {
    run(pnpmBin, ['install']);
  } finally {
    if (originalLockfile !== null) {
      writeFileSync(pnpmLockfilePath, originalLockfile, 'utf-8');
    }
  }
}

main();

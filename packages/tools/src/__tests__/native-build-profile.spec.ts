import { execFileSync } from 'node:child_process';
import { chmodSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterEach, expect, test } from 'vitest';
import { parse } from 'yaml';

const repoRoot = fileURLToPath(new URL('../../../..', import.meta.url));
const temporaryDirs: string[] = [];
// Native producers run on Linux and macOS, including Windows cross-builds.
const bashTest = test.skipIf(process.platform === 'win32');

function temporaryDir() {
  const dir = mkdtempSync(path.join(tmpdir(), 'vp-native-profile-'));
  temporaryDirs.push(dir);
  return dir;
}

function actionStep(action: string, name: string): { run: string } {
  const file = path.join(repoRoot, '.github/actions', action, 'action.yml');
  return parse(readFileSync(file, 'utf8')).runs.steps.find(
    (step: { name?: string }) => step.name === name,
  );
}

afterEach(() => {
  for (const dir of temporaryDirs.splice(0)) {
    rmSync(dir, { recursive: true, force: true });
  }
});

bashTest('native caches separate release and CI binaries with identical source inputs', () => {
  const dir = temporaryDir();
  const { run } = actionStep('compute-native-cache-input-hash', 'Compute native cache input hash');
  const keys = ['release', 'ci'].map((profile) => {
    const output = path.join(dir, profile);
    execFileSync('bash', ['-e', '-c', run], {
      env: {
        ...process.env,
        GITHUB_OUTPUT: output,
        NATIVE_CACHE_INPUT_HASH: 'same-source',
        INPUTS_PROFILE: profile,
      },
    });
    return readFileSync(output, 'utf8').trim();
  });
  expect(keys).toEqual(['hash=same-source-release', 'hash=same-source-ci']);
});

bashTest.each([
  ['build-upstream', 'x86_64-unknown-linux-gnu'],
  ['build-upstream', 'x86_64-pc-windows-msvc'],
  ['build-windows-cli', 'x86_64-pc-windows-msvc'],
])('%s stages %s binaries for existing artifact consumers', (action, target) => {
  const dir = temporaryDir();
  const targetDir = path.join(dir, 'custom target');
  const profileDir = path.join(targetDir, target, 'ci');
  const releaseDir = path.join(targetDir, target, 'release');
  mkdirSync(profileDir, { recursive: true });
  mkdirSync(releaseDir, { recursive: true });
  const binaries = target.includes('windows') ? ['vp.exe', 'vp-setup.exe'] : ['vp'];
  for (const binary of binaries) {
    writeFileSync(path.join(profileDir, binary), `ci ${binary}`);
    chmodSync(path.join(profileDir, binary), 0o755);
    writeFileSync(path.join(releaseDir, binary), `stale ${binary}`);
  }
  writeFileSync(path.join(releaseDir, 'vp-shim.exe'), 'release trampoline');

  execFileSync('bash', ['-e', '-c', actionStep(action, 'Stage native CLI binaries').run], {
    cwd: dir,
    env: {
      ...process.env,
      CARGO_TARGET_DIR: targetDir,
      INPUTS_PROFILE: 'ci',
      INPUTS_TARGET: target,
    },
  });

  for (const binary of binaries) {
    expect(readFileSync(path.join(releaseDir, binary), 'utf8')).toBe(`ci ${binary}`);
  }
  expect(readFileSync(path.join(releaseDir, 'vp-shim.exe'), 'utf8')).toBe('release trampoline');
});

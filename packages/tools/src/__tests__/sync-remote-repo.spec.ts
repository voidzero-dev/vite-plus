import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

import { describe, expect, test } from 'vitest';

import { cloneOrResetRepo } from '../sync-remote-deps.ts';

function git(cwd: string, ...args: string[]): string {
  return execFileSync(
    'git',
    [
      '-c',
      'user.name=Test',
      '-c',
      'user.email=test@example.com',
      '-c',
      'commit.gpgsign=false',
      ...args,
    ],
    { cwd, encoding: 'utf8', stdio: 'pipe' },
  ).trim();
}

describe('cloneOrResetRepo', () => {
  test.each(['fresh', 'existing'])(
    'fetches a pull request pin missing from a %s checkout',
    (checkoutState) => {
      const root = mkdtempSync(join(tmpdir(), 'vp-upstream-pin-'));
      try {
        const source = join(root, 'source');
        const remote = join(root, 'remote.git');
        const checkout = join(root, 'checkout');
        git(root, 'init', '--initial-branch=main', source);
        writeFileSync(join(source, 'version.txt'), 'main');
        git(source, 'add', 'version.txt');
        git(source, 'commit', '-m', 'main');
        git(root, 'clone', '--bare', source, remote);

        // Only the pull request ref contains this commit. A normal branch/tag
        // fetch must not supply it, as with an unmerged PR on GitHub.
        writeFileSync(join(source, 'version.txt'), 'pull request');
        git(source, 'commit', '-am', 'pull request');
        const pin = git(source, 'rev-parse', 'HEAD');
        git(source, 'push', remote, 'HEAD:refs/pull/123/head');
        const remoteUrl = pathToFileURL(remote).href;
        // file:// uses Git's transport instead of copying every local object.
        git(root, 'clone', '--branch', 'main', remoteUrl, checkout);
        expect(
          spawnSync('git', ['cat-file', '-e', `${pin}^{commit}`], { cwd: checkout }).status,
        ).not.toBe(0);
        if (checkoutState === 'fresh') {
          rmSync(checkout, { recursive: true });
        }

        cloneOrResetRepo(remoteUrl, checkout, 'main', pin);
        expect(git(checkout, 'rev-parse', 'HEAD')).toBe(pin);
        expect(git(checkout, 'branch', '--show-current')).toBe('main');
        expect(readFileSync(join(checkout, 'version.txt'), 'utf8')).toBe('pull request');
        cloneOrResetRepo(remoteUrl, checkout, 'main', pin);
        expect(git(checkout, 'rev-parse', 'HEAD')).toBe(pin);
      } finally {
        rmSync(root, { recursive: true, force: true });
      }
    },
  );
});

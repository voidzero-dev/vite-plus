import { tmpdir } from 'node:os';

import { describe, expect, test } from '@voidzero-dev/vite-plus/test';

import { isPassThroughEnv, replaceUnstableOutput } from '../utils.ts';

describe('replaceUnstableOutput()', () => {
  test('replace unstable semver version', () => {
    const output = `
foo v1.0.0
 v1.0.0-beta.1
 v1.0.0-beta.1+build.1
 1.0.0
 1.0.0-beta.1
 1.0.0-beta.1+build.1
tsdown/0.15.1
vitest/3.2.4
foo/v100.1.1000
foo@1.0.0
bar@v1.0.0
    `;
    expect(replaceUnstableOutput(output.trim())).toMatchSnapshot();
  });

  test('replace date', () => {
    const output = `
Start at  15:01:23
15:01:23
    `;
    expect(replaceUnstableOutput(output.trim())).toMatchSnapshot();
  });

  test('replace unstable pnpm install output', () => {
    const outputs = [
      `
Scope: all 6 workspace projects
Packages: +312
++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
Progress: resolved 1, reused 0, downloaded 0, added 0
Progress: resolved 316, reused 316, downloaded 0, added 315
WARN  Skip adding vite to the default catalog because it already exists as npm:@voidzero-dev/vite-plus. Please use \`pnpm update\` to update the catalogs.
WARN  Skip adding vitest to the default catalog because it already exists as beta. Please use \`pnpm update\` to update the catalogs.
Progress: resolved 316, reused 316, downloaded 0, added 316, done

devDependencies:
+ @voidzero-dev/vite-plus 0.0.0-8a4f4936e0eca32dd57e1a503c2b09745953344d
+ vitest 3.2.4
      `,
      `
Scope: all 2 workspace projects
Lockfile is up to date, resolution step is skipped
Already up to date

╭ Warning ───────────────────────────────────────────────────────────────────────────────────╮
│                                                                                            │
│   Ignored build scripts: esbuild.                                                          │
│   Run "pnpm approve-builds" to pick which dependencies should be allowed to run scripts.   │
│                                                                                            │
╰────────────────────────────────────────────────────────────────────────────────────────────╯

Done in 171ms using pnpm v10.16.1
      `,
    ];
    for (const output of outputs) {
      expect(replaceUnstableOutput(output.trim())).toMatchSnapshot();
    }
  });

  test('replace unstable cwd', () => {
    const cwd = tmpdir();
    const output = `${cwd}/foo.txt`;
    expect(replaceUnstableOutput(output.trim(), cwd)).toMatchSnapshot();
  });

  test('replace tsdown output', () => {
    const output = `
ℹ tsdown v0.15.1 powered by rolldown v0.15.1
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.js  0.15 kB │ gzip: 0.12 kB
ℹ 1 files, total: 0.15 kB
✔ Build complete in 100ms
    `;
    expect(replaceUnstableOutput(output.trim())).toMatchSnapshot();
  });

  test('replace yarn YN0013', () => {
    const output = `
➤ YN0000: ┌ Fetch step
➤ YN0013: │ A package was added to the project (+ 0.7 KiB).
➤ YN0000: └ Completed
    `;
    expect(replaceUnstableOutput(output.trim())).toMatchSnapshot();
  });
});

describe('isPassThroughEnv()', () => {
  test('should return true if env is pass-through', () => {
    expect(isPassThroughEnv('NPM_AUTH_TOKEN')).toBe(true);
    expect(isPassThroughEnv('PATH')).toBe(true);
  });

  test('should return false if env is not pass-through', () => {
    expect(isPassThroughEnv('NODE_ENV')).toBe(false);
    expect(isPassThroughEnv('API_URL')).toBe(false);
  });
});

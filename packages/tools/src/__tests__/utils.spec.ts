import { randomUUID } from 'node:crypto';
import fs from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { describe, expect, test } from '@voidzero-dev/vite-plus-test';

import { isPassThroughEnv, replaceUnstableOutput } from '../utils';

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
    const output = `${path.join(cwd, 'foo.txt')}`;
    expect(replaceUnstableOutput(output.trim(), cwd)).toMatchSnapshot();
  });

  test('replace unstable tmpdir with realpath', () => {
    const tmp = fs.realpathSync(tmpdir());
    const cwd = path.join(tmp, `vite-plus-unittest-${randomUUID()}`);
    const output = `${path.join(cwd, 'foo.txt')}\n${path.join(cwd, '../other/bar.txt')}`;
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

  test('replace yarn YN0000: └ Completed with duration to empty string', () => {
    const output = `
➤ YN0000: └ Completed in 100ms
➤ YN0000: └ Completed in 100ms 200ms
➤ YN0000: └ Completed
    `;
    expect(replaceUnstableOutput(output.trim())).toMatchSnapshot();
  });

  test('replace ignore pnpm request warning log', () => {
    const output = `
Foo bar
 WARN  Request took <variable>ms: https://registry.npmjs.org/testnpm2
Packages:
    `;
    expect(replaceUnstableOutput(output.trim())).toMatchSnapshot();
  });

  test('replace ignore npm audited packages log', () => {
    const output = `
removed 1 package, and audited 3 packages in 700ms
up to date, audited 4 packages in 11ms
added 1 package, and audited 3 packages in 700ms
added 3 packages, and audited 4 packages in 100ms

found 0 vulnerabilities
Done in 1000ms
    `;
    expect(replaceUnstableOutput(output.trim())).toMatchSnapshot();
  });

  test('replace ignore npm registry domain', () => {
    const output = `
https://registry.npmjs.org/testnpm2
https://registry.yarnpkg.com/debug
https://registry.yarnpkg.com/testnpm2/-/testnpm2-1.0.0.tgz
"resolved": "https://registry.yarnpkg.com/testnpm2/-/testnpm2-1.0.0.tgz",
"resolved": "https://registry.npmjs.org/testnpm2/-/testnpm2-1.0.0.tgz",
    `;
    expect(replaceUnstableOutput(output.trim())).toMatchSnapshot();
  });

  test('replace pnpm registry request error warning log', () => {
    const output = `
 WARN  GET https://registry.npmjs.org/test-vite-plus-install error (ECONNRESET). Will retry in 10 seconds. 2 retries left.
Progress: resolved
`;
    expect(replaceUnstableOutput(output.trim())).toMatchSnapshot();
  });

  test('replace ignore tarball download average speed warning log', () => {
    const output = `
 WARN  Tarball download average speed 29 KiB/s (size 56 KiB) is below 50 KiB/s: https://registry.npmjs.org/qs/-/qs-6.14.0.tgz (GET)
 WARN  Tarball download average speed 34 KiB/s (size 347 KiB) is below 50 KiB/s: https://registry.npmjs.org/undici/-/undici-7.16.0.tgz (GET)
`;
    expect(replaceUnstableOutput(output.trim())).toMatchSnapshot();
  });

  test('replace hash values', () => {
    const output = `
npm notice shasum: 65c35f9599054722ecde040abd4a19682a723cdc
npm notice integrity: sha512-qugLL42iCblSD[...]Gfk6HJodp2ZOQ==
"shasum": "65c35f9599054722ecde040abd4a19682a723cdc",
"integrity": "sha512-qugLL42iCblSDO0Vwic9xYkKYNtf+MwPW4cQSppKbGtQ/xswl1gXyu/DF5b7I/WbsVi02DJIHGfk6HJodp2ZOQ==",
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

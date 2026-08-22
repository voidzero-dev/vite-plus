const { chmodSync, mkdirSync, writeFileSync } = require('node:fs');
const { join } = require('node:path');

const vpHome = process.env.VP_HOME;
if (!vpHome) {
  throw new Error('VP_HOME is required');
}

// The layout of a managed pnpm install: the JS CLI entry `pnpm.cjs` plus the
// platform shims, so package-manager resolution finds pnpm@11.0.0 without a
// download and `npm_execpath` resolves to the real JS CLI entry.
const binDir = join(vpHome, 'package_manager', 'pnpm', '11.0.0', 'pnpm', 'bin');
mkdirSync(binDir, { recursive: true });

writeFileSync(
  join(binDir, 'pnpm.cjs'),
  "console.log('pnpm ' + process.argv.slice(2).join(' '));\n",
);

const unixShim = join(binDir, 'pnpm');
writeFileSync(unixShim, "#!/usr/bin/env node\nrequire('./pnpm.cjs');\n");
chmodSync(unixShim, 0o755);

writeFileSync(join(binDir, 'pnpm.cmd'), '@echo off\r\nnode "%~dp0pnpm.cjs" %*\r\n');
writeFileSync(
  join(binDir, 'pnpm.ps1'),
  'node "$PSScriptRoot/pnpm.cjs" @args\nexit $LASTEXITCODE\n',
);

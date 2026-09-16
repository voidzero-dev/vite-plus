const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const bin = path.resolve('packages/app/node_modules/.bin');
fs.mkdirSync(bin, { recursive: true });
// The directory must be empty when case sensitivity is enabled. Do not depend
// on the checkout's NTFS flags or the casing chosen by a package manager.
execFileSync('fsutil.exe', ['file', 'setCaseSensitiveInfo', bin, 'enable']);
fs.writeFileSync(path.join(bin, 'astro.cmd'), '@echo off\r\nnode "%~dp0/../../print.cjs" %*\r\n');
assert.equal(fs.existsSync(path.join(bin, 'astro.CMD')), false);
assert.equal(process.env.PATHEXT, '.COM;.EXE;.BAT;.CMD');

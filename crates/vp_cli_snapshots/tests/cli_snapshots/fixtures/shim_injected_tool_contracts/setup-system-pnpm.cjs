const { mkdirSync, writeFileSync } = require('node:fs');

mkdirSync('system-pnpm');
if (process.platform === 'win32') {
  writeFileSync('system-pnpm/pnpm.cmd', '@node "%~dp0../system-pnpm-probe.cjs"\r\n');
} else {
  writeFileSync('system-pnpm/pnpm', '#!/usr/bin/env node\nrequire("../system-pnpm-probe.cjs");\n', {
    mode: 0o755,
  });
}

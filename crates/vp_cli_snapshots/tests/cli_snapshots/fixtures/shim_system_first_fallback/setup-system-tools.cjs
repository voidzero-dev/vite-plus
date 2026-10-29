// Fake system npm and pnpm beside system-bin/node. Each reports its name, its
// arguments, and whether the shim left PATH unchanged (EXPECTED_PATH).
const { chmodSync, writeFileSync } = require('node:fs');
const path = require('node:path');

for (const name of ['npm', 'pnpm']) {
  if (process.platform === 'win32') {
    writeFileSync(
      path.join('system-bin', `${name}.cmd`),
      [
        '@echo off',
        `echo system ${name} %*`,
        'if "%PATH%"=="%EXPECTED_PATH%" (echo same PATH) else (echo changed PATH)',
        '',
      ].join('\r\n'),
    );
  } else {
    const file = path.join('system-bin', name);
    writeFileSync(
      file,
      [
        '#!/bin/sh',
        `echo "system ${name} $*"`,
        'if [ "$PATH" = "$EXPECTED_PATH" ]; then echo "same PATH"; else echo "changed PATH"; fi',
        '',
      ].join('\n'),
    );
    chmodSync(file, 0o755);
  }
}

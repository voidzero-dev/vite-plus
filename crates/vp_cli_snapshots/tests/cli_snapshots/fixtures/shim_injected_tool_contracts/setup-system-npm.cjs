const { mkdirSync, symlinkSync, writeFileSync } = require('node:fs');

mkdirSync('system-npm', { recursive: true });
symlinkSync(process.execPath, 'system-npm/node');
writeFileSync(
  'system-npm/npm',
  '#!/bin/sh\nif [ "$1" = "--version" ]; then echo 10.5.0; else exec npx --version; fi\n',
  { mode: 0o755 },
);

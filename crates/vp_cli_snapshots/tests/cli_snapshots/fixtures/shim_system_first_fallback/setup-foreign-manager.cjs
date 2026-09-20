const { mkdirSync, writeFileSync } = require('node:fs');

mkdirSync('foreign-bin');
// Retain PATH when forwarding to Vite+, as an external version manager would.
if (process.platform === 'win32') {
  writeFileSync('foreign-bin/node.cmd', '@"%VP_HOME%\\fallback-bin\\node.exe" %*\r\n');
} else {
  writeFileSync('foreign-bin/node', '#!/bin/sh\nexec "$VP_HOME/fallback-bin/node" "$@"\n', { mode: 0o755 });
}

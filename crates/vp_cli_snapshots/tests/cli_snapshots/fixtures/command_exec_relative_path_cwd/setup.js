const fs = require('fs');

fs.mkdirSync('packages/app/tools', { recursive: true });
fs.writeFileSync(
  'packages/app/tools/fake-node',
  '#!/usr/bin/env node\nconsole.log("resolved from package cwd");\n',
  { mode: 0o755 },
);
fs.writeFileSync('packages/app/tools/fake-node.cmd', '@node "%~dp0\\fake-node" %*\n');

const fs = require('fs');
fs.mkdirSync('node_modules/.bin', { recursive: true });
fs.writeFileSync(
  'node_modules/.bin/hello-test',
  '#!/usr/bin/env node\nconsole.log("hello from test-bin");\n',
  { mode: 0o755 },
);
fs.writeFileSync('node_modules/.bin/hello-test.cmd', '@node "%~dp0\\hello-test" %*\n');

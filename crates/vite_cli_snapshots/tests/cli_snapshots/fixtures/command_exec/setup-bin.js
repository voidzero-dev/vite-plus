const fs = require('fs');
fs.mkdirSync('node_modules/.bin', { recursive: true });
fs.writeFileSync(
  'node_modules/.bin/hello-test',
  '#!/usr/bin/env node\nconsole.log("hello from test-bin");\n',
  { mode: 0o755 },
);
fs.writeFileSync('node_modules/.bin/hello-test.cmd', '@node "%~dp0\\hello-test" %*\n');

// Create subdir with a local executable for cwd resolution test
fs.mkdirSync('subdir', { recursive: true });
fs.writeFileSync('subdir/my-local', '#!/usr/bin/env node\nconsole.log("hello from subdir");\n', {
  mode: 0o755,
});

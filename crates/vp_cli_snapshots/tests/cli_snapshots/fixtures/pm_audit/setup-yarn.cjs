const fs = require('node:fs');
const path = require('node:path');

// Yarn 2.3 predates cli-dist, so seed launchers that fail if the guard delegates.
const bin = path.join(process.env.VP_HOME, 'package_manager', 'yarn', '2.3.1', 'yarn', 'bin');
fs.mkdirSync(bin, { recursive: true });
for (const [name, source] of Object.entries({
  yarn: '#!/bin/sh\nexit 99\n',
  'yarn.cmd': '@exit /b 99\r\n',
  'yarn.ps1': 'exit 99\n',
})) {
  fs.writeFileSync(path.join(bin, name), source, { mode: 0o755 });
}

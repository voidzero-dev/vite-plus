const fs = require('node:fs');
const path = require('node:path');

// Yarn 2.3 predates cli-dist. Allow version queries, but fail if audit is executed.
const bin = path.join(process.env.VP_HOME, 'package_manager', 'yarn', '2.3.1', 'yarn', 'bin');
fs.mkdirSync(bin, { recursive: true });
for (const [name, source] of Object.entries({
  yarn: '#!/bin/sh\nif [ "$1" = "--version" ]; then echo 2.3.1; exit 0; fi\nexit 99\n',
  'yarn.cmd': '@if "%~1"=="--version" (echo 2.3.1& exit /b 0)\r\n@exit /b 99\r\n',
  'yarn.ps1': 'if ($args[0] -eq "--version") { Write-Output "2.3.1"; exit 0 }\nexit 99\n',
})) {
  fs.writeFileSync(path.join(bin, name), source, { mode: 0o755 });
}

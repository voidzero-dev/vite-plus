const { copyFileSync, mkdirSync } = require('node:fs');
const path = require('node:path');

mkdirSync('system-bin');
copyFileSync(process.execPath, path.join('system-bin', process.platform === 'win32' ? 'node.exe' : 'node'));

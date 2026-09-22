const { copyFileSync, mkdirSync, symlinkSync } = require('node:fs');
const path = require('node:path');

mkdirSync('shared-bin');
copyFileSync(process.execPath, path.join('shared-bin', process.platform === 'win32' ? 'node.exe' : 'node'));
// Reuse the seeded runtime without downloading it again under the split data root.
symlinkSync(path.join(process.env.VP_HOME, 'js_runtime'), 'data/js_runtime', 'junction');

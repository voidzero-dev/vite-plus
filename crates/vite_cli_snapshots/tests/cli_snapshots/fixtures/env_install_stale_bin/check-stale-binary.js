const fs = require('fs');
const path = require('path');

const shimSuffix = process.platform === 'win32' ? '.exe' : '';
const shim = path.join(process.env.VP_HOME, 'bin', 'env-install-stale-drop' + shimSuffix);
const config = path.join(process.env.VP_HOME, 'bins/env-install-stale-drop.json');

console.log(fs.existsSync(shim) ? 'stale shim exists' : 'stale shim removed');
console.log(fs.existsSync(config) ? 'stale config exists' : 'stale config removed');

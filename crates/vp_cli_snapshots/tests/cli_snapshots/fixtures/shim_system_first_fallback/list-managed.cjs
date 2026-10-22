// List managed runtimes, package managers, and the Node.js version index cache.
const { existsSync, readdirSync } = require('node:fs');
const path = require('node:path');

const list = (directory) => (existsSync(directory) ? readdirSync(directory).sort() : []);
const runtimes = path.join(process.env.VP_HOME, 'js_runtime', 'node');
console.log(`js_runtime/node: ${list(runtimes).join(', ') || '(empty)'}`);
const managers = path.join(process.env.VP_HOME, 'package_manager');
for (const family of list(managers)) {
  console.log(`package_manager/${family}: ${list(path.join(managers, family)).join(', ')}`);
}
if (list(managers).length === 0) console.log('package_manager: (empty)');

// Report the managed installs that the system-only steps could use or create.
// The case's js_runtime links the runner's shared seed, whose versions differ
// between machines, so only the relevant versions are checked.
const { existsSync, readdirSync } = require('node:fs');
const path = require('node:path');

const list = (directory) => (existsSync(directory) ? readdirSync(directory).sort() : []);
const runtimes = path.join(process.env.VP_HOME, 'js_runtime', 'node');
// Snapshots redact a version after a tool name, so the lines name its source.
const versions = { '.node-version pin': '20.18.0', 'VP_NODE_VERSION override': '24.0.0' };
for (const [source, version] of Object.entries(versions)) {
  console.log(`managed Node.js (${source}): ${existsSync(path.join(runtimes, version))}`);
}
const managers = path.join(process.env.VP_HOME, 'package_manager');
for (const family of list(managers)) {
  console.log(`package_manager/${family}: ${list(path.join(managers, family)).join(', ')}`);
}
if (list(managers).length === 0) console.log('package_manager: (empty)');

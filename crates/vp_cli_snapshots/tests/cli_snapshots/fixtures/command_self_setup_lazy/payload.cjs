const fs = require('node:fs');
const path = require('node:path');

module.exports = function writePayload(pkg, nodeVersion) {
  fs.mkdirSync(path.join(pkg, 'dist'), { recursive: true });
  fs.writeFileSync(
    path.join(pkg, 'package.json'),
    JSON.stringify({
      name: 'vite-plus',
      version: '0.3.2',
      devEngines: { runtime: { name: 'node', version: nodeVersion } },
    }),
  );
  fs.writeFileSync(
    path.join(pkg, 'dist/bin.js'),
    'console.log(JSON.stringify({ args: process.argv.slice(2), node: process.versions.node }));',
  );
  fs.writeFileSync(path.join(pkg, 'dist/script-preload.cjs'), "globalThis.lazyLoader = 'global';");
  fs.writeFileSync(path.join(pkg, 'dist/script-register.js'), '');
  fs.writeFileSync(
    path.join(pkg, 'dist/script-watch.js'),
    "console.log(JSON.stringify({ args: process.argv.slice(2), loader: 'global-watch', node: process.versions.node }));",
  );
  fs.writeFileSync(
    path.join(pkg, 'dist/toolchain.json'),
    JSON.stringify({
      schemaVersion: 1,
      nodes: [
        {
          id: 'vite-plus',
          name: 'vite-plus',
          version: '0.3.2',
          kind: 'package',
          delivery: ['dependency'],
        },
      ],
      edges: [],
    }),
  );
};

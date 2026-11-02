const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const source = process.env.PATH.split(path.delimiter)
  .map((dir) => path.join(dir, 'yarn.js'))
  .find((file) => fs.existsSync(file));
assert.ok(source, 'env exec must expose the selected Yarn installation');
fs.mkdirSync('.yarn/releases', { recursive: true });
fs.copyFileSync(source, `.yarn/releases/yarn-${process.argv[2]}.cjs`);

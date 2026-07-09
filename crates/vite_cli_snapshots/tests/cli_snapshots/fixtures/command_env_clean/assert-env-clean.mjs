import fs from 'node:fs';
import path from 'node:path';

const expected = [
  path.join('home', 'js_runtime', 'node', '20.18.0'),
  path.join('home', 'js_runtime', 'node', '24.11.0'),
  path.join('home', 'corepack-cleaned'),
];
const unexpected = [
  path.join('home', 'js_runtime', 'node', '22.18.0'),
  path.join('home', 'package_manager'),
];

for (const filePath of expected) {
  if (!fs.existsSync(filePath)) {
    throw new Error(`Expected ${filePath} to exist`);
  }
}

for (const filePath of unexpected) {
  if (fs.existsSync(filePath)) {
    throw new Error(`Expected ${filePath} to be removed`);
  }
}

console.log('clean preserved current/default and removed caches');

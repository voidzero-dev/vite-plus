import { mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const directoryFlag = process.argv.indexOf('--directory');
const directory = directoryFlag === -1 ? 'app' : process.argv[directoryFlag + 1];
mkdirSync(directory, { recursive: true });

function write(filename, content) {
  writeFileSync(path.join(directory, filename), content);
}

write('package.json', JSON.stringify({ name: path.basename(directory), type: 'module' }));
write('vite.config.mjs', 'export default {};\n');
write('.oxlintrc.json', JSON.stringify({ extends: ['../../shared/lint.json'] }));
write('index.js', 'debugger;\n');

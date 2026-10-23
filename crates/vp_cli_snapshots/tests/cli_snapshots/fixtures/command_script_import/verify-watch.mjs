import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { writeFileSync } from 'node:fs';
import { writeFile, unlink } from 'node:fs/promises';

const entry = 'src/watch-entry.ts';
const dependency = 'src/watch-dependency.ts';
await writeFile(entry, "import { value } from './watch-dependency.ts'; console.log('watch-value:' + value);\n");
await writeFile(dependency, "export enum Value { Current = 'first' }\nexport const value = Value.Current;\n");
const child = spawn(process.execPath, [
  '--watch', '--watch-preserve-output', '--import=vite-plus/register', entry,
], { stdio: ['ignore', 'pipe', 'pipe'] });
let output = '';
let edited = false;
const exited = once(child, 'exit');
let timer;
let rewrite;
try {
  await new Promise((resolve, reject) => {
    timer = setTimeout(() => reject(new Error(`native watch timed out:\n${output}`)), 20000);
    child.once('error', reject);
    child.once('exit', (code, signal) => reject(new Error(`native watch exited ${code}/${signal}:\n${output}`)));
    child.stderr.on('data', chunk => { output += chunk; });
    child.stdout.on('data', chunk => {
      output += chunk;
      if (!edited && output.includes('watch-value:first')) {
        edited = true;
        // The child can print before the parent receives its dependency report.
        // Repeat the edit until the real watcher observes it, as the vpx fixture does.
        const edit = () => {
          try {
            writeFileSync(dependency, "export enum Value { Current = 'second' }\nexport const value = Value.Current;\n");
          } catch (error) { reject(error); }
        };
        edit();
        rewrite = setInterval(edit, 500);
      }
      if (output.includes('watch-value:second')) resolve();
    });
  });
  assert.equal(edited, true);
  console.log('native node --watch reloads an imported TypeScript dependency');
} finally {
  clearTimeout(timer);
  clearInterval(rewrite);
  child.kill();
  await exited;
  await Promise.all([unlink(entry), unlink(dependency)]);
}

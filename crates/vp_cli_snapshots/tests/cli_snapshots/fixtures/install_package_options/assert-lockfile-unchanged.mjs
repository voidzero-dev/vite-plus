import { readFileSync } from 'node:fs';

const lockfile = readFileSync(process.argv[2]);
const original = readFileSync('before.lock');
if (!lockfile.equals(original)) {
  process.exit(1);
}
console.log('lockfile unchanged');

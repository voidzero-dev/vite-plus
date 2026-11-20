import fs from 'node:fs';
import path from 'node:path';

const destination = path.resolve('linked-directory');
if (process.argv[2] !== 'dangling') {
  fs.mkdirSync(destination);
  fs.writeFileSync(path.join(destination, 'keep.txt'), 'keep\n');
}
fs.symlinkSync(destination, 'new-project', process.platform === 'win32' ? 'junction' : 'dir');

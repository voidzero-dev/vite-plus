import { writeFileSync } from 'node:fs';

writeFileSync(process.argv[2], 'task modification\n');
console.error('Task diagnostic');
process.exitCode = 1;

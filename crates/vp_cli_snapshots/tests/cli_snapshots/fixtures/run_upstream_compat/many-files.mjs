import { statSync } from 'node:fs';

for (let i = 0; i < 20_000; i++) {
  statSync(`missing-${i}`, { throwIfNoEntry: false });
}
console.log('Task completed after 20000 file accesses');

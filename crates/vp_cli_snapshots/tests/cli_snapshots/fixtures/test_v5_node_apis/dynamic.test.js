import { writeFileSync } from 'node:fs';
import { test, expect } from 'vite-plus/test';

writeFileSync('collection-ran.txt', 'collected');
const register = (name) => test(name, () => expect(true).toBe(true));
register('generated at collection time');

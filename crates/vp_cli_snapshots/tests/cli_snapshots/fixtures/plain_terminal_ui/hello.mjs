import fs from 'node:fs';

console.log(fs.readFileSync('input.txt', 'utf8').trim(), process.env.FOO);

import { readFileSync } from 'node:fs';

const { name } = JSON.parse(readFileSync('package.json', 'utf8'));
console.log(`Legacy CLI in ${name}: ${JSON.stringify(process.argv.slice(2))}`);

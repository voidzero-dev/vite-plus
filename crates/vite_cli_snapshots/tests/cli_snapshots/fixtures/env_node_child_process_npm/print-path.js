import { execFileSync } from 'node:child_process';

const npmPath = execFileSync('which', ['npm'], { encoding: 'utf8' }).trim();
const normalizedNpmPath = npmPath.split('/').join('/');

console.log(normalizedNpmPath);

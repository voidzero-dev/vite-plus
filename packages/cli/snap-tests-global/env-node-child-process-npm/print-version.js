import { execFileSync } from 'node:child_process';

const versionOutput = execFileSync('npm', ['--version'], { encoding: 'utf8' }).trim();
const version = versionOutput.split(/\r?\n/).at(-1);

console.log(version);

import { execFileSync } from 'node:child_process';

const expectedVersion = '11.17.0';

const npmPath = execFileSync('which', ['npm'], { encoding: 'utf8' }).trim();
const versionOutput = execFileSync('npm', ['--version'], { encoding: 'utf8' }).trim();
const version = versionOutput.split(/\r?\n/).at(-1);
const normalizedNpmPath = npmPath.split('/').join('/');

if (version !== expectedVersion) {
  console.error(`Expected npm ${expectedVersion}, got ${versionOutput}`);
  process.exit(1);
}

if (
  !normalizedNpmPath.includes(`/package_manager/npm/${expectedVersion}/npm/bin/npm`) ||
  normalizedNpmPath.includes('/js_runtime/node/')
) {
  console.error(`Expected managed npm path, got ${npmPath}`);
  process.exit(1);
}

console.log(`node child process uses managed npm \n${npmPath} \n${version}`);

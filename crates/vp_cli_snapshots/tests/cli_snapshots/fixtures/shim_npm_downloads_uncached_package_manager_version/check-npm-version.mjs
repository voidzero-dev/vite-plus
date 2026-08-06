import { execSync } from 'node:child_process';

const [expected, message] = process.argv.slice(2);
const version = execSync('npm --version', { encoding: 'utf8' }).trim();
if (version !== expected) {
  console.error(`npm --version printed ${version}, expected ${expected}`);
  process.exit(1);
}
console.log(message);

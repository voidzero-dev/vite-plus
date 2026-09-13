const { execSync } = require('node:child_process');

console.log(
  JSON.stringify({
    node: process.version,
    npm: execSync('npm --version', { encoding: 'utf8' }).trim(),
  }),
);

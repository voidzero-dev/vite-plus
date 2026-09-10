const assert = require('node:assert/strict');
const { execSync } = require('node:child_process');

assert.equal(process.version, 'v22.18.0');
for (const tool of ['npm', 'npx']) {
  assert.equal(execSync(`${tool} --version`, { encoding: 'utf8' }).trim(), '10.5.0');
}

if (process.argv[2] === 'child') {
  assert.equal(execSync('pnpm --version', { encoding: 'utf8' }).trim(), '10.19.0');
  console.log('Adding pnpm preserves inherited Node 22.18.0 and npm/npx 10.5.0');
} else {
  execSync('pnpm exec node ../assert-inherited-npm.cjs child', {
    cwd: 'pnpm-child',
    stdio: 'inherit',
    timeout: 30000,
  });
}

#!/usr/bin/env node
const args = process.argv.slice(2);
if (args[0] === 'inner') {
  console.log('inner call succeeded');
} else {
  console.log('outer call');
  const { execSync } = require('child_process');
  // This re-invokes the shim, testing recursion
  const output = execSync('recursive-cli inner', { encoding: 'utf8' });
  process.stdout.write(output);
}

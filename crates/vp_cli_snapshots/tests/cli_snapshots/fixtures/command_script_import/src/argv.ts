import assert from 'node:assert/strict';
import path from 'node:path';
assert.equal(process.execPath, process.env.EXPECTED_EXEC_PATH);
assert.equal(process.ppid, Number(process.env.EXPECTED_PARENT_PID));
assert.equal(process.argv[1], path.resolve('src/argv.ts'));
assert.equal((globalThis as any).setup, 'loaded');
console.log(JSON.stringify({ args: process.argv.slice(2), execArgv: process.execArgv }));
process.exitCode = 7;

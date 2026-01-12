import assert from 'node:assert';
import path from 'node:path';

// Assert that config is loaded with package directory as CWD (not workspace root)
assert.strictEqual(
  path.basename(process.cwd()),
  'app',
  `Expected CWD to be 'app', got ${process.cwd()}`
);

// This console.log tests that stdout noise doesn't break config resolution
// (the implementation writes to a temp file instead of stdout)
console.log('This message should not break config resolution');

export default {
  tasks: {
    build: {
      command: "echo 'build from vite.config.ts'",
      dependsOn: []
    }
  }
};

// Asserts a generated file is not matched by any ignore rule in the created
// project: `git check-ignore --no-index` exits 1 for trackable files.
// Usage: node check-trackable.cjs <project-dir> <file>
const { spawnSync } = require('node:child_process');
const [dir, file] = process.argv.slice(2);
const result = spawnSync('git', ['-C', dir, 'check-ignore', '--no-index', file], {
  stdio: 'ignore',
});
const status = result.status;
if (status === 1) {
  console.log(`${file} trackable`);
} else {
  console.log(
    status === 0
      ? `ERROR: ${file} ignored`
      : `ERROR: git check-ignore failed with status ${status}`,
  );
  process.exit(status || 1);
}

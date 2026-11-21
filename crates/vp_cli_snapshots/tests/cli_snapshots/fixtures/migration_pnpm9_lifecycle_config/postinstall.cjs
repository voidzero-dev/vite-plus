const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const { appendFileSync, existsSync, realpathSync } = require('node:fs');

if (process.env.VP_TEST_FAIL_POSTINSTALL) {
  console.log('Configuring the project');
  console.error('Fixture postinstall failed');
  process.exit(7);
}

const root = execFileSync(process.execPath, [process.env.npm_execpath, 'root'], {
  encoding: 'utf8',
}).trim();
assert.ok(existsSync(root), 'pnpm root must return an existing path');
assert.equal(realpathSync(root), realpathSync('node_modules'));

const options = ['reporter', 'loglevel', 'json', 'parseable'];
const config = Object.fromEntries(
  options.map((name) => [name, process.env['npm_config_' + name] ?? null]),
);
appendFileSync('postinstall-config.jsonl', JSON.stringify(config) + '\n');

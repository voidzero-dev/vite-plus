import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const require = createRequire(`${process.cwd()}/`);
const pkg = JSON.parse(readFileSync('package.json', 'utf-8')) as Record<
  string,
  Record<string, string> | undefined
>;

// A fresh pnpm install must load the migrated plugin without direct Oxlint dependencies.
for (const name of ['@oxlint/plugins', 'oxlint']) {
  for (const field of [
    'dependencies',
    'devDependencies',
    'peerDependencies',
    'optionalDependencies',
  ]) {
    assert.equal(pkg[field]?.[name], undefined, `${field} still contains ${name}`);
  }
  assert.throws(() => require.resolve(`${name}/package.json`), { code: 'MODULE_NOT_FOUND' });
}

const pluginDir = 'tools/oxlint/anti-slop';
for (const [file, specifier] of [
  ['index.ts', 'vite-plus/lint/plugins'],
  ['rules/padding-line-between-statements.ts', 'vite-plus/lint/plugins'],
  ['rules/tests/padding-line-between-statements.test.ts', 'vite-plus/lint/plugins-dev'],
]) {
  const source = readFileSync(`${pluginDir}/${file}`, 'utf-8');
  assert.ok(
    source.includes(`from "${specifier}"`) || source.includes(`from '${specifier}'`),
    `${file} must import from ${specifier}`,
  );
}

console.log('ok Video.js custom rules and RuleTester use the Vite+ plugin APIs');

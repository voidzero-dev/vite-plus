import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { realpathSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

const require = createRequire(import.meta.url);
const packageFile = require.resolve('vite-plus/package.json');
assert.equal(realpathSync(packageFile), realpathSync('node_modules/vite-plus/package.json'));
const compiler = path.join(path.dirname(require.resolve('typescript/package.json')), 'bin/tsc');

function check(source) {
  writeFileSync('browser-type-probe.mts', source);
  // TypeScript 7 no longer exposes the legacy JavaScript compiler API. Run
  // each probe through its CLI so declarations cannot leak between programs.
  execFileSync(process.execPath, [compiler, '--project', 'tsconfig.json'], { encoding: 'utf8' });
}

// Compile this separately: a context import can load the upstream matcher
// declarations and hide a broken vite-plus/test/matchers export.
check(`
import 'vite-plus/test/matchers';
import { expect } from 'vite-plus/test';
expect(document.body).toBeInTheDocument();
expect(document.body).toHaveFocus();
expect(document.body).toHaveAttribute('id', 'app');
// @ts-expect-error Attribute names must be strings.
expect(document.body).toHaveAttribute(123);
`);
console.log('Standalone browser matcher declarations compile');

for (const alias of [
  'browser',
  'context',
  'browser/context',
  'plugins/browser-context',
  'browser-playwright/context',
  'browser-preview/context',
  'browser/providers/playwright/context',
  'browser/providers/preview/context',
]) {
  check(`
import 'vite-plus/test/browser-playwright';
import { page, type UserEventClickOptions } from 'vite-plus/test/${alias}';
declare const click: UserEventClickOptions;
const force: boolean | undefined = click.force;
await page.getByRole('button').screenshot({ caret: 'hide' });
// @ts-expect-error Roles accept strings, not numbers.
page.getByRole(123);
// @ts-expect-error Playwright's force option must remain a boolean.
const invalidClick: UserEventClickOptions = { force: 'yes' };
`);
  console.log(`${alias}: provider augmentations and role types preserved`);
}

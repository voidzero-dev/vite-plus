/**
 * `--import` entry for `vpx <script>`; `script-preload.ts` (the `--require` entry) runs
 * first and does the rest.
 *
 * On Node.js < 26 it registers the off-thread ESM hooks through `module.register()`,
 * which Node.js 22 cannot run from a `--require`. On every version, an `--import` makes
 * Node.js run the entry point through the ESM loader, where the hooks decide each
 * file's module format.
 */

import * as NodeModule from 'node:module';

import { canRegisterSyncHooks } from './script-hooks.ts';

// Look `register` up on the namespace: a named import would fail to link on every
// version once Node.js removes the deprecated API (DEP0205), even where it is unused.
const { register } = NodeModule as Partial<typeof NodeModule>;

if (!canRegisterSyncHooks(process.versions.node)) {
  register!(new URL('./script-esm-hooks.js', import.meta.url));
}

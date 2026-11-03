import assert from 'node:assert/strict';
import { cpSync, lstatSync, mkdirSync, symlinkSync, unlinkSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';

// Keep global npm installs out of the runner's shared runtime seed.
const runtimeLink = join(process.env.VP_HOME, 'js_runtime');
assert(lstatSync(runtimeLink).isSymbolicLink());
const privateRuntime = resolve('private-runtime');
const nodePrefix = join(privateRuntime, 'node', process.versions.node);
mkdirSync(dirname(nodePrefix), { recursive: true });
cpSync(dirname(dirname(process.execPath)), nodePrefix, { recursive: true, verbatimSymlinks: true });
unlinkSync(runtimeLink);
symlinkSync(privateRuntime, runtimeLink, 'dir');
symlinkSync(nodePrefix, 'node-prefix', 'dir');
writeFileSync('.node-version', process.versions.node);

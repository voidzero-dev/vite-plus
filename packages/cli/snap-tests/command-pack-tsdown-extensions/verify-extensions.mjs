// Issue #1586: `@tsdown/exe` and `@tsdown/css` have a hard peer dependency on
// `tsdown` and import `tsdown/internal`, but Vite+ bundles tsdown internally and
// does not expose a resolvable top-level `tsdown` package. Before the fix, the
// bundled tsdown loaded them as external top-level packages, so `vp pack --exe`
// (and CSS bundling) failed with `Failed to import module "@tsdown/exe"`.
//
// They are now bundled into core, so this loads the bundled extension chunks
// directly to prove they resolve `tsdown/internal` against the bundled tsdown.
// `vp pack --exe` itself needs Node >= 25.7 (SEA), so it cannot run end-to-end
// in CI; this check is Node-version independent.
import { createRequire } from 'node:module';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const require = createRequire(import.meta.url);
const packEntry = require.resolve('@voidzero-dev/vite-plus-core/pack');
const tsdownDir = path.dirname(packEntry);

for (const chunk of ['tsdown-exe.js', 'tsdown-css.js']) {
  const mod = await import(pathToFileURL(path.join(tsdownDir, chunk)).href);
  console.log(`${chunk}: ${Object.keys(mod).sort().join(', ')}`);
}

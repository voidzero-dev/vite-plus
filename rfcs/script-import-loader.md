# RFC: Preload the Vite+ TypeScript Loader in Node.js

- Status: Implemented
- Related: [Run TypeScript Scripts with `vpx`](./vpx-script-execution.md)

## Problem

`vpx` runs TypeScript files, but a test runner, debugger, worker host, or other tool
can own the Node.js command. These integrations still need `node --import=tsx` or
`NODE_OPTIONS=--import=tsx`. Replacing the executable with `vpx` does not give them
a module to preload. The original script execution RFC deferred this public entry.

## Proposal

Export `vite-plus/register` as a side-effectful ESM preload from the existing
`vite-plus` package:

```sh
node --import=vite-plus/register ./scripts/seed.ts
node --import vite-plus/register --test ./test/example.test.ts
NODE_OPTIONS='--import=vite-plus/register' node ./scripts/seed.ts
```

Tools that accept Node.js arguments can use `["--import", "vite-plus/register"]`.
An ESM launcher can use `await import('vite-plus/register')` before it dynamically
imports TypeScript. The module exports no functions. Static imports are linked
before module evaluation, so importing the preload beside a static TypeScript
import is not a registration API.

The project must install `vite-plus`, and the Node.js process must meet its
`engines.node` range. Node.js resolves the package from the working directory for
`--import`, including `NODE_OPTIONS`. The global installation is not a fallback.
The preload does not select a runtime, alter `PATH`, rewrite arguments, spawn a
process, or replace Node.js test discovery, watch mode, eval, or stdin behavior.
Files imported from eval or stdin are transformed; the input text itself is not.
This is the same distinction as tsx's preload, rather than the tsx executable.

### Public Name

`register` describes the module's side effect: install loading hooks into the
current Node.js process. It follows the entry proposed by the script execution
RFC and the convention used by `@oxc-node/core/register` and `ts-node/register`.

The package name alone, `--import vite-plus`, would overload the existing root
configuration API with process-wide loader registration. `vite-plus/loader` names
the purpose but can suggest Node.js's older `--loader` interface.
`vite-plus/node` is less specific about the operation. `vite-plus/tsx` makes the
migration familiar but suggests the tsx implementation or exact compiler semantics.
Use one public name so documentation and tool configurations converge on the same
entry. A separate `vpx` npm package would also conflict with the original goal of
shipping the loader inside `vite-plus`.

## Registration

The public entry loads the **existing emitted** `script-preload.cjs`, then the
existing `script-register.js`. The former installs the CommonJS transform hook,
source maps, runtime-helper resolution, and synchronous ESM hooks on supported
Node.js releases. The latter registers asynchronous ESM hooks on older supported
releases. Registration completes before the next Node.js `--import` or entry file.

Both imports must retain their emitted file identities. Bundling another copy of
the preload would install a second CommonJS hook when `vpx` and the public preload
are combined. Node.js module caching must make repeated preloads and an explicit
dynamic import harmless, including a public import mixed with the CLI's existing
two flags. The public entry is listed in `sideEffects`, so bundlers do not remove
an intentional side-effect import.

The CLI continues to pass its existing two flags. Its `--require` entry runs before
user `--require` modules. With a public `--import` alone, Node.js runs all
`--require` modules first; the preload cannot retroactively transform those files.
Put a TypeScript setup file in a subsequent `--import`, as with `node --import tsx`.
No public CommonJS-only entry or legacy `--loader` API is added.

Node.js inherits `--import` in `process.execArgv` for `fork()` and workers. A
`NODE_OPTIONS` preload also reaches spawned Node.js processes through their
environment. Explicit replacement of `execArgv` or the environment follows Node.js
rules. Registration occurs independently in each JavaScript thread; the loader
worker must not recursively register itself.

## Configuration and Compatibility

The public entry uses the same transforms, resolution, dependency policy, source
maps, and errors as `vpx`. It supports TypeScript, TSX/JSX, ESM and CommonJS files,
tsconfig paths for imports, TypeScript dependencies, and runtime helpers without
installing tsx. The existing [tsx differences](../docs/guide/vpx.md#coming-from-tsx)
remain visible: this entry does not change the compiler's module-format policy,
CommonJS resolution, JSX defaults, or supported decorator syntax. The acceptance
claim is complete replacement of the **preload integration surface**, not identical
output for every program accepted by tsx.

`VP_SCRIPT_TSCONFIG` explicitly selects a config, including the value set by
`vpx --tsconfig`. `TSX_TSCONFIG_PATH` is accepted as a compatibility fallback so an
integration can replace its preload without renaming its existing config variable.
Without either, the existing `OXC_TSCONFIG_PATH` / `TS_NODE_PROJECT` behavior and
per-file discovery apply. Configuration is established before either hook path
resolves or transforms application files. The loader does not change environment
variables; children inherit the original values.

Each JavaScript thread owns its native resolver, config selection, dependency
policy, and resolution caches. Concurrent workers can select different configs or
clear their parent's config variables. Reading Rust's process environment is not
sufficient: Node.js workers have their own `process.env` values. The preload
captures the effective config in JavaScript, and `initializeLoader` explicitly
selects that path or automatic discovery. The asynchronous ESM hook receives the
same selection through `module.register` data before its first resolve.

The pinned oxc-node commit includes
[oxc-node#885](https://github.com/oxc-project/oxc-node/pull/885), which stores these
values in a loader context per thread. Async native transform tasks carry their
caller's context onto the task thread. Vite+ consumes that upstream source without
a local source patch.

Do not load tsx and this preload together. They both claim TypeScript files, and
their registration order is not a supported compatibility mechanism.

## Migration

For literal `node --import tsx` and `node --import=tsx` commands, `vp migrate`
replaces the module with `vite-plus/register` and preserves the Node.js executable,
other flags, wrappers, and arguments. It also rewrites literal `--import tsx`
values in `NODE_OPTIONS` assignments in supported script commands. Existing
source checks keep commands whose known syntax differs from the Vite+ loader.
The direct Node.js path does not inherit `vpx` CLI restrictions on eval, test
discovery, or native watch mode.

The existing rules for `tsx <file>` and legacy `tsx/esm`, `tsx/cjs`, and `--loader`
forms remain, except that `TSX_TSCONFIG_PATH` no longer prevents CLI migration:
the shared loader now supports it too. Arbitrary JavaScript configuration, source imports, CI, and shell
files are not edited automatically. The documentation shows the corresponding
manual replacement. The dependency scan continues to retain tsx if any remaining
use needs it, including its programmatic APIs or runtime dependencies.

## Validation

Use packed or installed package resolution, not only imports of source files.
CLI snapshot fixtures cover the asynchronous hook path on Node.js 22.18 and
24.11.0 and the synchronous path on Node.js 22, 24, and 26:

1. Both `--import` spellings, a tool's argument array, and `NODE_OPTIONS`.
2. ESM and CommonJS entries and imports, `.ts`, `.mts`, `.cts`, JSX, enums,
   decorators, tsconfig aliases, `.js` to `.ts`, and TypeScript dependencies.
3. `--test`, eval/stdin importing a TypeScript file, and a subsequent TypeScript
   `--import`; argument and exit-status preservation.
4. `fork()`, ESM/CommonJS workers, and spawned children through `NODE_OPTIONS`;
   concurrent workers with distinct configs and workers that clear parent config
   variables, without changing the parent's resolution.
5. Repeated registration and registration combined with `vpx`.
6. Explicit tsconfig selection, `TSX_TSCONFIG_PATH`, precedence, and source-map
   stack locations on errors.
7. An unsupported Node.js version fails before the native binding loads.
8. Migration preserves command structure, keeps unsupported uses and dependencies,
   removes tsx when unused, and makes no further changes on a second run.

Run the existing loader and migration suites as regressions, the TypeScript unit
suite, `vp check`, and a full package build. Review generated snapshot output.

The installed-package cases live in
[`command_script_import`](../crates/vp_cli_snapshots/tests/cli_snapshots/fixtures/command_script_import/snapshots.toml).
The migration integration case lives in
[`migration_tsx_preload`](../crates/vp_cli_snapshots/tests/cli_snapshots/fixtures/migration_tsx_preload/snapshots.toml),
alongside the existing loader and migration regression fixtures. Unit tests in
`packages/cli/src/migration/__tests__/tsx.spec.ts` cover shell quoting, inherited
options, and dependency retention.

## References

- [Node.js `--import`](https://nodejs.org/api/cli.html#--importmodule)
- [Node.js module customization hooks](https://nodejs.org/api/module.html#customization-hooks)
- [tsx as a Node.js enhancement](https://tsx.is/dev-api/node-cli)

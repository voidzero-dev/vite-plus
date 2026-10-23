# Running Binaries

Use `vpx`, `vp exec`, and `vp dlx` to run package binaries and scripts without switching between local installs, downloaded packages, and project-specific tools.

## Overview

`vpx` does one of two things, depending on its first argument:

- [`vpx <package>`](#running-package-binaries) runs a binary from a local or remote npm package, like `npx`.
- [`vpx <file>`](#running-scripts) runs a TypeScript or JavaScript file, like `tsx`, and [`vpx --watch <file>`](#watch-mode) runs it again when a file it uses changes, like `tsx watch`.

The other commands give stricter control over where a binary comes from:

- [`vp exec`](#vp-exec) runs a command from local `node_modules/.bin` directories, falling back to `PATH`.
- [`vp dlx`](#vp-dlx) runs a package binary without adding it as a dependency.

`vpx -v` or `vpx --version` prints the Vite+ version, like `vp --version`. In `package.json` scripts, `vpx` prints the version of the project's `vite-plus` package.

## Running Package Binaries

```bash
vpx [options] <pkg[@version]> [args...]
```

`vpx` looks for the binary in local `node_modules/.bin` directories, Vite+-managed global packages, and the system `PATH`, in that order. If none has it, `vpx` downloads the package and runs it via `vp dlx`. With `pkg@version`, `--package/-p`, or `--shell-mode`, it runs via `vp dlx` directly.

### Options

- `-p, --package <name>` installs one or more additional packages before running the command
- `-c, --shell-mode` executes the command inside a shell
- `-s, --silent` suppresses Vite+ output and only shows the command output

### Examples

```bash
vpx eslint .
vpx create-vue my-app
vpx oxlint@1.85.0 --version
vpx -p cowsay -c 'echo "hi" | cowsay'
```

Package managers and `vp run` put `node_modules/.bin` first on `PATH`, so `package.json` scripts run the `vpx` bin of the project's `vite-plus` package. That bin does not need the global CLI, and it looks only in local `node_modules/.bin` directories. It runs an installed binary directly, as the global `vpx` does, and `pkg@version`, `--package/-p`, or `--shell-mode` via `vp dlx`. A binary that is not installed is an error, so a script never downloads a package by accident:

```
error: vpx: 'eslint' is not installed in node_modules
note: Add it to devDependencies, or run `vp dlx eslint` to download and run it.
```

## Running Scripts

```bash
vpx [--tsconfig <path>] [node options] <file> [args...]
```

`vpx` runs TypeScript and JavaScript files directly, without a `tsx` or `ts-node` dependency. Scripts run on the project's Node.js version, the same one [`vp node`](/guide/env) selects, with the [oxc-node](https://github.com/oxc-project/oxc-node) loader that ships with Vite+.

```bash
vpx ./scripts/seed.ts --dry-run
vpx scripts/build.mts
vpx --watch ./server.ts
vpx --env-file=.env ./scripts/migrate.ts
vpx --tsconfig tsconfig.scripts.json ./tools/gen.ts
```

### Files and Package Names

`vpx` runs its argument as a file when it is:

- a path (starting with `./`, `../`, or `/`; on Windows also `.\`, `..\`, a drive such as `C:\`, or `\\`) ending in `.ts`, `.mts`, `.cts`, `.tsx`, `.js`, `.mjs`, `.cjs`, or `.jsx`
- a name with one of those extensions that exists as a file, such as `scripts/seed.ts`
- a path to a file whose shebang runs `vpx` (see [Shebang Scripts](#shebang-scripts))

A missing path, or a missing name with a TypeScript extension, is an error: `vpx` never downloads a package for it. After Node.js options, such as `vpx --inspect seed.ts`, only a missing path is reported by `vpx`; Node.js reports a missing name itself. A name with a JavaScript extension that is not a file, such as `highlight.js`, runs the package of that name.

### Supported Syntax

- TypeScript syntax that Node.js type stripping rejects: enums, namespaces, parameter properties, and JSX
- Decorators with `"experimentalDecorators": true`, including `emitDecoratorMetadata`; standard decorators and `accessor` fields are not supported yet
- tsconfig `paths` for `import`, `.js` imports that point at `.ts` files, and extensionless imports
- ESM and CommonJS, including `require()` of `.ts` files; a `.cts` file is CommonJS, so it uses `import x = require('...')` and `export =`, and `import`/`export` declarations there fail with an error that suggests renaming the file to `.mts`
- TypeScript published in `node_modules`
- tsconfig `jsx`, `jsxImportSource`, `jsxFactory`, `jsxFragmentFactory`, `useDefineForClassFields`, and `verbatimModuleSyntax`; see [JSX](#jsx) and [tsconfig](#tsconfig) for the defaults

`vpx` does not type-check. Run [`vp check`](/guide/check) for that.

### Options

- `--tsconfig <path>` applies one tsconfig to every file (see [tsconfig](#tsconfig))
- `--watch` runs the file again when a file it uses changes (see [Watch Mode](#watch-mode))
- Node.js options before the file, such as `--inspect`, `--test`, `--env-file`, `--require`, and `--import`, are passed to Node.js; `--require` and `--import` preloads can be TypeScript too
- Everything after the file, including `--`, is passed to the script

### Shebang Scripts

Scripts can use `vpx` as their interpreter. Make the file executable and keep its extension:

```ts
#!/usr/bin/env vpx
// scripts/release.ts
```

### In `package.json` Scripts

The `vpx` bin of the `vite-plus` package runs scripts on its own, so `"seed": "vpx ./scripts/seed.ts"` works in `package.json` scripts without the global CLI. It runs them on the Node.js that started it, which inside `vp run` and package manager scripts is the project's version.

### Watch Mode

```bash
vpx --watch [watch options] [node options] <file> [args...]
```

`vpx --watch` runs a file and runs it again when a file it uses changes, like `tsx watch`. It is not Node.js's own watch mode: `vpx` runs oxc-node's watcher (`oxnode watch`) instead, which runs the file with the `vpx` loader.

```bash
vpx --watch ./src/server.ts --port 3000
vpx --watch --include "./config/*.json" --exclude "src/**/*.test.ts" ./src/server.ts
vpx --watch --inspect --clear-screen=false ./src/index.ts
```

It watches every file the script imports or requires (in worker threads too, on Node.js 24 and later), the tsconfig files `vpx` read and the ones they `extends`, the files a failed import looked for, so creating one reruns the script, and `--env-file` files. `node_modules` is not watched unless `--include` names it. Press Return to rerun at once. Ctrl+C stops the script and the watcher; press it again to force kill. `vpx --watch --help` lists the options:

- `--include <glob>` also watches matching files, and `--exclude <glob>` never watches them
- `--clear-screen=false` (or `--no-clear-screen`) keeps the output of earlier runs; the screen is cleared only on a terminal
- `--kill-signal <signal>` stops the script before a rerun (default `SIGTERM`), and `--kill-timeout <ms>` force kills it after that long (default 5000)
- `--rerun-on-enter=false` (or `--no-rerun-on-enter`) leaves stdin to the script
- `--poll[=<ms>]` polls files instead of using file system events, for Docker, WSL, and network drives
- `--tsconfig <path>` applies one tsconfig to every file, as for `vpx <file>`

`tsx watch`'s `--ignore` and `--no-cache` and its `CHOKIDAR_USEPOLLING` variable work too (`--no-cache` does nothing: `vpx` has no transform cache), and so do the Node.js watch mode options `--watch-preserve-output` and `--watch-kill-signal`, so scripts written for `tsx watch` or `node --watch` move over. `--watch-path`, which would start Node.js's watch mode, is an error that points to `--include`, and so are `--test`, `--eval`, and other options that run no file. Other Node.js options go before the file.

### tsconfig

`vpx` reads compiler options from a tsconfig but does not type-check with it. Each file uses the nearest `tsconfig.json` above it that includes it through `files`, `include`, `exclude`, or project `references`, as `tsc` does, and `extends` is followed. Files in `node_modules` use no tsconfig, and neither does a file that no tsconfig includes, so it gets the defaults below. Such a file still resolves `paths` aliases through the nearest `tsconfig.json` above it, as tsx does: with a root tsconfig whose `include` is `["src"]`, a script in `scripts/` can import the same aliases as `src/`. Because the config depends on the file's location, not on the working directory, `vpx` compiles a file the same way wherever it runs from.

`--tsconfig <path>` applies one config to every file instead, including files that config does not include. Without `--tsconfig`, an `OXC_TSCONFIG_PATH` or `TS_NODE_PROJECT` environment variable does the same. If it names a file that does not exist, or a tsconfig fails to parse, `vpx` prints a warning and runs without that config.

A file that no tsconfig applies to runs as if its `compilerOptions` were empty:

| Option                            | Without a tsconfig                                                                                                                                                                                                                                                                                                                                       |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `module`                          | The module format comes from `package.json` `"type"`: ESM with `"module"`, CommonJS otherwise. `.mts` is always ESM and `.cts` always CommonJS. With a tsconfig `module` of `node16`, `node18`, `nodenext`, `es2015`, or later, `.ts` and `.tsx` files run as ESM regardless of `"type"`; for the `node*` values, `tsc` and tsx follow `"type"` instead. |
| `paths`, `baseUrl`                | None. Imports resolve as in Node.js, plus `.js` to `.ts` or `.tsx`, `.mjs` to `.mts`, `.cjs` to `.cts`, and extensionless imports.                                                                                                                                                                                                                       |
| `useDefineForClassFields`         | `true`: class fields have `[[Define]]` semantics and run natively, as with a `target` of `ES2022` or later. A lower `target` switches to `[[Set]]` semantics, as in `tsc`.                                                                                                                                                                               |
| `experimentalDecorators`          | `false`: a decorator fails with an error that asks for this option.                                                                                                                                                                                                                                                                                      |
| `emitDecoratorMetadata`           | `false`.                                                                                                                                                                                                                                                                                                                                                 |
| `jsx`, `jsxImportSource`          | The automatic runtime, importing `react/jsx-runtime` (see [JSX](#jsx)).                                                                                                                                                                                                                                                                                  |
| `verbatimModuleSyntax`            | `false`: imports whose bindings are unused at runtime are removed, with their side effects.                                                                                                                                                                                                                                                              |
| `rewriteRelativeImportExtensions` | `false`. `./file.ts` imports work either way.                                                                                                                                                                                                                                                                                                            |

Types are removed, and enums, namespaces, and parameter properties are compiled, whatever the tsconfig says. `strict`, `lib`, and other type-checking options have no effect, and `target` only changes the class field default.

### JSX

`.jsx` and `.tsx` files compile to calls into a JSX runtime, React's by default. Install the runtime they use, `react` or the package named by `jsxImportSource`; Vite+ does not ship one. tsconfig options select the runtime:

| tsconfig `jsx`                 | Output                                                                                                                             |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| not set, or `"react-jsx"`      | The automatic runtime: `jsx()` and `jsxs()` from `<jsxImportSource>/jsx-runtime`, with `jsxImportSource` defaulting to `react`     |
| `"react-jsxdev"`               | The development runtime: `jsxDEV()` from `<jsxImportSource>/jsx-dev-runtime`, with the file name, line, and column of each element |
| `"react"`                      | The classic runtime: `React.createElement()`, or `jsxFactory`, with fragments as `React.Fragment`, or `jsxFragmentFactory`         |
| `"preserve"`, `"react-native"` | The automatic runtime, because Node.js cannot run JSX                                                                              |

A classic factory that the file imports, such as `import { h, Fragment } from 'my-jsx'`, is kept even though only JSX uses it.

Pragma comments override the tsconfig for that file:

```tsx
/** @jsxImportSource my-jsx */
```

```tsx
/** @jsxRuntime classic */
/** @jsx h */
/** @jsxFrag Fragment */
import { h, Fragment } from 'my-jsx';
```

When a file has both `@jsx` and `@jsxImportSource`, `@jsxImportSource` selects the automatic runtime and `@jsx` is ignored. Pragmas are read from every comment in the file, so prose such as `// @jsxImportSource is set above` in an ordinary comment changes the runtime too.

A `.jsx` file gets a tsconfig's JSX options only when that tsconfig sets `"allowJs": true`, as in `tsc` and tsx. Otherwise it uses the defaults above.

JSX that depends on a compiler plugin is not supported. `vpx` compiles JSX only into calls to a runtime's `jsx()`, `jsxs()`, and `jsxDEV()` exports or to a classic factory. A setup that relies on a Babel or bundler plugin to transform JSX, usually with `"jsx": "preserve"`, fails at runtime or loses what that plugin adds.

### Coming from tsx

`vp migrate` rewrites tsx commands in `package.json` scripts to `vpx` and removes the `tsx` devDependency once nothing uses it; see [tsx to `vpx`](/guide/migrate-rules#tsx-to-vpx). Most scripts that run with tsx run with `vpx` unchanged. These differences can need a change:

- **JSX default**: without a tsconfig `jsx` option, tsx compiles JSX to `React.createElement()`, which needs `React` in scope, while `vpx` uses the automatic runtime. Set `"jsx": "react"` to keep the classic output. Because of its default, tsx also ignores a lone `/** @jsxImportSource … */` comment unless the file has `/** @jsxRuntime automatic */` or the tsconfig sets `"jsx": "react-jsx"`; `vpx` applies the comment on its own.
- **`.cts` files with `import` or `export`**: tsx compiles them to CommonJS, and `vpx` reports an error. Rename the file to `.mts`, or write it with `import x = require()` and `export =`.
- **`import` in a CommonJS package**: in a package without `"type": "module"`, tsx compiles a `.ts` or `.js` file that uses `import`, `export`, or `import.meta` to CommonJS, so `__filename`, `__dirname`, and `require` still work. `vpx` runs such a file as an ES module, as Node.js does, so they are not defined, and a default import of it from an ES module gets its default export rather than the whole module. Use `import.meta.filename`, `import.meta.dirname`, and `createRequire(import.meta.url)` instead.
- **`require()` resolution**: tsx maps `require('./a.js')` to `a.ts` and applies tsconfig `paths` to `require()`. `vpx` resolves `require()` as Node.js does, so write the `.ts` extension, and use `paths` aliases only with `import`.
- **Signals**: tsx turns `SIGINT` and `SIGTERM` into exit codes 130 and 143 and runs `process.on('exit')` handlers first. Under `vpx` the signal ends the script, as it does under Node.js, so clean up in a `process.on('SIGINT')` or `process.on('SIGTERM')` handler.
- **Node.js version**: `vpx` needs Node.js `^22.18.0 || ^24.11.0 || >=26.0.0`, while tsx also runs on older releases.
- **Watch mode**: `tsx watch` becomes `vpx --watch`, which takes tsx watch's options (see [Watch Mode](#watch-mode)). It also reruns the script on tsconfig edits and when a file a failed import looked for is created, which tsx watch misses.
- **Eval, REPL, and `--check`**: `vpx --eval`, the REPL, and `vpx --check` do not transform TypeScript.
- **`node --import tsx`**: `vpx` has no equivalent for tools that must start `node` themselves.
- **Other loaders**: remove `--import tsx` from `NODE_OPTIONS`. Two loaders registered together compete for the same files.

## `vp exec`

Use `vp exec` to run tools already installed in your project or available in your environment.

```bash
vp exec <command> [args...]
```

Examples:

```bash
vp exec eslint .
vp exec tsc --noEmit
```

## `vp dlx`

Use `vp dlx` for one-off package execution without adding the package to your project dependencies. See the [dlx command reference](/package-management/dlx) for options and package-manager compatibility.

```bash
vp dlx <package> [args...]
```

Examples:

```bash
vp dlx create-vite
vp dlx oxlint@1.85.0 --version
```

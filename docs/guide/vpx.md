# Running Binaries

Use `vpx`, `vp exec`, and `vp dlx` to run package binaries and scripts without switching between local installs, downloaded packages, and project-specific tools.

## Overview

`vpx` does one of two things, depending on its first argument:

- [`vpx <package>`](#running-package-binaries) runs a binary from a local or remote npm package, like `npx`.
- [`vpx <file>`](#running-scripts) runs a TypeScript or JavaScript file, like `tsx`.

The other commands give stricter control over where a binary comes from:

- [`vp exec`](#vp-exec) runs a command from local `node_modules/.bin` directories, falling back to `PATH`.
- [`vp dlx`](#vp-dlx) runs a package binary without adding it as a dependency.

`vpx -v` or `vpx --version` prints the Vite+ version, like `vp --version`.

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

Running package binaries needs the global Vite+ CLI. In `package.json` scripts, the `vpx` bin of the project's `vite-plus` package hands package commands to it.

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

- a path (starting with `./`, `../`, or `/`) ending in `.ts`, `.mts`, `.cts`, `.tsx`, `.js`, `.mjs`, `.cjs`, or `.jsx`
- a name with one of those extensions that exists as a file, such as `scripts/seed.ts`
- a path to an executable file whose shebang runs `vpx` (see [Shebang Scripts](#shebang-scripts))

A missing path, or a missing name with a TypeScript extension, is an error: `vpx` never downloads a package for it. A name with a JavaScript extension that is not a file, such as `highlight.js`, runs the package of that name.

### Supported Syntax

- TypeScript syntax that Node.js type stripping rejects: enums, namespaces, parameter properties, and JSX
- Decorators with `"experimentalDecorators": true`, including `emitDecoratorMetadata`; standard decorators are not supported yet
- tsconfig `paths` for `import`, `.js` imports that point at `.ts` files, and extensionless imports
- ESM and CommonJS, including `require()` of `.ts` files; a `.cts` file is CommonJS, so it uses `import x = require('...')` and `export =`, and `import`/`export` declarations there fail with an error that suggests renaming the file to `.mts`
- TypeScript published in `node_modules`
- tsconfig `jsx`, `jsxImportSource`, `jsxFactory`, `jsxFragmentFactory`, `useDefineForClassFields`, and `verbatimModuleSyntax`; see [tsconfig](#tsconfig) for the defaults without one

`vpx` does not type-check. Run [`vp check`](/guide/check) for that.

### Options

- `--tsconfig <path>` applies one tsconfig to every file (see [tsconfig](#tsconfig))
- Node.js options before the file, such as `--watch`, `--inspect`, `--test`, `--env-file`, `--require`, and `--import`, are passed to Node.js; `--require` and `--import` preloads can be TypeScript too
- Everything after the file, including `--`, is passed to the script

### Shebang Scripts

Scripts can use `vpx` as their interpreter. Make the file executable and keep its extension:

```ts
#!/usr/bin/env vpx
// scripts/release.ts
```

### In `package.json` Scripts

The `vpx` bin of the `vite-plus` package runs scripts on its own, so `"seed": "vpx ./scripts/seed.ts"` works in `package.json` scripts without the global CLI.

### tsconfig

`vpx` reads compiler options from a tsconfig but does not type-check with it. Each file uses the nearest `tsconfig.json` above it that includes it through `files`, `include`, `exclude`, or project `references`, as `tsc` does, and `extends` is followed. Files in `node_modules` use no tsconfig.

`--tsconfig <path>` applies one config to every file instead, including files that config does not include. Without `--tsconfig`, an `OXC_TSCONFIG_PATH` or `TS_NODE_PROJECT` environment variable does the same.

A file that no tsconfig applies to runs as if its `compilerOptions` were empty:

| Option                            | Without a tsconfig                                                                                                                                                                                                                                                                                                                                       |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `module`                          | The module format comes from `package.json` `"type"`: ESM with `"module"`, CommonJS otherwise. `.mts` is always ESM and `.cts` always CommonJS. With a tsconfig `module` of `node16`, `node18`, `nodenext`, `es2015`, or later, `.ts` and `.tsx` files run as ESM regardless of `"type"`; for the `node*` values, `tsc` and tsx follow `"type"` instead. |
| `paths`, `baseUrl`                | None. Imports resolve as in Node.js, plus `.js` to `.ts` or `.tsx`, `.mjs` to `.mts`, `.cjs` to `.cts`, and extensionless imports.                                                                                                                                                                                                                       |
| `useDefineForClassFields`         | `true`: class fields have `[[Define]]` semantics and run natively, as with a `target` of `ES2022` or later. A lower `target` switches to `[[Set]]` semantics, as in `tsc`.                                                                                                                                                                               |
| `experimentalDecorators`          | `false`: a decorator fails with an error that asks for this option.                                                                                                                                                                                                                                                                                      |
| `emitDecoratorMetadata`           | `false`.                                                                                                                                                                                                                                                                                                                                                 |
| `jsx`, `jsxImportSource`          | The automatic runtime, importing `react/jsx-runtime`.                                                                                                                                                                                                                                                                                                    |
| `verbatimModuleSyntax`            | `false`: imports whose bindings are unused at runtime are removed, with their side effects.                                                                                                                                                                                                                                                              |
| `rewriteRelativeImportExtensions` | `false`. `./file.ts` imports work either way.                                                                                                                                                                                                                                                                                                            |

Types are removed, and enums, namespaces, and parameter properties are compiled, whatever the tsconfig says. `strict`, `lib`, and other type-checking options have no effect, and `target` only changes the class field default.

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

Use `vp dlx` for one-off package execution without adding the package to your project dependencies.

```bash
vp dlx <package> [args...]
```

Examples:

```bash
vp dlx create-vite
vp dlx oxlint@1.85.0 --version
```

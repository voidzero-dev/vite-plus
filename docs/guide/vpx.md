# Running Binaries

Use `vpx`, `vp exec`, and `vp dlx` to run binaries without switching between local installs, downloaded packages, and project-specific tools.

## Overview

`vpx` executes a command from a local or remote npm package. It can run a package that is already available locally, download a package on demand, or target an explicit package version.

Use the other binary commands when you need stricter control:

- `vpx` looks for a binary in local `node_modules/.bin` directories, Vite+-managed global packages, and system `PATH`, in that order, then falls back to `vp dlx`. With `pkg@version`, `--package/-p`, or `--shell-mode`, it runs via `vp dlx` directly.
- `vpx <file>` runs a TypeScript or JavaScript file with the Vite+ script loader
- `vp exec` runs a command from local `node_modules/.bin` directories, falling back to `PATH` if not found
- `vp dlx` runs a package binary without adding it as a dependency

## `vpx`

Use `vpx` for running any local or remote binary:

```bash
vpx <pkg[@version]> [args...]
```

### Options

- `-p, --package <name>` installs one or more additional packages before running the command
- `-c, --shell-mode` executes the command inside a shell
- `-s, --silent` suppresses Vite+ output and only shows the command output
- `--tsconfig <path>` selects the tsconfig when running a script (see [tsconfig](#tsconfig))
- `-v, --version` prints the Vite+ version, like `vp --version`

### Examples

```bash
vpx eslint .
vpx create-vue my-app
vpx oxlint@1.85.0 --version
vpx -p cowsay -c 'echo "hi" | cowsay'
```

### Running Scripts

::: warning Experimental
Running script files with `vpx` is experimental. Its loader, [oxc-node](https://github.com/oxc-project/oxc-node), is experimental too.
:::

`vpx` also runs TypeScript and JavaScript files directly, without a `tsx` or `ts-node` dependency:

```bash
vpx ./scripts/seed.ts --dry-run
vpx scripts/build.mts
vpx --watch ./server.ts
vpx --env-file=.env ./scripts/migrate.ts
vpx --tsconfig tsconfig.scripts.json ./tools/gen.ts
```

When the command is a file ending in `.ts`, `.mts`, `.cts`, `.tsx`, `.js`, `.mjs`, `.cjs`, or `.jsx`, `vpx` runs it on the project's Node.js version, the same one [`vp node`](/guide/env) selects, with the [oxc-node](https://github.com/oxc-project/oxc-node) loader that ships with Vite+. The loader supports:

- TypeScript syntax that Node.js type stripping rejects: enums, namespaces, parameter properties, and JSX
- Decorators with `"experimentalDecorators": true`, including `emitDecoratorMetadata`; standard decorators are not supported yet
- tsconfig `paths` for `import`, `.js` imports that point at `.ts` files, and extensionless imports
- ESM and CommonJS, including `require()` of `.ts` files; a `.cts` file is CommonJS, so it uses `import x = require('...')` and `export =`, and `import`/`export` declarations there fail with an error that suggests renaming the file to `.mts`
- TypeScript published in `node_modules`
- tsconfig `jsx`, `jsxImportSource`, `jsxFactory`, `jsxFragmentFactory`, `useDefineForClassFields`, and `verbatimModuleSyntax`; see [tsconfig](#tsconfig) for the defaults without one

Options before the script, such as `--watch`, `--inspect`, `--test`, `--env-file`, `--require`, and `--import`, are passed to Node.js, and `--require` or `--import` preloads can be TypeScript too. Everything after the script, including `--`, is passed to the script. A missing script is an error; `vpx` never downloads a package for a path.

`vpx` does not type-check. Run [`vp check`](/guide/check) for that.

Scripts can also use `vpx` as their interpreter. Make the file executable and keep its extension:

```ts
#!/usr/bin/env vpx
// scripts/release.ts
```

The `vpx` bin of the `vite-plus` package runs scripts on its own, so `"seed": "vpx ./scripts/seed.ts"` works in `package.json` scripts without the global CLI. Running package binaries with `vpx` still needs the global CLI.

#### tsconfig

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

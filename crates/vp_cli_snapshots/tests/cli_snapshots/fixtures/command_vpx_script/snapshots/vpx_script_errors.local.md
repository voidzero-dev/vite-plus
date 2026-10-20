# vpx_script_errors

Script paths never fall back to a remote package download.

## `vpx ./src/missing.ts`

Missing explicit path

**Exit code:** 1

```
error: vpx: Script not found: ./src/missing.ts
```

## `vpx seed.ts`

Missing bare name with a TypeScript extension

**Exit code:** 1

```
error: vpx: Script not found: seed.ts
```

## `vpx --tsconfig missing.json ./src/syntax.ts`

Missing tsconfig

**Exit code:** 1

```
error: vpx: tsconfig not found: missing.json
```

## `vpx --tsconfig`

--tsconfig without a path, as a vpx option

**Exit code:** 1

```
error: vpx: --tsconfig requires a path
```

## `vpx -p cowsay ./src/syntax.ts`

--package selects package mode

**Exit code:** 1

```
error: vpx: --package cannot be used when running a script
```

## `vpx -p cowsay ./src/missing.ts`

A missing explicit path never falls through to a package download, even with --package

**Exit code:** 1

```
error: vpx: Script not found: ./src/missing.ts
```

## `vpx -p cowsay seed.ts`

Nor does a missing bare name with a TypeScript extension

**Exit code:** 1

```
error: vpx: Script not found: seed.ts
```

## `vpx ./bin/missing`

Nor a missing explicit path without an extension

**Exit code:** 1

```
error: vpx: Script not found: ./bin/missing
```

## `vpx`

Without a command, both CLIs print the same usage

**Exit code:** 1

```
error: vpx requires a command to run

Usage: vpx <pkg[@version]> [args...]
       vpx <script> [args...]

Examples:
  vpx eslint .
  vpx create-vue my-app
  vpx ./scripts/seed.ts
```

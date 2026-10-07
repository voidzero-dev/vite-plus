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

## `vpx -p cowsay ./src/syntax.ts`

--package selects package mode

**Exit code:** 1

```
error: vpx: --package cannot be used when running a script
```

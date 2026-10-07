# vpx_script_resolution

Module resolution matches tsx: tsconfig paths, .js to .ts, extensionless imports, JSON, and CommonJS.

## `vpx src/resolution.ts`

A bare relative path runs when the file exists

```
hello vpx from src/lib
4 6 42 cts counter=1
```

## `vpx --tsconfig tsconfig.alt.json src/resolution.ts`

--tsconfig replaces per-file tsconfig selection

```
hello vpx from alt
4 6 42 cts counter=1
```

## `vpx --no-warnings --tsconfig=tsconfig.alt.json src/resolution.ts`

--tsconfig is also recognized among Node.js options

```
hello vpx from alt
4 6 42 cts counter=1
```

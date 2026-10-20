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

## `cd order && vpx ./main.ts`

A .ts next to its compiled .js wins for ./dep.js and ./dep, as with tsc and tsx

```
./dep.js -> dep.ts | ./dep -> dep.ts
```

## `cd outside-include && vpx ./scripts/seed.ts`

A file outside every tsconfig include resolves paths through the nearest tsconfig

```
alias from scripts/: src/name.ts
```

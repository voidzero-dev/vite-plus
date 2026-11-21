# pack_inline_type_with_standalone_release

Type-check migrated pack options and build with a retained tsdown release.

## `vpt json-edit package.json devDependencies.tsdown 0.22.2`


## `vpt json-edit package.json devDependencies.typescript 5.9.3`


## `vpt json-edit package.json scripts.bundle 'tsdown -c release.config.mts'`


## `vpt write-file vite.config.ts 'import type { UserConfig } from '\''tsdown'\'';
export default { pack: { entry: '\''src/library.ts'\'', outDir: '\''typed-out'\'' } satisfies UserConfig };
'`


## `vpt write-file release.config.mts 'import type { UserConfig } from '\''tsdown'\'';
export default { entry: '\''src/library.ts'\'', outDir: '\''legacy-out'\'', dts: false } satisfies UserConfig;
'`


## `vpt write-file tsconfig.json '{"compilerOptions": {"target": "ES2022", "module": "ESNext", "moduleResolution": "Bundler", "skipLibCheck": true, "strict": true, "noUnusedLocals": true}, "include": ["vite.config.ts", "release.config.mts"]}'`


## `vp install --ignore-scripts`


## `pnpm exec tsc --noEmit`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `pnpm exec tsc --noEmit`


## `vpt print-file vite.config.ts`

```
import type { UserConfig } from "vite-plus/pack";
export default {
  fmt: {},
  lint: {
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: { "vite-plus/prefer-vite-plus-imports": "error" },
    options: { typeAware: true, typeCheck: true },
  },
  pack: {
    deps: {
      // tsdown <0.23 compatibility: resolve external dependency subpaths.
      // Remove to preserve subpath imports as written (the new default).
      // https://tsdown.dev/options/dependencies#deps-resolvedepsubpath
      resolveDepSubpath: true,
    },
    entry: "src/library.ts",
    outDir: "typed-out",
  } satisfies UserConfig,
};
```

## `vpt print-file release.config.mts`

```
import type { UserConfig } from "tsdown";
export default { entry: "src/library.ts", outDir: "legacy-out", dts: false } satisfies UserConfig;
```

## `vp pack`


## `vpt stat-file typed-out/library.mjs --assert file`

```
typed-out/library.mjs: file
```

## `pnpm run bundle`


## `vpt stat-file legacy-out/library.mjs --assert file`

```
legacy-out/library.mjs: file
```

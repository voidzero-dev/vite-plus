# focused_namespace_pack_types

Type-check namespace imports in pack options after installing only the workspace member.

## `vpt json-edit package.json devDependencies '{"vite":"8.3.8"}'`


## `vpt write-file pnpm-workspace.yaml 'packages:
  - '\''packages/*'\''
sharedWorkspaceLockfile: false
'`


## `vpt mkdir -p packages/generator/src`


## `vpt write-file packages/generator/package.json '{"name":"generator","private":true,"type":"module","scripts":{"release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.2","typescript":"5.9.3"}}'`


## `vpt write-file packages/generator/src/index.ts 'export const release = true;
'`


## `vpt write-file packages/generator/vite.config.ts 'import type * as tsdown from '\''tsdown'\'';
export default { pack: { entry: '\''src/index.ts'\'', outDir: '\''pack-out'\'' } satisfies tsdown.UserConfig };
'`


## `vpt write-file packages/generator/release.config.mts 'import type * as tsdown from '\''tsdown'\'';
export default { entry: '\''src/index.ts'\'', outDir: '\''release-out'\'', dts: false } satisfies tsdown.UserConfig;
'`


## `vpt write-file packages/generator/tsconfig.json '{"compilerOptions": {"target": "ES2022", "module": "ESNext", "moduleResolution": "Bundler", "skipLibCheck": true, "strict": true, "noUnusedLocals": true}, "include": ["vite.config.ts", "release.config.mts"]}'`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `pnpm --filter generator install --ignore-scripts`


## `vpt stat-file node_modules/vite-plus --assert missing`

```
node_modules/vite-plus: missing
```

## `vpt stat-file packages/generator/node_modules/vite-plus/package.json --assert file`

```
packages/generator/node_modules/vite-plus/package.json: file
```

## `cd packages/generator && pnpm exec tsc --noEmit`


## `vpt print-file packages/generator/vite.config.ts`

```
import type * as tsdown from 'vite-plus/pack';
export default { pack: { deps: {
    // tsdown <0.23 compatibility: resolve external dependency subpaths.
    // Remove to preserve subpath imports as written (the new default).
    // https://tsdown.dev/options/dependencies#deps-resolvedepsubpath
    resolveDepSubpath: true }, entry: 'src/index.ts', outDir: 'pack-out' } satisfies tsdown.UserConfig };
```

## `vpt print-file packages/generator/release.config.mts`

```
import type * as tsdown from 'tsdown';
export default { entry: 'src/index.ts', outDir: 'release-out', dts: false } satisfies tsdown.UserConfig;
```

## `cd packages/generator && pnpm run release`


## `vpt stat-file packages/generator/release-out/index.mjs --assert file`

```
packages/generator/release-out/index.mjs: file
```

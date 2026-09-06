# core_module_identity_pnpm_peer_contexts

## `vpt json-edit package.json packageManager pnpm@11.24.0`


## `vpt write-file pnpm-workspace.yaml 'packages:
  - packages/*
resolvePeersFromWorkspaceRoot: false
overrides:
  vite: npm:@voidzero-dev/vite-plus-core@latest
minimumReleaseAge: 0
'`


## `vpt write-file packages/a/package.json '{"name":"app-a","private":true,"type":"module","devDependencies":{"vite":"npm:@voidzero-dev/vite-plus-core@latest","vite-plus":"latest","@types/node":"22.20.1"}}
'`


## `vpt write-file packages/b/package.json '{"name":"app-b","private":true,"type":"module","devDependencies":{"vite":"npm:@voidzero-dev/vite-plus-core@latest","vite-plus":"latest","@types/node":"24.10.3"}}
'`


## `vpt cp check-api.mjs packages/a/check-api.mjs`


## `vpt cp check-api.mjs packages/b/check-api.mjs`


## `vpt cp vite.config.ts packages/a/vite.config.ts`


## `vpt cp vite.config.ts packages/b/vite.config.ts`


## `vp install --ignore-scripts`


## `cd packages/a && node check-api.mjs`

```
Packed alias, ESM, CommonJS, and module-runner identity passed
```

## `cd packages/a && vp dev`

```

  VITE <version>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
SSR environment identity and HTTP response passed
```

## `cd packages/b && node check-api.mjs`

```
Packed alias, ESM, CommonJS, and module-runner identity passed
```

## `cd packages/b && vp dev`

```

  VITE <version>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
SSR environment identity and HTTP response passed
```

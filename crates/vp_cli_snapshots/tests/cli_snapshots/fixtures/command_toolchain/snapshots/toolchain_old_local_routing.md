# toolchain_old_local_routing

## `vpt mkdir -p node_modules/vite-plus/dist`


## `vpt write-file node_modules/vite-plus/package.json '{"name":"vite-plus","version":"0.1.0"}
'`


## `vpt write-file node_modules/vite-plus/dist/bin.js 'console.error("error: Command '\''toolchain'\'' not found");
process.exitCode = 2;
'`


## `vp toolchain`

the global binary delegates and lets an old local CLI reject the command

**Exit code:** 2

```
error: Command 'toolchain' not found
```

## `vpt rm -f node_modules/vite-plus/dist/bin.js`


## `vpt write-file node_modules/vite-plus/dist/toolchain.json '{"schemaVersion":1,"nodes":[{"id":"vite-plus","name":"vite-plus","version":"0.1.0","kind":"package","delivery":["dependency"],"aliases":[]},{"id":"vite","name":"vite","version":"0.1.0","kind":"tool","delivery":["bundled"],"aliases":[]}],"edges":[{"from":"vite-plus","to":"vite","relationship":"bundles"}]}
'`


## `vp toolchain`

a package without a runnable local CLI uses the global toolchain

```
Vite+ toolchain (global)

vite-plus@<version>
├── depends on @voidzero-dev/vite-plus-core@<version>
│   ├── bundles vite@<version>
│   │   └── uses rolldown@<version>
│   │       ├── compiles oxc@<version>
│   │       └── compiles oxc-resolver@<version>
│   ├── bundles rolldown@<version>
│   │   ├── compiles oxc@<version>
│   │   └── compiles oxc-resolver@<version>
│   └── bundles tsdown@<version>
├── depends on vitest@<version>
├── depends on oxlint@<version>
├── depends on oxlint-tsgolint@<version>
├── depends on oxfmt@<version>
└── compiles vite-task (built <build-time>, revision <revision>)
```

## `vp why vite`

the hint uses the global manifest when the local CLI is not runnable

```

Vite+ also provides vite@<version> through its toolchain.
Run `vp toolchain vite` to show these versions and relationships.
```

## `vp toolchain --global`

--global skips the old local package

```
Vite+ toolchain (global)

vite-plus@<version>
├── depends on @voidzero-dev/vite-plus-core@<version>
│   ├── bundles vite@<version>
│   │   └── uses rolldown@<version>
│   │       ├── compiles oxc@<version>
│   │       └── compiles oxc-resolver@<version>
│   ├── bundles rolldown@<version>
│   │   ├── compiles oxc@<version>
│   │   └── compiles oxc-resolver@<version>
│   └── bundles tsdown@<version>
├── depends on vitest@<version>
├── depends on oxlint@<version>
├── depends on oxlint-tsgolint@<version>
├── depends on oxfmt@<version>
└── compiles vite-task (built <build-time>, revision <revision>)
```

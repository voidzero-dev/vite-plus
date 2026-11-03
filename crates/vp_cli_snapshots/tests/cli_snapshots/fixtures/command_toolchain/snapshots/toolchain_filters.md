# toolchain_filters

## `vp toolchain vite`

a tool filter keeps its ownership and engine chain

```
Vite+ toolchain (local)

vite-plus@<version>
└── depends on @voidzero-dev/vite-plus-core@<version>
    └── bundles vite@<version>
        └── uses rolldown@<version>
            ├── compiles oxc@<version>
            └── compiles oxc-resolver@<version>
```

## `vp toolchain vite vitest`

multiple filters return a stable union

```
Vite+ toolchain (local)

vite-plus@<version>
├── depends on @voidzero-dev/vite-plus-core@<version>
│   └── bundles vite@<version>
│       └── uses rolldown@<version>
│           ├── compiles oxc@<version>
│           └── compiles oxc-resolver@<version>
└── depends on vitest@<version>
```

## `vp toolchain vite-plus-core tsgolint vite-task`

stable IDs and declared aliases resolve

```
Vite+ toolchain (local)

vite-plus@<version>
├── depends on @voidzero-dev/vite-plus-core@<version>
├── depends on oxlint-tsgolint@<version>
└── compiles vite-task (built <build-time>, revision <revision>)
```

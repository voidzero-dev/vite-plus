# toolchain_filters

## `vp toolchain vite`

a tool filter keeps its ownership and engine chain

```
Vite+ toolchain (local)

vite-plus@0.2.8
`-- depends on @voidzero-dev/vite-plus-core@<version>
    `-- bundles vite@8.2.0
        `-- uses rolldown@1.2.2
            |-- compiles oxc@0.142.0
            `-- compiles oxc-resolver@11.24.2
```

## `vp toolchain vite vitest`

multiple filters return a stable union

```
Vite+ toolchain (local)

vite-plus@0.2.8
|-- depends on @voidzero-dev/vite-plus-core@<version>
|   `-- bundles vite@8.2.0
|       `-- uses rolldown@1.2.2
|           |-- compiles oxc@0.142.0
|           `-- compiles oxc-resolver@11.24.2
`-- depends on vitest@4.1.10
```

## `vp toolchain vite-plus-core tsgolint vite-task`

stable IDs and declared aliases resolve

```
Vite+ toolchain (local)

vite-plus@0.2.8
|-- depends on @voidzero-dev/vite-plus-core@<version>
|-- depends on oxlint-tsgolint@7.0.2001
`-- compiles vite-task (built <build-time>, revision ebe583739b0b1e7828199b9ee9dd52273fa2fd20)
```

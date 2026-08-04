# toolchain_full_global

The global flag reads the manifest paired with the global binary.

## `vp toolchain --global`

```
Vite+ toolchain (global)

vite-plus@0.2.8
|-- depends on @voidzero-dev/vite-plus-core@<version>
|   |-- bundles vite@8.2.0
|   |   `-- uses rolldown@1.2.2
|   |       |-- compiles oxc@0.142.0
|   |       `-- compiles oxc-resolver@11.24.2
|   |-- bundles rolldown@1.2.2
|   |   |-- compiles oxc@0.142.0
|   |   `-- compiles oxc-resolver@11.24.2
|   `-- bundles tsdown@0.22.14
|-- depends on vitest@4.1.10
|-- depends on oxlint@1.76.0
|-- depends on oxlint-tsgolint@7.0.2001
|-- depends on oxfmt@0.61.0
`-- compiles vite-task (built <build-time>, revision ebe583739b0b1e7828199b9ee9dd52273fa2fd20)
```

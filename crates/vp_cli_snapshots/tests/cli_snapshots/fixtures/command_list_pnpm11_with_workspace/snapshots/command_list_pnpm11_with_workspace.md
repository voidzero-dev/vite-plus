# command_list_pnpm11_with_workspace

## `vp install`

should install packages first

```
VITE+ - The Unified Toolchain for the Web

Scope: all 3 workspace projects

Done in <duration> using pnpm <version>
```

## `vp pm list`

should list current workspace root dependencies

```
Legend: production dependency, optional only, dev only

app@1.0.0 <workspace>/packages/app
│
│   dependencies:
├── @vite-plus-test/utils@link:../utils
└── test-vite-plus-package-optional@1.0.0

@vite-plus-test/utils@1.0.0 <workspace>/packages/utils (PRIVATE)
│
│   dependencies:
└── testnpm2@1.0.1

3 packages in 3 projects
```

## `vp pm list --recursive`

should list all packages in workspace recursively

```
Legend: production dependency, optional only, dev only

app@1.0.0 <workspace>/packages/app
│
│   dependencies:
├── @vite-plus-test/utils@link:../utils
└── test-vite-plus-package-optional@1.0.0

@vite-plus-test/utils@1.0.0 <workspace>/packages/utils (PRIVATE)
│
│   dependencies:
└── testnpm2@1.0.1

3 packages in 3 projects
```

## `vp pm list --filter app`

should list specific workspace package (uses --filter app list)

```
Legend: production dependency, optional only, dev only

app@1.0.0 <workspace>/packages/app
│
│   dependencies:
├── @vite-plus-test/utils@link:../utils
└── test-vite-plus-package-optional@1.0.0

2 packages
```

## `vp pm list --filter app --filter @vite-plus-test/utils`

should list multiple workspace packages

```
Legend: production dependency, optional only, dev only

app@1.0.0 <workspace>/packages/app
│
│   dependencies:
├── @vite-plus-test/utils@link:../utils
└── test-vite-plus-package-optional@1.0.0

@vite-plus-test/utils@1.0.0 <workspace>/packages/utils (PRIVATE)
│
│   dependencies:
└── testnpm2@1.0.1

3 packages in 2 projects
```

## `vp pm list --recursive --json`

should list all workspace packages in JSON format

```
[
  {
    "name": "command-list-pnpm11-with-workspace",
    "version": "1.0.0",
    "path": "<workspace>",
    "private": false
  },
  {
    "name": "app",
    "version": "1.0.0",
    "path": "<workspace>/packages/app",
    "private": false,
    "dependencies": {
      "@vite-plus-test/utils": {
        "from": "@vite-plus-test/utils",
        "version": "link:../utils",
        "path": "<workspace>/packages/utils"
      },
      "test-vite-plus-package-optional": {
        "from": "test-vite-plus-package-optional",
        "version": "1.0.0",
        "resolved": "https://registry.npmjs.org/test-vite-plus-package-optional/-/test-vite-plus-package-optional-1.0.0.tgz",
        "path": "<workspace>/node_modules/.pnpm/test-vite-plus-package-optional@1.0.0/node_modules/test-vite-plus-package-optional"
      }
    }
  },
  {
    "name": "@vite-plus-test/utils",
    "version": "1.0.0",
    "path": "<workspace>/packages/utils",
    "private": true,
    "dependencies": {
      "testnpm2": {
        "from": "testnpm2",
        "version": "1.0.1",
        "resolved": "https://registry.npmjs.org/testnpm2/-/testnpm2-1.0.1.tgz",
        "path": "<workspace>/node_modules/.pnpm/testnpm2@1.0.1/node_modules/testnpm2"
      }
    }
  }
]
```

## `vp pm list --recursive --depth 0`

should list workspace packages with depth limit

```
Legend: production dependency, optional only, dev only

app@1.0.0 <workspace>/packages/app
│
│   dependencies:
├── @vite-plus-test/utils@link:../utils
└── test-vite-plus-package-optional@1.0.0

@vite-plus-test/utils@1.0.0 <workspace>/packages/utils (PRIVATE)
│
│   dependencies:
└── testnpm2@1.0.1

3 packages in 3 projects
```

## `vp pm list --recursive --only-projects`

should list only workspace projects (pnpm-specific)

```
Legend: production dependency, optional only, dev only

app@1.0.0 <workspace>/packages/app
│
│   dependencies:
└── @vite-plus-test/utils@link:../utils

1 package in 3 projects
```

## `vp pm list --recursive --exclude-peers`

should exclude peer dependencies in workspace

```
Legend: production dependency, optional only, dev only

app@1.0.0 <workspace>/packages/app
│
│   dependencies:
├── @vite-plus-test/utils@link:../utils
└── test-vite-plus-package-optional@1.0.0

@vite-plus-test/utils@1.0.0 <workspace>/packages/utils (PRIVATE)
│
│   dependencies:
└── testnpm2@1.0.1

3 packages in 3 projects
```

## `vp pm list --recursive --prod`

should list production dependencies in workspace

```
Legend: production dependency, optional only, dev only

app@1.0.0 <workspace>/packages/app
│
│   dependencies:
├── @vite-plus-test/utils@link:../utils
└── test-vite-plus-package-optional@1.0.0

@vite-plus-test/utils@1.0.0 <workspace>/packages/utils (PRIVATE)
│
│   dependencies:
└── testnpm2@1.0.1

3 packages in 3 projects
```

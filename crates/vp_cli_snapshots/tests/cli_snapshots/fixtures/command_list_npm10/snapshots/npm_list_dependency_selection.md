# npm_list_dependency_selection

## `vpt json-edit package.json optionalDependencies.yocto-queue '"0.1.0"'`


## `vp install --ignore-scripts`


## `vp pm list --prod --json`

production listing excludes dev but keeps optional and peer dependencies

```
{
  "version": "1.0.0",
  "name": "command-list-npm10",
  "dependencies": {
    "test-vite-plus-package-optional": {
      "version": "1.0.0",
      "resolved": "https://registry.npmjs.org/test-vite-plus-package-optional/-/test-vite-plus-package-optional-1.0.0.tgz",
      "overridden": false
    },
    "testnpm2": {
      "version": "1.0.1",
      "resolved": "https://registry.npmjs.org/testnpm2/-/testnpm2-1.0.1.tgz",
      "overridden": false
    },
    "yocto-queue": {
      "version": "0.1.0",
      "resolved": "https://registry.npmjs.org/yocto-queue/-/yocto-queue-0.1.0.tgz",
      "overridden": false
    }
  }
}
```

## `vp pm list --prod --no-optional --exclude-peers --json`

production listing respects both exclusions

```
{
  "version": "1.0.0",
  "name": "command-list-npm10",
  "dependencies": {
    "testnpm2": {
      "version": "1.0.1",
      "resolved": "https://registry.npmjs.org/testnpm2/-/testnpm2-1.0.1.tgz",
      "overridden": false
    }
  }
}
```

## `NODE_ENV=production vp pm list --dev --json`

include installed dev dependencies without excluding other dependency types

```
{
  "version": "1.0.0",
  "name": "command-list-npm10",
  "dependencies": {
    "test-vite-plus-package-optional": {
      "version": "1.0.0",
      "resolved": "https://registry.npmjs.org/test-vite-plus-package-optional/-/test-vite-plus-package-optional-1.0.0.tgz",
      "overridden": false
    },
    "test-vite-plus-package": {
      "version": "1.0.0",
      "resolved": "https://registry.npmjs.org/test-vite-plus-package/-/test-vite-plus-package-1.0.0.tgz",
      "overridden": false
    },
    "testnpm2": {
      "version": "1.0.1",
      "resolved": "https://registry.npmjs.org/testnpm2/-/testnpm2-1.0.1.tgz",
      "overridden": false
    },
    "yocto-queue": {
      "version": "0.1.0",
      "resolved": "https://registry.npmjs.org/yocto-queue/-/yocto-queue-0.1.0.tgz",
      "overridden": false
    }
  }
}
```

## `vp pm list -- --include=dev --json`

raw native include remains available and lists all four dependency types

```
{
  "version": "1.0.0",
  "name": "command-list-npm10",
  "dependencies": {
    "test-vite-plus-package-optional": {
      "version": "1.0.0",
      "resolved": "https://registry.npmjs.org/test-vite-plus-package-optional/-/test-vite-plus-package-optional-1.0.0.tgz",
      "overridden": false
    },
    "test-vite-plus-package": {
      "version": "1.0.0",
      "resolved": "https://registry.npmjs.org/test-vite-plus-package/-/test-vite-plus-package-1.0.0.tgz",
      "overridden": false
    },
    "testnpm2": {
      "version": "1.0.1",
      "resolved": "https://registry.npmjs.org/testnpm2/-/testnpm2-1.0.1.tgz",
      "overridden": false
    },
    "yocto-queue": {
      "version": "0.1.0",
      "resolved": "https://registry.npmjs.org/yocto-queue/-/yocto-queue-0.1.0.tgz",
      "overridden": false
    }
  }
}
```

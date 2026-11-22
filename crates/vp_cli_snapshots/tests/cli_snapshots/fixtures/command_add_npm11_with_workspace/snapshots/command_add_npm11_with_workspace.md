# command_add_npm11_with_workspace

## `cd packages/app && vp add testnpm2 -D -w -- --no-audit`

add to the root from a workspace member

```

added 3 packages in <duration>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "command-add-npm11-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "packageManager": "npm@11.6.2",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "app"
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "private": true
}
```

## `vp add @vite-plus-test/utils --workspace -- --no-audit`

reject unsupported --workspace without adding packages to the root

**Exit code:** 1

```
npm does not support --workspace.
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "command-add-npm11-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "packageManager": "npm@11.6.2",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "app"
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "private": true
}
```

## `vp add testnpm2 test-vite-plus-install@1.0.0 --filter app -- --no-audit`

should add packages to packages/app

```

added 1 package in <duration>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "command-add-npm11-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "packageManager": "npm@11.6.2",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "app",
  "dependencies": {
    "test-vite-plus-install": "^1.0.0",
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "private": true
}
```

## `vp add @vite-plus-test/utils --workspace --filter app -- --no-audit`

reject unsupported --workspace without adding packages to app

**Exit code:** 1

```
npm does not support --workspace.
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "command-add-npm11-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "packageManager": "npm@11.6.2",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "app",
  "dependencies": {
    "test-vite-plus-install": "^1.0.0",
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "private": true
}
```

## `vp add testnpm2 test-vite-plus-install@1.0.0 --filter * -- --no-audit`

should add testnpm2 test-vite-plus-install to all packages except workspace root

```

up to date in <duration>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "command-add-npm11-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "packageManager": "npm@11.6.2",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "app",
  "dependencies": {
    "test-vite-plus-install": "^1.0.0",
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "private": true,
  "dependencies": {
    "test-vite-plus-install": "^1.0.0",
    "testnpm2": "^1.0.1"
  }
}
```

## `vp add -E testnpm2 test-vite-plus-install@1.0.0 --filter * --workspace-root -- --no-audit`

should add testnpm2 test-vite-plus-install to all packages include workspace root

```

up to date in <duration>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "command-add-npm11-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "packageManager": "npm@11.6.2",
  "devDependencies": {
    "testnpm2": "1.0.1"
  },
  "dependencies": {
    "test-vite-plus-install": "1.0.0"
  }
}
{
  "name": "app",
  "dependencies": {
    "test-vite-plus-install": "1.0.0",
    "testnpm2": "1.0.1"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "private": true,
  "dependencies": {
    "test-vite-plus-install": "1.0.0",
    "testnpm2": "1.0.1"
  }
}
```

## `vp install test-vite-plus-package@1.0.0 --filter * --workspace-root -- --no-audit`

should install packages alias for add command

```
VITE+ - The Unified Toolchain for the Web

added 1 package in <duration>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "command-add-npm11-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "packageManager": "npm@11.6.2",
  "devDependencies": {
    "testnpm2": "1.0.1"
  },
  "dependencies": {
    "test-vite-plus-install": "1.0.0",
    "test-vite-plus-package": "^1.0.0"
  }
}
{
  "name": "app",
  "dependencies": {
    "test-vite-plus-install": "1.0.0",
    "test-vite-plus-package": "^1.0.0",
    "testnpm2": "1.0.1"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "private": true,
  "dependencies": {
    "test-vite-plus-install": "1.0.0",
    "test-vite-plus-package": "^1.0.0",
    "testnpm2": "1.0.1"
  }
}
```

## `cd packages/app && vp remove -w testnpm2 -- --no-audit`

remove the root devDependency and preserve the member dependencies

```

up to date in <duration>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "command-add-npm11-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "packageManager": "npm@11.6.2",
  "dependencies": {
    "test-vite-plus-install": "1.0.0",
    "test-vite-plus-package": "^1.0.0"
  }
}
{
  "name": "app",
  "dependencies": {
    "test-vite-plus-install": "1.0.0",
    "test-vite-plus-package": "^1.0.0",
    "testnpm2": "1.0.1"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "private": true,
  "dependencies": {
    "test-vite-plus-install": "1.0.0",
    "test-vite-plus-package": "^1.0.0",
    "testnpm2": "1.0.1"
  }
}
```

## `node -e 'const { packages } = require('\''./package-lock.json'\''); console.log(packages['\'''\''].devDependencies?.testnpm2, packages['\''packages/app'\''].dependencies.testnpm2)'`

the lockfile records the same removal at the root and keeps the app dependency

```
undefined 1.0.1
```

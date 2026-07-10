# command_add_yarn4_with_workspace

## `vp add testnpm2 -D -w`

should add package to workspace root

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + testnpm2@npm:1.0.1
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ A package was added to the project (+ <size> KiB).
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "command-add-yarn4-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "packageManager": "yarn@4.10.3",
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

## `vp add @vite-plus-test/utils --workspace -w`

should add @vite-plus-test/utils to workspace root

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "command-add-yarn4-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "packageManager": "yarn@4.10.3",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  },
  "dependencies": {
    "@vite-plus-test/utils": "workspace:^"
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

## `vp add testnpm2 test-vite-plus-install@1.0.0 --filter app`

should add packages to packages/app

```
[app]: Process started
[app]: ➤ YN0000: · Yarn <version>
[app]: ➤ YN0000: ┌ Resolution step
[app]: ➤ YN0085: │ + test-vite-plus-install@npm:1.0.0
[app]: ➤ YN0000: └ Completed
[app]: ➤ YN0000: ┌ Fetch step
[app]: ➤ YN0013: │ A package was added to the project (+ <size> KiB).
[app]: ➤ YN0000: └ Completed
[app]: ➤ YN0000: ┌ Link step
[app]: ➤ YN0000: └ Completed
[app]: ➤ YN0000: · Done in <duration> <duration>
[app]: Process exited (exit code 0), completed in <duration> <duration>

Done in <duration> <duration>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "command-add-yarn4-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "packageManager": "yarn@4.10.3",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  },
  "dependencies": {
    "@vite-plus-test/utils": "workspace:^"
  }
}
{
  "name": "app",
  "dependencies": {
    "test-vite-plus-install": "1.0.0",
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "private": true
}
```

## `vp add @vite-plus-test/utils --workspace --filter app`

should add @vite-plus-test/utils to packages/app

```
[app]: Process started
[app]: ➤ YN0000: · Yarn <version>
[app]: ➤ YN0000: ┌ Resolution step
[app]: ➤ YN0000: └ Completed
[app]: ➤ YN0000: ┌ Fetch step
[app]: ➤ YN0000: └ Completed
[app]: ➤ YN0000: ┌ Link step
[app]: ➤ YN0000: └ Completed
[app]: ➤ YN0000: · Done in <duration> <duration>
[app]: Process exited (exit code 0), completed in <duration> <duration>

Done in <duration> <duration>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "command-add-yarn4-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "packageManager": "yarn@4.10.3",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  },
  "dependencies": {
    "@vite-plus-test/utils": "workspace:^"
  }
}
{
  "name": "app",
  "dependencies": {
    "@vite-plus-test/utils": "workspace:^",
    "test-vite-plus-install": "1.0.0",
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "private": true
}
```

## `vp add testnpm2 test-vite-plus-install@1.0.0 --filter * --filter @vite-plus-test/utils`

should add testnpm2 test-vite-plus-install to all packages and workspace root

```
[command-add-yarn4-with-workspace]: Process started
[command-add-yarn4-with-workspace]: ➤ YN0000: · Yarn <version>
[command-add-yarn4-with-workspace]: ➤ YN0000: ┌ Resolution step
[command-add-yarn4-with-workspace]: ➤ YN0000: └ Completed
[command-add-yarn4-with-workspace]: ➤ YN0000: ┌ Fetch step
[command-add-yarn4-with-workspace]: ➤ YN0000: └ Completed
[command-add-yarn4-with-workspace]: ➤ YN0000: ┌ Link step
[command-add-yarn4-with-workspace]: ➤ YN0000: └ Completed
[command-add-yarn4-with-workspace]: ➤ YN0000: · Done in <duration> <duration>
[command-add-yarn4-with-workspace]: Process exited (exit code 0), completed in <duration> <duration>

[admin]: Process started
[admin]: ➤ YN0000: · Yarn <version>
[admin]: ➤ YN0000: ┌ Resolution step
[admin]: ➤ YN0000: └ Completed
[admin]: ➤ YN0000: ┌ Fetch step
[admin]: ➤ YN0000: └ Completed
[admin]: ➤ YN0000: ┌ Link step
[admin]: ➤ YN0000: └ Completed
[admin]: ➤ YN0000: · Done in <duration> <duration>
[admin]: Process exited (exit code 0), completed in <duration> <duration>

[app]: Process started
[app]: ➤ YN0000: · Yarn <version>
[app]: ➤ YN0000: ┌ Resolution step
[app]: ➤ YN0000: └ Completed
[app]: ➤ YN0000: ┌ Fetch step
[app]: ➤ YN0000: └ Completed
[app]: ➤ YN0000: ┌ Link step
[app]: ➤ YN0000: └ Completed
[app]: ➤ YN0000: · Done in <duration> <duration>
[app]: Process exited (exit code 0), completed in <duration> <duration>

[@vite-plus-test/utils]: Process started
[@vite-plus-test/utils]: ➤ YN0000: · Yarn <version>
[@vite-plus-test/utils]: ➤ YN0000: ┌ Resolution step
[@vite-plus-test/utils]: ➤ YN0000: └ Completed
[@vite-plus-test/utils]: ➤ YN0000: ┌ Fetch step
[@vite-plus-test/utils]: ➤ YN0000: └ Completed
[@vite-plus-test/utils]: ➤ YN0000: ┌ Link step
[@vite-plus-test/utils]: ➤ YN0000: └ Completed
[@vite-plus-test/utils]: ➤ YN0000: · Done in <duration> <duration>
[@vite-plus-test/utils]: Process exited (exit code 0), completed in <duration> <duration>

Done in <duration> <duration>
```

## `vpt print-file package.json packages/app/package.json packages/admin/package.json packages/utils/package.json`

```
{
  "name": "command-add-yarn4-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "packageManager": "yarn@4.10.3",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  },
  "dependencies": {
    "@vite-plus-test/utils": "workspace:^",
    "test-vite-plus-install": "1.0.0"
  }
}
{
  "name": "app",
  "dependencies": {
    "@vite-plus-test/utils": "workspace:^",
    "test-vite-plus-install": "1.0.0",
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "admin",
  "dependencies": {
    "test-vite-plus-install": "1.0.0",
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "private": true,
  "dependencies": {
    "test-vite-plus-install": "1.0.0",
    "testnpm2": "^1.0.1"
  }
}
```

## `vp install -O test-vite-plus-package-optional --filter *`

should install packages alias for add command

```
VITE+ - The Unified Toolchain for the Web

[command-add-yarn4-with-workspace]: Process started
[command-add-yarn4-with-workspace]: ➤ YN0000: · Yarn <version>
[command-add-yarn4-with-workspace]: ➤ YN0000: ┌ Resolution step
[command-add-yarn4-with-workspace]: ➤ YN0085: │ + test-vite-plus-package-optional@npm:1.0.0
[command-add-yarn4-with-workspace]: ➤ YN0000: └ Completed
[command-add-yarn4-with-workspace]: ➤ YN0000: ┌ Fetch step
[command-add-yarn4-with-workspace]: ➤ YN0013: │ A package was added to the project (+ <size> KiB).
[command-add-yarn4-with-workspace]: ➤ YN0000: └ Completed
[command-add-yarn4-with-workspace]: ➤ YN0000: ┌ Link step
[command-add-yarn4-with-workspace]: ➤ YN0000: └ Completed
[command-add-yarn4-with-workspace]: ➤ YN0000: · Done in <duration> <duration>
[command-add-yarn4-with-workspace]: Process exited (exit code 0), completed in <duration> <duration>

[admin]: Process started
[admin]: ➤ YN0000: · Yarn <version>
[admin]: ➤ YN0000: ┌ Resolution step
[admin]: ➤ YN0000: └ Completed
[admin]: ➤ YN0000: ┌ Fetch step
[admin]: ➤ YN0000: └ Completed
[admin]: ➤ YN0000: ┌ Link step
[admin]: ➤ YN0000: └ Completed
[admin]: ➤ YN0000: · Done in <duration> <duration>
[admin]: Process exited (exit code 0), completed in <duration> <duration>

[app]: Process started
[app]: ➤ YN0000: · Yarn <version>
[app]: ➤ YN0000: ┌ Resolution step
[app]: ➤ YN0000: └ Completed
[app]: ➤ YN0000: ┌ Fetch step
[app]: ➤ YN0000: └ Completed
[app]: ➤ YN0000: ┌ Link step
[app]: ➤ YN0000: └ Completed
[app]: ➤ YN0000: · Done in <duration> <duration>
[app]: Process exited (exit code 0), completed in <duration> <duration>

Done in <duration> <duration>
```

## `vpt print-file package.json packages/app/package.json packages/admin/package.json packages/utils/package.json`

```
{
  "name": "command-add-yarn4-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "packageManager": "yarn@4.10.3",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  },
  "dependencies": {
    "@vite-plus-test/utils": "workspace:^",
    "test-vite-plus-install": "1.0.0"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "^1.0.0"
  }
}
{
  "name": "app",
  "dependencies": {
    "@vite-plus-test/utils": "workspace:^",
    "test-vite-plus-install": "1.0.0",
    "testnpm2": "^1.0.1"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "^1.0.0"
  }
}
{
  "name": "admin",
  "dependencies": {
    "test-vite-plus-install": "1.0.0",
    "testnpm2": "^1.0.1"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "^1.0.0"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "private": true,
  "dependencies": {
    "test-vite-plus-install": "1.0.0",
    "testnpm2": "^1.0.1"
  }
}
```

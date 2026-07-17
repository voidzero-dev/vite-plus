# command_update_npm10

## `vp update testnpm2 -- --no-audit`

should update package within semver range

```

added 3 packages in <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-update-npm10",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "*"
  },
  "devDependencies": {
    "test-vite-plus-package": "*"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "*"
  },
  "packageManager": "npm@10.9.2"
}
```

## `vp up testnpm2 --latest -- --no-audit`

should to absolute latest version

```
warn: npm doesn't support --latest flag. Updating within semver range only.

up to date in <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-update-npm10",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "*"
  },
  "devDependencies": {
    "test-vite-plus-package": "*"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "*"
  },
  "packageManager": "npm@10.9.2"
}
```

## `vp update -D -- --no-audit`

should update only dev dependencies

```

up to date in <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-update-npm10",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "*"
  },
  "devDependencies": {
    "test-vite-plus-package": "*"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "*"
  },
  "packageManager": "npm@10.9.2"
}
```

## `vp update -P --no-save -- --no-audit`

should update only dependencies and optionalDependencies without saving

```

up to date in <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-update-npm10",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "*"
  },
  "devDependencies": {
    "test-vite-plus-package": "*"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "*"
  },
  "packageManager": "npm@10.9.2"
}
```

## `vp rm testnpm2`

should skip optional dependencies

```

removed 1 package, and audited 3 packages in <duration>

found 0 vulnerabilities
```

## `vp add testnpm2@1.0.0 -O -- --no-audit`

```

added 1 package in <duration>
```

## `vp update --no-optional --latest -- --no-audit`

```
warn: npm doesn't support --latest flag. Updating within semver range only.
npm warn config optional Use `--omit=optional` to exclude optional dependencies, or
npm warn config `--include=optional` to include them.
npm warn config
npm warn config       Default value does install optional deps unless otherwise omitted.

changed 1 package in <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-update-npm10",
  "version": "1.0.0",
  "devDependencies": {
    "test-vite-plus-package": "*"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "*",
    "testnpm2": "^1.0.0"
  },
  "packageManager": "npm@10.9.2"
}
```

## `vp update -- --no-audit`

should update all packages but won't change the package.json

```

added 2 packages in <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-update-npm10",
  "version": "1.0.0",
  "devDependencies": {
    "test-vite-plus-package": "*"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "*",
    "testnpm2": "^1.0.0"
  },
  "packageManager": "npm@10.9.2"
}
```

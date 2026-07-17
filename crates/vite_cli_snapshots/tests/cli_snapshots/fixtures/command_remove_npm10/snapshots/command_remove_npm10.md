# command_remove_npm10

## `vp remove testnpm2 -D -- --no-audit`

should pass when remove not exists package

```

up to date in <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-remove-npm10",
  "version": "1.0.0",
  "packageManager": "npm@10.9.4"
}
```

## `vp add testnpm2 -- --no-audit`

should add packages to dependencies

```

added 1 package in <duration>
```

## `vp add -D test-vite-plus-install -- --no-audit`

```

added 1 package in <duration>
```

## `vp add -O test-vite-plus-package-optional -- --no-audit`

```

added 1 package in <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-remove-npm10",
  "version": "1.0.0",
  "packageManager": "npm@10.9.4",
  "dependencies": {
    "testnpm2": "^1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-install": "^1.0.0"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "^1.0.0"
  }
}
```

## `vp remove testnpm2 test-vite-plus-install -- --no-audit`

should remove packages from dependencies

```

removed 2 packages in <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-remove-npm10",
  "version": "1.0.0",
  "packageManager": "npm@10.9.4",
  "optionalDependencies": {
    "test-vite-plus-package-optional": "^1.0.0"
  }
}
```

## `vp remove -D test-vite-plus-package-optional -- --loglevel=warn --no-audit`

support ignore -O flag and remove package from optional dependencies

```

removed 1 package in <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-remove-npm10",
  "version": "1.0.0",
  "packageManager": "npm@10.9.4"
}
```

## `vp remove -g --dry-run testnpm2`

support remove global package with dry-run

**Exit code:** 1

```
Failed to uninstall testnpm2: Package testnpm2 is not installed
```

*(skipped 1 step(s) to the next line boundary: step failed)*

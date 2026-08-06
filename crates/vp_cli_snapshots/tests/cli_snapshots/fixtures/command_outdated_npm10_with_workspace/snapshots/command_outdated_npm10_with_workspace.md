# command_outdated_npm10_with_workspace

## `vp install`

```
VITE+ - The Unified Toolchain for the Web

added 6 packages, and audited 9 packages in <duration>

found 0 vulnerabilities
```

## `vp outdated testnpm2 -w`

should outdated in workspace root

**Exit code:** 1

```
Package   Current  Wanted  Latest  Location               Depended by
testnpm2    1.0.0   1.0.0   1.0.1  node_modules/testnpm2  workspace
```

## `vp outdated testnpm2 --filter app`

should outdated in specific package

```
```

## `vp outdated --filter * --format json`

should outdated in all packages

**Exit code:** 1

```
{
  "test-vite-plus-other-optional": {
    "current": "1.0.0",
    "wanted": "1.0.0",
    "latest": "1.1.0",
    "dependent": "app",
    "location": "<workspace>/node_modules/test-vite-plus-other-optional"
  }
}
```

## `vp outdated -r`

should outdated recursively

**Exit code:** 1

```
Package                        Current  Wanted  Latest  Location                                    Depended by
test-vite-plus-other-optional    1.0.0   1.0.0   1.1.0  node_modules/test-vite-plus-other-optional  app@
testnpm2                         1.0.0   1.0.0   1.0.1  node_modules/testnpm2                       workspace
```

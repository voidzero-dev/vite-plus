# command_registry_pnpm11

## `vp pm login --scope @company -- --help`

should reject scope on pnpm before native support was added

**Exit code:** 1

```
pnpm >= 11 && < 11.2 does not support --scope.
```

## `vp pm dist-tag list`

should preserve the native missing-package error

**Exit code:** 1

```
[ERR_PNPM_DIST_TAG_LS_PACKAGE_REQUIRED] Package name is required
```

## `vp pm deprecate command-registry-pnpm11@1.0.0 `

should preserve the native empty-message error without undeprecating

**Exit code:** 1

```
[ERR_PNPM_DEPRECATE_MESSAGE_REQUIRED] Deprecation message is required. To un-deprecate, use the undeprecate command.
```

## `vp pm search react --long`

should reject long before invoking native pnpm search

**Exit code:** 1

```
pnpm >= 11 does not support --long.
```

## `vp pm logout --scope @company -- --help`

should reject scope before invoking native pnpm logout

**Exit code:** 1

```
pnpm >= 11 does not support --scope.
```

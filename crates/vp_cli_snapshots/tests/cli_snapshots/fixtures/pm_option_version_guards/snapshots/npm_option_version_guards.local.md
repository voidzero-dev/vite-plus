# npm_option_version_guards

## `vp pm publish --dry-run`

Reject preview before an npm version that would publish despite dry-run. The fixture is also private for safety.

```
📦 pm-option-version-guards@1.0.0 → http://127.0.0.1:<port>/
[WARN] Skip publishing pm-option-version-guards@1.0.0 (dry run)
```

## `vp pm publish --provenance`

**Exit code:** 1

```
📦 pm-option-version-guards@1.0.0 → http://127.0.0.1:<port>/
Error: ERR_PNPM_PRIVATE_PACKAGE

  x This package has been marked as private
  help: Remove the 'private' field from the package.json to publish it.
```

## `vp pm pack --filter app`

```
```

## `vp pm pack --pack-destination dist`

```
package: pm-option-version-guards@1.0.0
Tarball Contents
package.json
Tarball Details
<workspace>/dist/pm-option-version-guards-1.0.0.tgz
```

## `vp install --filter app`

```
Scope: 0 of 1 workspace projects

Done in <duration> using pnpm <version>
```

## `vp install --workspace-root`

**Exit code:** 1

```
Error: ERR_PNPM_NOT_IN_WORKSPACE

  x --workspace-root may only be used inside a workspace
```

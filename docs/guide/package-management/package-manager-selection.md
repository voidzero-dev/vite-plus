# Package Manager Selection

`vp` detects the package manager at the workspace root in this order:

1. `packageManager` in `package.json`
2. `devEngines.packageManager` in `package.json`
3. `pnpm-workspace.yaml`
4. `pnpm-lock.yaml`
5. `yarn.lock` or `.yarnrc.yml`
6. `package-lock.json`
7. `bun.lock` or `bun.lockb`
8. `.pnpmfile.cjs` or `pnpmfile.cjs`
9. `bunfig.toml`
10. `yarn.config.cjs`

If none of these declarations or files is present, Vite+ defaults to pnpm. Vite+ downloads the matching package manager as needed. Detection does not rewrite `package.json`.

Yarn Classic is Yarn 1. Yarn Modern is Yarn 2 or later.

## Pinning a Version

Use [`vp env pin`](/guide/env) to declare an exact package-manager version:

```bash
vp env pin pnpm@11.3.0
```

A `packageManager` pin can include an integrity hash, such as `yarn@4.17.1+sha512.…`. Vite+ supports Corepack-compatible integrity hashes and stops if verification fails.

## Version Ranges

The [`devEngines.packageManager`](https://docs.npmjs.com/cli/v11/configuring-npm/package-json#devengines) field accepts an object or an array of objects. Its `version` can be a semver range:

```json
{
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "^11.0.0",
      "onFail": "download"
    }
  }
}
```

Vite+ uses an already-downloaded satisfying version when possible. Otherwise, it resolves the latest satisfying version from the npm registry. The range remains the source of truth and is not replaced with an exact `packageManager` pin.

When both fields are present, `packageManager` drives selection. Vite+ warns if it does not satisfy the `devEngines.packageManager` constraint. Run `vp env doctor` for details.

## Package-Manager Shims

An explicit `packageManager` or `devEngines.packageManager` declaration also controls matching package-manager shims. For example, `packageManager: "npm@10.9.4"` makes `npm` and `npx` use npm 10.9.4.

The same applies to the generated aliases for each manager:

- pnpm: `pnpm`, `pnpx`, `pn`, and `pnx`.
- Yarn: `yarn` and `yarnpkg`.
- Bun: `bun` and `bunx`.

Mismatched tools are not translated. Running `npm` in a pnpm project still resolves npm. Use `vp` package-management commands when you want Vite+ to adapt the operation to the project's manager.

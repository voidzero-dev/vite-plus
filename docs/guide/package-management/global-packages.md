# Global Packages

Global package operations require the [global CLI](/guide/global-cli). Use `-g` to manage tools independently of a project's dependencies:

```bash
vp install -g typescript
vp install -g --node 22 typescript
vp uninstall -g typescript
vp update -g
vp list -g
vp outdated -g
```

## Storage and Node.js

Vite+ stores global packages in `packages/` under its resolved data directory, not in the underlying package manager's global installation directory.

Without `--node`, a global install uses the Node.js version resolved for the current directory. A project's pin therefore takes precedence over `vp env default`. See [Node.js Selection](/guide/env#node-js-selection).

`vp link` uses native package-manager links. It does not modify Vite+'s global package store, and those links do not appear in `vp list -g`.

## Updating Packages

Updates preserve the version specification recorded at installation. A package installed with `some-pkg@nightly` updates to the newest version of that tag. A version range stays within the range.

```bash
vp update -g some-pkg
vp update -g some-pkg@latest
vp update -g --latest
```

With --latest, future updates follow the latest tag instead of the original tag or version range.

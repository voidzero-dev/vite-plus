# Fixed library template

The `new_vite_monorepo` and `new_vite_monorepo_bun` cases check project creation
and catalog migration. Their registry overlays replace `degit` with a local
template copy. The wrapper checks the requested repository and arguments.
The real package managers and Vite+ commands still run. Other cases retain
the real `degit` package.

`template/` contains the `vite-plus/` template from
[`sxzz/tsdown-templates` at `01a79e74`](https://github.com/sxzz/tsdown-templates/tree/01a79e7473ae2cb90af2b1312decc54c14b7cb91/vite-plus).
`LICENSE` preserves its MIT license. Test runs never fetch or refresh this copy.

After an intentional fixture change, regenerate both case tarballs and their
integrity hashes from the repository root:

```sh
node crates/vp_cli_snapshots/tests/cli_snapshots/fixtures/new_vite_monorepo/degit-fixture/pack.mjs
```

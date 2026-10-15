# Fixed library template

The `new_vite_monorepo` and `new_vite_monorepo_bun` cases check project creation
and catalog migration. Their registry overlays replace `degit` with a local
template copy. The wrapper checks the requested repository and arguments.
The real package managers and Vite+ commands still run. Other cases retain
the real `degit` package.

`template/` contains the `vite-plus/` template from
[`sxzz/tsdown-templates` at `c1d30b48`](https://github.com/sxzz/tsdown-templates/tree/c1d30b48c445cc73d2da92be1db6f7c27cc0110c/vite-plus).
`LICENSE` preserves its MIT license. Test runs never fetch or refresh this copy.

After an intentional fixture change, regenerate both case tarballs and their
integrity hashes from the repository root:

```sh
node crates/vp_cli_snapshots/tests/cli_snapshots/fixtures/new_vite_monorepo/degit-fixture/pack.mjs
```

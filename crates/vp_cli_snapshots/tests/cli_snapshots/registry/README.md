# Registry fixtures

`manifest.json` records selected published npm versions for small test packages
used by the package-manager snapshots. Their tarballs are
the original published bytes, with their original integrity hashes. Only
`dist.tarball` URLs change to `{REGISTRY}` placeholders when recording data.
Metadata omits unused versions, their timestamps, and tags that refer to them.
No package-manager or toolchain implementation is mocked.

`packages/` contains two dependency-free CLI packages authored for the tests.
`vp-test-echo` prints arguments or standard input; `vp-test-upper` converts
standard input to uppercase. They replace unrelated public CLI dependencies in
the `dlx`, `vpx`, global binary lookup, and native pnpm shim cases. Package
managers still download, install, and execute real tarballs. The multiple-package
case runs both binaries through a pipe.
The echo package also exposes an alias to preserve coverage for packages with
multiple binaries. CommonJS entry points work with Yarn PnP without an ESM loader.

Regenerate `local/tarballs/` and `local/manifest.json` after editing their source:

```sh
node crates/vp_cli_snapshots/tests/cli_snapshots/registry/pack.mjs
```

Packing uses Node.js builtins with fixed timestamps and file modes. It needs no
network access and does not run package scripts. The npm refresh command below
does not change these local packages.
The `live` and `direct` comparison modes also serve authored packages locally,
because they have no public-registry counterpart.

The recorded `is-number` and `yocto-queue` packages supply the unchanged inputs
for update, list, remove, and patch cases. These tests exercise real package
contents and version ranges, so the fixtures keep the original tarball bytes.
Only the versions needed by these cases and their dependencies are recorded:

| Package       | Versions                  | Coverage                                                                            |
| ------------- | ------------------------- | ----------------------------------------------------------------------------------- |
| `is-number`   | `6.0.0`, `7.0.0`          | Initial installation, update target, and patching                                   |
| `yocto-queue` | `0.1.0`, `1.0.0`, `1.2.2` | Initial installation, in-range update, and a newer version outside the update range |

The registry serves these fixtures before forwarding requests upstream.
A case's `mock-manifest.json` and `tarballs/` take precedence over shared data.
Locally packed checkout packages keep precedence over both.

To refresh these packages deliberately, from the repository root:

```sh
node packages/tools/src/record-snapshot-registry.ts
```

The command fetches metadata and tarballs from `https://registry.npmjs.org`,
checks each tarball's SHA-512 integrity, and updates the recorded files.
`selectedVersions` in the recorder limits these two packages to the versions
above. The command removes tarballs for versions dropped from the manifest.
If the published `latest` tag changes, update the selection explicitly before
refreshing so it still covers the required ranges.
Review the changes and re-record affected snapshots. Test runs never refresh
these files. Pass package names to add or refresh only those packages:

```sh
node packages/tools/src/record-snapshot-registry.ts is-number yocto-queue
```

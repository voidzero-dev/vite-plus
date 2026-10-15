# Registry fixtures

These are the six small test packages already used by the package-manager
snapshots. `manifest.json` records their published npm metadata. Tarballs are
the original published bytes, with their original integrity hashes. Only
`dist.tarball` URLs change to `{REGISTRY}` placeholders when recording data.
No package-manager or toolchain implementation is mocked.

The registry serves these fixtures before forwarding requests upstream.
A case's `mock-manifest.json` and `tarballs/` take precedence over shared data.
Locally packed checkout packages keep precedence over both.

To refresh these packages deliberately, from the repository root:

```sh
node packages/tools/src/record-snapshot-registry.ts
```

The command fetches metadata and tarballs from `https://registry.npmjs.org`,
checks each tarball's SHA-512 integrity, and updates the recorded files.
Review the changes and re-record affected snapshots. Test runs never refresh
these files. To add another small test package, first add its name as a key
in `manifest.json`, then run the command.

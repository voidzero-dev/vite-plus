# Tarball regeneration

`create-1.0.0.tgz` is committed binary. The mock manifest's
`integrity: "sha512-…"` field locks the bytes — regenerating without
matching the original bit-for-bit will break the snap-test.

To regenerate (e.g. after editing one of the bundled template files):

```sh
mkdir -p /tmp/vp-bundled-monorepo-build/package/templates/workspace
# … recreate the template files inside that workspace dir …

cd /tmp/vp-bundled-monorepo-build
COPYFILE_DISABLE=1 tar --no-mac-metadata -czf \
  <repo>/packages/cli/snap-tests/create-org-bundled-monorepo/tarballs/create-1.0.0.tgz \
  package/

# Then update mock-manifest.json's `integrity` field:
node -e "
  const fs = require('node:fs');
  const crypto = require('node:crypto');
  const buf = fs.readFileSync(
    '<repo>/packages/cli/snap-tests/create-org-bundled-monorepo/tarballs/create-1.0.0.tgz',
  );
  console.log('sha512-' + crypto.createHash('sha512').update(buf).digest('base64'));
"
```

`COPYFILE_DISABLE=1` and `--no-mac-metadata` keep macOS from injecting
`._*` AppleDouble files that pollute the extracted output and break the
snap-test.

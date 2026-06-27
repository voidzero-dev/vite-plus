<!-- pkg-pr-bridge-version -->
### Registry bridge build (`__SHORT__`)

This commit is published to pkg.pr.new and registered with the [registry bridge](https://github.com/fengmk2/pkg-pr-registry-bridge), which serves these as ordinary npm versions (every other package proxies to npmjs):

| Package | Version |
| --- | --- |
| `vite-plus` | `0.0.0-commit.__SHA__` |
| `@voidzero-dev/vite-plus-core` | `0.0.0-commit.__SHA__` |

**Point your package manager at the bridge registry** `https://pkg-pr-registry-bridge.render.vip/`:

| Package manager | Registry config |
| --- | --- |
| npm / pnpm / Bun | `.npmrc`: `registry=https://pkg-pr-registry-bridge.render.vip/` |
| Yarn (v2+) | `.yarnrc.yml`: `npmRegistryServer: "https://pkg-pr-registry-bridge.render.vip/"` |

Then pin the build (`vite` aliases to vite-plus-core; pnpm can use a catalog, npm an `overrides` entry):

```json
{
  "devDependencies": {
    "vite-plus": "0.0.0-commit.__SHA__",
    "vite": "npm:@voidzero-dev/vite-plus-core@0.0.0-commit.__SHA__"
  }
}
```

# migration_standalone_bun_install

## `vp migrate --no-interactive --no-hooks`

standalone (non-workspace) bun upgrade must keep concrete specs so `bun install` resolves instead of failing with `vite@catalog: failed to resolve`

```
VITE+ - The Unified Toolchain for the Web

Formatting code...

Code formatted
◇ Updated . to Vite+ <version>
• Node <version>  bun <version>
• Dependencies:
    vite-plus  0.1.24 → <version>
    vite              → <version>
✓ Dependencies installed in <duration>
• Package manager settings configured
```

## `vpt print-file package.json`

vite/vite-plus stay concrete (vite via the @voidzero-dev/vite-plus-core alias); NO top-level catalog field is written

```
{
  "name": "migration-standalone-bun-install",
  "private": true,
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vite-plus": "<version>"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  },
  "devEngines": {
    "packageManager": {
      "name": "bun",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

# migration_downgrades_newer_toolchain_pnpm

Reproduce #2662: an existing Vite+ project has Vite and Vitest versions newer than the running CLI's bundled versions.

## `vpt cp package.json package.before.json`


## `vpt write-file node_modules/vite/package.json '{"name":"vite","version":"8.2.3"}'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

migration reports success without warning that the toolchain was downgraded

```
VITE+ - The Unified Toolchain for the Web

Formatting code...

Code formatted
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite                        8.2.3  → <version>
    vitest                      4.1.12 → <version>
    @vitest/browser             4.1.12 → <version>
    @vitest/browser-playwright  4.1.12 → <version>
✓ Dependencies installed in <duration>
• Package manager settings configured
```

## `vpt print-file package.json`

managed dependencies now resolve through the rewritten catalog

```
{
  "name": "migration-downgrades-newer-toolchain-pnpm",
  "private": true,
  "devDependencies": {
    "@vitest/browser-playwright": "catalog:",
    "playwright": "*",
    "vite": "catalog:",
    "vite-plus": "catalog:",
    "vitest": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vpt print-file pnpm-workspace.yaml`

the catalog records the running CLI's toolchain pins

```
packages:
  - .
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vitest: <version>
  vite-plus: <version>
  "@vitest/browser-playwright": <version>
overrides:
  vite@*: "catalog:"
  vitest@*: "catalog:"
peerDependencyRules:
  allowAny:
    - vite
    - vitest
  allowedVersions:
    vite: "*"
    vitest: "*"
```

## `node check-downgrade.mjs`

assert the installed versions are lower even though snapshots redact the bundled versions

```
vite: downgraded to the running CLI's bundled version
vitest: downgraded to the running CLI's bundled version
@vitest/browser: downgraded to the running CLI's bundled version
@vitest/browser-playwright: downgraded to the running CLI's bundled version
```

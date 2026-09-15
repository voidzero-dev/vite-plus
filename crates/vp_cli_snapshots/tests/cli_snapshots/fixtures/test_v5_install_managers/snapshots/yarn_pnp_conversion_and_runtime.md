# yarn_pnp_conversion_and_runtime

Vite+ does not support PnP; validate the documented conversion to node-modules before executing the v5 runner.

## `vpt json-edit package.json packageManager yarn@4.12.0`


## `vpt write-file .yarnrc.yml 'nodeLinker: pnp
'`


## `vp install`


## `vpt stat-file .pnp.cjs --assert file`

```
.pnp.cjs: file
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

⚠ Vite+ does not currently support Yarn Plug'n'Play (PnP).

✔ Switched Yarn to node-modules mode

Formatting code...

Code formatted
◇ Updated . to Vite+ <version>
• Node <version>  yarn <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
✓ Dependencies installed in <duration>
• 1 file had imports rewritten
• Package manager settings configured
```

## `vpt print-file .yarnrc.yml`

```
nodeLinker: node-modules
npmPreapprovedPackages:
  - vitest
  - "@vitest/*"
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vitest: <version>
  vite-plus: <version>
  "@vitest/browser-webdriverio": ^5.0.0-beta.5 || >=5.0.0
  "@vitest/browser-playwright": <version>
```

## `vp test run`

```
VITE+ - The Unified Toolchain for the Web

 RUN  <version> <workspace>

 ✓ identity.test.js (1 test) <duration>
   ✓ installed runner shares custom matchers and mock state <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (<timing>)
```

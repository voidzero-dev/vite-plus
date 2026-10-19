# rolldown_webcontainer_fallback

## `npm install --ignore-scripts --legacy-peer-deps --no-audit --no-fund`


## `node --experimental-vm-modules --disable-warning=ExperimentalWarning verify.mjs missing`

Without a standalone rolldown package, download the WASM binding at the bundled Rolldown version.

```
Standalone rolldown: missing
WASM cache: missing
[rolldown] Downloading @rolldown/binding-wasm32-wasi@<bundled-rolldown-version> on WebContainer...
Installer selected the bundled Rolldown version
Loaded the WebContainer WASM binding
Downloads: 1
```

## `node --experimental-vm-modules --disable-warning=ExperimentalWarning verify.mjs cached`

Load an existing WASM binding without a package lookup or another download.

```
Standalone rolldown: missing
WASM cache: cached
Loaded the WebContainer WASM binding
Downloads: 0
```

# local_registry_smoke

Smoke test for `local-registry = true`: a package that exists ONLY in this fixture's mock-manifest/tarballs resolves through the injected registry env, proving the runner packs the checkout, starts the per-case registry, and folds its env into every step.

## `npm install @vp-smoke/hello --no-save --no-audit --no-fund`

install a package served only by the local registry


## `vpt print-file node_modules/@vp-smoke/hello/package.json`

the local registry served the packument and its tarball (integrity verified by npm)

```
{
  "name": "@vp-smoke/hello",
  "version": "1.0.0",
  "description": "Dependency-free package served by the local registry smoke test.",
  "main": "index.js",
  "license": "MIT"
}
```

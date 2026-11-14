# core_peer_dependency_checks_pnpm_strict

The core version encodes the bundled Vite version, so strict pnpm peer checks accept a Vite 8 plugin through the vite alias and still reject a Vite 7 plugin.

## `vpt write-file pnpm-workspace.yaml 'strictPeerDependencies: true
minimumReleaseAge: 0
'`


## `vp install --ignore-scripts`


## `node check-peers.mjs`

```
pnpm strict peer check accepts a plugin that supports the bundled Vite
pnpm strict peer check rejects a plugin that requires Vite 7
```

# core_peer_dependency_checks_pnpm_strict_override

The same strict pnpm peer checks in the layout that vp migrate writes, with the vite@* alias override and no peer rule.

## `vpt write-file pnpm-workspace.yaml 'strictPeerDependencies: true
minimumReleaseAge: 0
overrides:
  vite@*: npm:@voidzero-dev/vite-plus-core@latest
'`


## `vp install --ignore-scripts`


## `node check-peers.mjs`

```
pnpm strict peer check accepts a plugin that supports the bundled Vite
pnpm strict peer check rejects a plugin that requires Vite 7
```

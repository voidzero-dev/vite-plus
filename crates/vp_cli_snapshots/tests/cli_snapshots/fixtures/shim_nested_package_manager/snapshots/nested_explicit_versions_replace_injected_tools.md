# nested_explicit_versions_replace_injected_tools

## `vp env on pnpm`


## `npx --offline --call 'pnpm exec vp env exec --node 22.18.0 --package-manager pnpm@10.18.0 node assert-injected-tools.cjs pnpm 10.18.0 22.18.0'`

Explicit versions replace the inherited runtime and package manager


## `npx --offline --call 'pnpm exec vp env exec --node 22.18.0 --package-manager pnpm@10.18.0 node assert-injected-tools.cjs pnpm 10.18.0 22.18.0'`

Re-entering a shim keeps the explicitly selected versions

```
Injected pnpm resolves to 10.18.0 on Node <version>
```

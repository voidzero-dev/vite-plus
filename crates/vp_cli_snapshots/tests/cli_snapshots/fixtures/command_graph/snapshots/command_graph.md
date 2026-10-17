# command_graph

Lists workspace packages with their internal dependencies; external packages are omitted.

## `vp graph`

```
monorepo (.)
@acme/web (apps/web)
  @acme/ui
  @acme/utils (dev)
@acme/ui (packages/ui)
  @acme/utils
@acme/utils (packages/utils)
```

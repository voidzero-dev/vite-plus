# builtin_tools_without_node_on_path

## `node assert-runtime.cjs`

Built-in tools reuse the current runtime even when its filename is not node and PATH has no node executable.

```
lint reused the current runtime without node on PATH
fmt reused the current runtime without node on PATH
```

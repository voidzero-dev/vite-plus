# vpx_script_node26

On Node.js 26 the loader uses in-thread module.registerHooks(), without the DEP0205 warning module.register() prints.

## `vpx ./main.ts`

```
registerHooks cts counter=1 commonjs .cts true
```

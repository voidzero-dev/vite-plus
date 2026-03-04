# vite_static_config

Statically extracts configuration from `vite.config.*` files without executing JavaScript.

## What it does

Parses vite config files using [oxc_parser](https://crates.io/crates/oxc_parser) and extracts
top-level fields whose values are pure JSON literals. This allows reading config like `run`
without needing a Node.js runtime (NAPI).

## Supported patterns

**ESM:**
```js
export default { run: { tasks: { build: { command: "echo build" } } } }
export default defineConfig({ run: { cacheScripts: true } })
```

**CJS:**
```js
module.exports = { run: { tasks: { build: { command: "echo build" } } } }
module.exports = defineConfig({ run: { cacheScripts: true } })
```

## Config file resolution

Searches for config files in the same order as Vite's
[`DEFAULT_CONFIG_FILES`](https://github.com/vitejs/vite/blob/25227bbdc7de0ed07cf7bdc9a1a733e3a9a132bc/packages/vite/src/node/constants.ts#L98-L105):

1. `vite.config.js`
2. `vite.config.mjs`
3. `vite.config.ts`
4. `vite.config.cjs`
5. `vite.config.mts`
6. `vite.config.cts`

## Return type

`resolve_static_config` returns `Option<FxHashMap<Box<str>, StaticFieldValue>>`:

- **`None`** — config is not statically analyzable (no config file, parse error, no
  `export default`/`module.exports`, or the exported value is not an object literal).
  Caller should fall back to runtime evaluation (e.g. NAPI).
- **`Some(map)`** — config object was successfully located:
  - `StaticFieldValue::Json(value)` — field value extracted as pure JSON
  - `StaticFieldValue::NonStatic` — field exists but contains non-JSON expressions
    (function calls, variables, template literals with interpolation, etc.)
  - Key absent — field does not exist in the config object

## Limitations

- Only extracts values that are pure JSON literals (strings, numbers, booleans, null,
  arrays, and objects composed of these)
- Fields with dynamic values (function calls, variable references, spread operators,
  computed properties, template literals with expressions) are reported as `NonStatic`
- Does not follow imports or evaluate expressions

# check_fix_dangerously_config_no_lint

## `vpt write-file vite.config.ts 'export default { check: { lint: false }, lint: { rules: { '\''no-unused-vars'\'': '\''error'\'' } } };
'`

```
```

## `vp check --fix-dangerously src/index.js`

```
note: Lint skipped (check.lint: false in vite.config.ts)
pass: Formatting completed for checked files (<duration>)
```

## `vpt print-file src/index.js`

```
const unused = 1;
export function greet(enabled) {
  if (enabled) return "hello";
  return "world";
}
```

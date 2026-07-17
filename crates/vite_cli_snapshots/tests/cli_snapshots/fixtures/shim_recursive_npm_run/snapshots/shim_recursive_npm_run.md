# shim_recursive_npm_run

## `npm run outer`

Outer script calls npm run inner recursively

```

> shim-recursive-npm-run@1.0.0 outer
> npm run inner

> shim-recursive-npm-run@1.0.0 inner
> echo hello from inner

hello from inner
```

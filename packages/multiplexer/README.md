# multiplexer

Dubbed `multiplexer` for now

Example using [template project](../global/template):

```sh
mkdir my-new-project
vpg new
vpl task test#packages/lib dev#packages/app 'exec#pnpm run -F @my-vite-plus-monorepo/lib lint' test#packages/lib
```

Requires [verdaccio](../global/verdaccio.md) with [vite-plus](../cli) published.

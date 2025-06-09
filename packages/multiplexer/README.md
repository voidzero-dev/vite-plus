# multiplexer

Dubbed `multiplexer` for now

Example using [template project](../global/template):

```sh
mkdir my-new-project
vpg new
vpg task test#packages/lib dev#packages/app 'exec#pnpm run -F @my-vite-plus-monorepo/lib lint' test#packages/lib
```

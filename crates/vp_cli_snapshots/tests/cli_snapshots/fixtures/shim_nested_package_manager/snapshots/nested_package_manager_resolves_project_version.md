# nested_package_manager_resolves_project_version

## `vp env on pnpm`


## `npx --offline --call 'pnpm --version'`

A nested shim resolves and installs the pinned pnpm on first use


## `npx --offline --call 'pnpm --version'`

npx can invoke the project pnpm without a system pnpm

```
10.19.0
```

## `npm run --silent probe`

npm lifecycle scripts can invoke the project pnpm

```
10.19.0
```

## `npx --offline --call 'pnpm exec node assert-injected-tools.cjs pnpm 10.19.0 20.18.0'`

Injected tools accumulate and an explicit shim invocation reuses them

```
Injected pnpm resolves to 10.19.0 on Node <version>
```

## `pnpm exec pnpm --version`

Same-family child calls still reach the real binary

```
10.19.0
```

## `npx --offline --call 'pnpm exec node -e "require('\''node:assert/strict'\'').equal(process.version,'\''v20.18.0'\'');console.log('\''project Node version preserved'\'')"'`

The nested package manager keeps the selected Node.js runtime

```
project Node version preserved
```

## `npx --offline --call 'pnpm exec npx --offline --call "pnpm --version"'`

Repeated cross-family calls complete without a shim loop

```
10.19.0
```

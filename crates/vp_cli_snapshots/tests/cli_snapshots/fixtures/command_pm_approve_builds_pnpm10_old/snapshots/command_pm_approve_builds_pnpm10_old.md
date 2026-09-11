# command_pm_approve_builds_pnpm10_old

## `vp pm approve-builds --all`

pnpm 10.31.0 < 10.32.0 → rejected with friendly UserMessage (no `error:` prefix)

**Exit code:** 1

```
`--all` requires pnpm >= 10.32.0. Upgrade pnpm or pass package names explicitly.
```

## `vp pm approve-builds esbuild !core-js`

pnpm 10.31.0 < 11.0.0 → !pkg deny syntax rejected

**Exit code:** 1

```
`!<pkg>` deny syntax requires pnpm >= 11.0.0. Upgrade pnpm or omit the `!` entries.
```

## `vp pm approve-builds esbuild`

plain positional still works on old pnpm

```
There are no packages awaiting approval
```

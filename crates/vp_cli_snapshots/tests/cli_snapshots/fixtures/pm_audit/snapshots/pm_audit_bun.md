# pm_audit_bun

## `vp install -- --silent`

should install packages first

```
VITE+ - The Unified Toolchain for the Web
```

## `vp pm audit --production`

should audit production dependencies

```
bun audit <version> (<hash>)

No vulnerabilities found (checked 1 package) [<duration>]
```

## `vp pm audit --fix`

bun 1.4 runs bun audit fix

```
bun audit fix <version> (<hash>)

No vulnerabilities found (checked 1 package) [<duration>]
```

## `node ../setup-yarn.cjs`


## `vpt json-edit package.json packageManager yarn@2.3.1`


## `vp pm audit --json`

Yarn Berry before 2.4.0 has no npm audit, so warn without querying the registry.

```
warn: yarn npm audit requires yarn >= 2.4.0. Skipping.
```

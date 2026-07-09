# pm_ci_package_managers

Covers `vp pm ci` command delegation for each supported package manager.
Fake managed package-manager installs live under VP_HOME, so the case records
the exact argv Vite+ delegates without hitting the network or real installers.

## `node scripts/setup-fake-pms.cjs`


## `vpt json-edit package.json packageManager pnpm@11.0.0`


## `vp pm ci`

pnpm uses the documented clean + frozen install flow

```
pnpm clean
pnpm install --frozen-lockfile
```

## `vpt json-edit package.json packageManager npm@10.5.0`


## `vp pm ci`

npm keeps native ci delegation

```
npm ci
```

## `vpt json-edit package.json packageManager yarn@1.22.22`


## `vp pm ci`

Yarn Classic uses frozen-lockfile install

```
yarn install --frozen-lockfile
```

## `vpt json-edit package.json packageManager yarn@4.0.0`


## `vp pm ci`

Yarn Berry uses immutable install

```
yarn install --immutable
```

## `vpt json-edit package.json packageManager bun@1.2.0`


## `vp pm ci`

Bun keeps native ci delegation

```
bun ci
```

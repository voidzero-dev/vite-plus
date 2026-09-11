# shim_pnpm_uses_project_node_version

## `pnpm --version`

The unpinned pnpm shim resolves the latest version


## `NPM_CONFIG_REGISTRY=http://127.0.0.1:9 pnpm --version`

The unpinned shim reuses its fresh latest-version cache without registry access


## `node -e 'const {execFileSync}=require('\''node:child_process'\'');const {delimiter}=require('\''node:path'\'');const env={...process.env,PATH:[process.env.VP_HOME+'\''/bin'\'','\''/usr/bin'\'','\''/bin'\''].join(delimiter)};const version=execFileSync('\''pnpm'\'',['\''--version'\''],{encoding:'\''utf8'\'',env}).trim();if('\!'version)process.exit(1);console.log('\''node child reached pnpm shim'\'')'`

A Node process launched through its shim can invoke the pnpm shim

```
node child reached pnpm shim
```

## `node -e 'const {execFileSync}=require('\''node:child_process'\'');execFileSync('\''pnpm'\'',['\''--silent'\'','\''exec'\'','\''node'\'','\''-e'\'','\''if(process.version'\!'==process.env.EXPECTED_NODE_VERSION)process.exit(1)'\''],{stdio:'\''inherit'\'',env:{...process.env,EXPECTED_NODE_VERSION:process.version}});console.log('\''pnpm child uses project Node version'\'')'`

pnpm uses the project Node version

```
pnpm child uses project Node version
```

## `vp env exec --node 22.13 pnpm exec node -e 'if('\!'process.version.startsWith('\''v22.13.'\''))process.exit(1);console.log('\''explicit Node reaches pnpm child'\'')'`

Explicit env exec version overrides the project version through the pnpm shim

```
explicit Node reaches pnpm child
```

## `vpt write-file .node-version '>=999.0.0
'`


## `VP_NODE_DIST_MIRROR=http://127.0.0.1:9 pnpm --version`

JS package-manager shims report project Node resolution failures

**Exit code:** 1

```
vp: Failed to resolve Node version: Failed to download Node.js runtime: No version matching '>=999.0.0' found
vp: Run 'vp env doctor' for diagnostics
```

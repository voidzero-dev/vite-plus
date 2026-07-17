# command_env_install_node_version

## `vp install -g --node 22 ./command-env-install-node-version-pkg`

Install with Node.js 22

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed command-env-install-node-version-pkg 1.0.0
  Bins: command-env-install-node-version-pkg-cli
```

## `node -e 'const d=JSON.parse(require('\''fs'\'').readFileSync(process.env.VP_HOME+'\''/bins/command-env-install-node-version-pkg-cli.json'\'','\''utf8'\'')); console.log('\''Node major:'\'', d.nodeVersion.split('\''.'\'')[0])'`

Verify Node 22

```
Node major: 22
```

## `vp remove -g command-env-install-node-version-pkg`

Cleanup

```
Uninstalled command-env-install-node-version-pkg
```

## `vp install -g --node 20 ./command-env-install-node-version-pkg`

Install with Node.js 20

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
error: Failed to install command-env-install-node-version-pkg: Package was not installed correctly, package.json not found at <home>/.vite-plus/packages/command-env-install-node-version-pkg#<uuid>/lib/node_modules/command-env-install-node-version-pkg/package.json
```

## `node -e 'const d=JSON.parse(require('\''fs'\'').readFileSync(process.env.VP_HOME+'\''/bins/command-env-install-node-version-pkg-cli.json'\'','\''utf8'\'')); console.log('\''Node major:'\'', d.nodeVersion.split('\''.'\'')[0])'`

Verify Node 20

**Exit code:** 1

```
node:fs:441
    return binding.readFileUtf8(path, stringToFlags(options.flag));
                   ^

Error: ENOENT: no such file or directory, open '<home>/.vite-plus/bins/command-env-install-node-version-pkg-cli.json'
    at Object.readFileSync (node:fs:441:20)
    at [eval]:1:34
    at runScriptInThisContext (node:internal/vm:219:10)
    at node:internal/process/execution:451:12
    at [eval]-wrapper:6:24
    at runScriptInContext (node:internal/process/execution:449:60)
    at evalFunction (node:internal/process/execution:283:30)
    at evalTypeScript (node:internal/process/execution:295:3)
    at node:internal/main/eval_string:71:3 {
  errno: -2,
  code: 'ENOENT',
  syscall: 'open',
  path: '<home>/.vite-plus/bins/command-env-install-node-version-pkg-cli.json'
}

Node.js <version>
```

## `vp remove -g command-env-install-node-version-pkg`

Cleanup

**Exit code:** 1

```
Failed to uninstall command-env-install-node-version-pkg: Package command-env-install-node-version-pkg is not installed
```

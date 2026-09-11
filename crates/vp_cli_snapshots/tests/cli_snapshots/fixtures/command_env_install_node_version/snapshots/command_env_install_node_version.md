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

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed command-env-install-node-version-pkg 1.0.0
  Bins: command-env-install-node-version-pkg-cli
```

## `node -e 'const d=JSON.parse(require('\''fs'\'').readFileSync(process.env.VP_HOME+'\''/bins/command-env-install-node-version-pkg-cli.json'\'','\''utf8'\'')); console.log('\''Node major:'\'', d.nodeVersion.split('\''.'\'')[0])'`

Verify Node 20

```
Node major: 20
```

## `vp remove -g command-env-install-node-version-pkg`

Cleanup

```
Uninstalled command-env-install-node-version-pkg
```

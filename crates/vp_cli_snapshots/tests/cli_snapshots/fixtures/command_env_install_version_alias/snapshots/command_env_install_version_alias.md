# command_env_install_version_alias

## `vp install -g --node lts ./command-env-install-version-alias-pkg`

Install with LTS alias

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed command-env-install-version-alias-pkg 1.0.0
  Bins: command-env-install-version-alias-pkg-cli
```

## `node -e 'const d=JSON.parse(require('\''fs'\'').readFileSync(process.env.VP_HOME+'\''/bins/command-env-install-version-alias-pkg-cli.json'\'','\''utf8'\'')); const v=parseInt(d.nodeVersion.split('\''.'\'')[0]); console.log('\''LTS major >= 20:'\'', v >= 20)'`

Verify LTS version

```
LTS major >= 20: true
```

## `vp remove -g command-env-install-version-alias-pkg`

Cleanup

```
Uninstalled command-env-install-version-alias-pkg
```

## `vp install -g --node latest ./command-env-install-version-alias-pkg`

Install with latest alias

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed command-env-install-version-alias-pkg 1.0.0
  Bins: command-env-install-version-alias-pkg-cli
```

## `node -e 'const d=JSON.parse(require('\''fs'\'').readFileSync(process.env.VP_HOME+'\''/bins/command-env-install-version-alias-pkg-cli.json'\'','\''utf8'\'')); const v=parseInt(d.nodeVersion.split('\''.'\'')[0]); console.log('\''Latest major >= 20:'\'', v >= 20)'`

Verify latest version

```
Latest major >= 20: true
```

## `vp remove -g command-env-install-version-alias-pkg`

Cleanup

```
Uninstalled command-env-install-version-alias-pkg
```

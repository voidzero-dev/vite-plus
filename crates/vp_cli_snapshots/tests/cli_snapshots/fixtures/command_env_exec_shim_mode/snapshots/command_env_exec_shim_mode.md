# command_env_exec_shim_mode

## `vp env exec node -v`

Shim mode: version resolved from package.json engines.node

```
<version>
```

## `vp env exec npm -v`

Shim mode: npm uses same version

```
10.8.2
```

## `vp env exec node -e 'console.log('\''Hello from shim mode'\'')'`

Shim mode: run inline script

```
Hello from shim mode
```

## `vp env exec nonexistent-tool --version`

expected error: non-shim command requires --node

**Exit code:** 1

```
vp env exec: --node is required when running non-shim commands
Usage: vp env exec --node <version> <command> [args...]

For shim tools, --node is optional (version resolved automatically):
  vp env exec node script.js    # Core tool
  vp env exec npm install       # Core tool
  vp env exec tsc --version     # Global package
```

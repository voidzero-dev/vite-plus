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

automatic mode resolves the environment before reporting a missing command

**Exit code:** 1

```
error: Command execution failed: program not found
```

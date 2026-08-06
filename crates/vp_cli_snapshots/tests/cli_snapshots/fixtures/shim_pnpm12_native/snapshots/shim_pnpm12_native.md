# shim_pnpm12_native

pnpm 12 ships a native binary via @pnpm/exe.* platform packages; the pnpm shim runs it directly and the pnpx shim injects the dlx subcommand.

## `vp install -g pnpm`

Expose the pnpm/pnpx shims


## `vp env exec node --version`

Ensure Node.js is installed first


## `pnpm --version`

pnpm shim downloads the native binary and resolves the pinned packageManager version (12.0.0-beta.0)

```
12.0.0-beta.0
```

## `pnpx --silent cowsay hello`

pnpx shim injects dlx so the native binary runs the package

```
 _______
< hello >
 -------
        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||
```

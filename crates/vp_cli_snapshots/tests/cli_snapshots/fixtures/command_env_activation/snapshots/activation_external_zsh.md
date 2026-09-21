# activation_external_zsh

## `node verify.mjs external zsh`

```
First setup:
$ vp env list node
Setup:
  Preparing vite-plus environment.

Created Shims:
  <workspace>/activation/home/bin/node
  <workspace>/activation/home/fallback-bin/npm
  <workspace>/activation/home/fallback-bin/npx
  <workspace>/activation/home/fallback-bin/pnpm
  <workspace>/activation/home/fallback-bin/pnpx
  <workspace>/activation/home/fallback-bin/yarn
  <workspace>/activation/home/fallback-bin/yarnpkg
  <workspace>/activation/home/fallback-bin/bun
  <workspace>/activation/home/fallback-bin/bunx
  <workspace>/activation/home/bin/vpx
  <workspace>/activation/home/bin/vpr

Next Steps:
  Activate Vite+ in this terminal:
  . "<workspace>/activation/home/env"

  Add the command to your .zshrc file to activate future Zsh terminals.

  Restart an already-running IDE to load its environment. Run `vp env doctor` to verify.
✓ Vite+ setup complete.
VITE+ - The Unified Toolchain for the Web

Node.js
  * <version> current

note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
Setup with an existing profile entry:
$ vp env setup
VITE+ - The Unified Toolchain for the Web

Setup:
  Preparing vite-plus environment.

Skipped Shims:
  <workspace>/activation/home/bin/node
  <workspace>/activation/home/fallback-bin/npm
  <workspace>/activation/home/fallback-bin/npx
  <workspace>/activation/home/fallback-bin/pnpm
  <workspace>/activation/home/fallback-bin/pnpx
  <workspace>/activation/home/fallback-bin/yarn
  <workspace>/activation/home/fallback-bin/yarnpkg
  <workspace>/activation/home/fallback-bin/bun
  <workspace>/activation/home/fallback-bin/bunx
  <workspace>/activation/home/bin/vpx
  <workspace>/activation/home/bin/vpr

  Use --refresh to update existing shims.

Next Steps:
  Activate Vite+ in this terminal:
  . "<workspace>/activation/home/env"

  Or open a new terminal to load your configured shell profile.

  Restart an already-running IDE to load its environment. Run `vp env doctor` to verify.
Same terminal:
$ command -v node
<workspace>/activation/system/node
$ node --version
system-node
$ . "<workspace>/activation/home/env"
$ command -v node
<workspace>/activation/home/bin/node
$ node --version
<version>
$ vp env list node
VITE+ - The Unified Toolchain for the Web

Node.js
  * <version> current

note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
```

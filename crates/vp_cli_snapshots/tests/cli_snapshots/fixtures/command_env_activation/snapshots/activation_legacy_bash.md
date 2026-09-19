# activation_legacy_bash

## `node verify.mjs legacy bash`

```
First setup:
  Activate Vite+ in this terminal:
  . "<workspace>/activation/user/.vite-plus/env"
  Add the command for your shell to its profile to activate future terminals.
Setup with an existing profile entry:
  Or open a new terminal to load your configured shell profile.
Same terminal:
$ command -v node
<workspace>/activation/system/node
$ node --version
system-node
$ . "<workspace>/activation/user/.vite-plus/env"
$ command -v node
<workspace>/activation/user/.vite-plus/bin/node
$ node --version
<version>
$ vp env list node
VITE+ - The Unified Toolchain for the Web

Node.js
  * <version> current

note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
```

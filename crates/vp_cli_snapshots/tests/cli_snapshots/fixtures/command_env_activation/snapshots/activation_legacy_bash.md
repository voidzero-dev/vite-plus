# activation_legacy_bash

## `node verify.mjs legacy bash`

```
First setup:
  Activate Vite+ in this terminal:
  . "<workspace>/activation/user/.vite-plus/env"
  Add the command to ~/.bashrc for interactive non-login Bash sessions.
  Login Bash shells must also load the command through their login profile.
Setup with an existing profile entry:
  Or start an interactive non-login Bash shell to load your configured ~/.bashrc.
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

# activation_external_zsh_linux

## `node verify.mjs external zsh`

```
First setup:
$ vp env list node
Setup:
  Preparing vite-plus environment.

Created Shims:
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/node
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/npm
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/npx
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/pnpm
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/pnpx
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/yarn
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/yarnpkg
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/bun
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/bunx
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/vpx
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/vpr

Next Steps:
  Activate Vite+ in this terminal:
  . "<workspace>/activation/home space 'quote' \$cash \`tick\` \"double\" \\slash/env"

  Add the command for your shell to its profile to activate future terminals.
  If setup already updated your profile, you can open a new terminal instead.

  For IDE support (VS Code, Cursor), ensure bin directory is in system PATH:
  - Linux: Add to ~/.profile for display manager integration

  Restart an already-running IDE to load its environment. Run `vp env doctor` to verify.
✓ Vite+ setup complete.
VITE+ - The Unified Toolchain for the Web

Node.js
  * <version> current

note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
Same terminal:
$ zsh -f session.sh
$ command -v node
<workspace>/activation/system/node
$ node
system-node
$ vp env list node
Vite+ managed tools are not first on PATH. Activate in this terminal:
  . "<workspace>/activation/home space 'quote' \$cash \`tick\` \"double\" \\slash/env"
VITE+ - The Unified Toolchain for the Web

Node.js
  * <version> current

note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
$ . "<workspace>/activation/home space 'quote' \$cash \`tick\` \"double\" \\slash/env"
$ node --version
<version>
$ vp env list node
VITE+ - The Unified Toolchain for the Web

Node.js
  * <version> current

note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
$ export PATH="$ACTIVATION_SYSTEM:$ACTIVATION_BIN"
$ node
system-node
$ vp env list node
Vite+ managed tools are not first on PATH. Activate in this terminal:
  . "<workspace>/activation/home space 'quote' \$cash \`tick\` \"double\" \\slash/env"
VITE+ - The Unified Toolchain for the Web

Node.js
  * <version> current

note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
$ . "<workspace>/activation/home space 'quote' \$cash \`tick\` \"double\" \\slash/env"
$ node --version
<version>
$ vp env list node
VITE+ - The Unified Toolchain for the Web

Node.js
  * <version> current

note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
Another unactivated terminal:
$ vp env list node
Vite+ managed tools are not first on PATH. Activate in this terminal:
  . "<workspace>/activation/home space 'quote' \$cash \`tick\` \"double\" \\slash/env"
VITE+ - The Unified Toolchain for the Web

Node.js
  * <version> current

note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
Preferences, setup state, and installation ownership unchanged.
Equivalent shim directory on PATH:
$ PATH='<workspace>/activation/bin-alias:<workspace>/activation/system' vp env list node
VITE+ - The Unified Toolchain for the Web

Node.js
  * <version> current

note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
JSON output:
$ vp env list node --json
{
  "node": [
    {
      "version": "22.18.0",
      "current": true,
      "default": false
    }
  ]
}
Shell-evaluated output:
$ VP_ENV_USE_EVAL_ENABLE=1 vp env use 22.18.0 --no-install
export VP_NODE_VERSION=22.18.0
Using Node.js <version> (resolved from 22.18.0)
CI with a TTY:
$ CI=1 vp env list node
VITE+ - The Unified Toolchain for the Web

Node.js
  * <version> current

note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
Redirected stdout, stderr, and stdin:
$ vp env list node
note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
Redirected stdout:
Node.js
  * <version> current

$ vp env list node
VITE+ - The Unified Toolchain for the Web

Node.js
  * <version> current

Redirected stderr:
note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
$ vp env list node
VITE+ - The Unified Toolchain for the Web

Node.js
  * <version> current

note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
Direct node shim:
$ node --version
<version>
Completion protocol:
$ VP_COMPLETE=bash _CLAP_COMPLETE_INDEX=2 vp -- vp env li
list
list-remote
Installer capability protocol:
$ VP_SELF_SETUP_SUPPORT_CHECK=1 vp
vite-plus-self-setup-v1
Installer handoff:
$ VP_SELF_SETUP_SHELL=sh vp
Setup:
  Preparing vite-plus environment.

Created Shims:
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/node
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/npm
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/npx
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/pnpm
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/pnpx
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/yarn
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/yarnpkg
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/bun
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/bunx
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/vpx
  <workspace>/activation/home space 'quote' $cash `tick` "double" \slash/bin/vpr

Next Steps:
  Activate Vite+ in this terminal:
  . "<workspace>/activation/home space 'quote' \$cash \`tick\` \"double\" \\slash/env"

  Add the command for your shell to its profile to activate future terminals.
  If setup already updated your profile, you can open a new terminal instead.

  For IDE support (VS Code, Cursor), ensure bin directory is in system PATH:
  - Linux: Add to ~/.profile for display manager integration

  Restart an already-running IDE to load its environment. Run `vp env doctor` to verify.
✓ Vite+ setup complete.
INSTALL_DIR="<workspace>/activation/home space 'quote' \$cash \`tick\` \"double\" \\slash"
SHIM_DIR="<workspace>/activation/home space 'quote' \$cash \`tick\` \"double\" \\slash/bin"
CACHE_DIR="<workspace>/activation/home space 'quote' \$cash \`tick\` \"double\" \\slash/cache"
CONFIG_DIR="<workspace>/activation/home space 'quote' \$cash \`tick\` \"double\" \\slash"
STATE_DIR="<workspace>/activation/home space 'quote' \$cash \`tick\` \"double\" \\slash"
Internal background protocol:
$ vp upgrade --background-check
Quiet mode:
$ vp upgrade --silent
error: Upgrade error: Homebrew manages this installation. Run `brew upgrade vite-plus` to update it.
Failing eligible command keeps exit status:
$ vp env clean invalid-scope
Vite+ managed tools are not first on PATH. Activate in this terminal:
  . "<workspace>/activation/home space 'quote' \$cash \`tick\` \"double\" \\slash/env"
VITE+ - The Unified Toolchain for the Web

error: invalid environment scope "invalid-scope"; expected node, pm, npm, pnpm, yarn, or bun
System-first mode:
$ vp env off
VITE+ - The Unified Toolchain for the Web

✓ Node.js and package-manager management set to system-first.

Selected commands and shims will now prefer system tools, falling back to managed tools.

Run `vp env on` to always use Vite+ managed tools.
$ vp env list node
VITE+ - The Unified Toolchain for the Web

Node.js
  * <version> current

note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
Unknown current shell gets labeled commands despite SHELL=fish:
$ VP_SHELL='' vp env list node
Vite+ managed tools are not first on PATH. Activate in this terminal:
  Bash/Zsh: . "<workspace>/activation/home space 'quote' \$cash \`tick\` \"double\" \\slash/env"
  Fish: source "<workspace>/activation/home space 'quote' \$cash `tick` \"double\" \\slash/env.fish"
  Nushell: source "<workspace>/activation/home space 'quote' $cash `tick` \"double\" \\slash/env.nu"
  PowerShell: . '<workspace>/activation/home space ''quote'' $cash `tick` "double" \slash/env.ps1'
VITE+ - The Unified Toolchain for the Web

Node.js
  * <version> current

note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
Missing environment file needs repair:
$ vp env list node
Vite+ shell environment files are missing. Run `vp env setup` to recreate them.
VITE+ - The Unified Toolchain for the Web

Node.js
  * <version> current

note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
Missing shim needs repair:
$ vp env list node
Vite+ managed tool shims are missing or unusable. Run `vp env setup --refresh` to repair them.
VITE+ - The Unified Toolchain for the Web

Node.js
  * <version> current

note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
```

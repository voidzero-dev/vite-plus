# command_self_setup_external_doctor

## `node verify-refresh.mjs doctor`

```
Homebrew with shims on PATH
    CLI source        Homebrew
    CLI binary        <workspace>/doctor/cellar/vite-plus/0.3.2/bin/vp
  ✓ vp                ~/bin/vp
  ✓ Shim dir          ~/bin
Homebrew without shims on PATH
    CLI source        Homebrew
    CLI binary        <workspace>/doctor/cellar/vite-plus/0.3.2/bin/vp
  ✓ vp                <workspace>/doctor/brew/bin/vp
  ✗ Shim dir          not in PATH
Missing vp in the shim directory
    CLI source        Homebrew
    CLI binary        <workspace>/doctor/cellar/vite-plus/0.3.2/bin/vp
  ✗ vp                not in PATH
  ✓ Shim dir          ~/bin
External package without a Homebrew receipt
  ✓ vp                ~/bin/vp
  ✓ Shim dir          ~/bin
```

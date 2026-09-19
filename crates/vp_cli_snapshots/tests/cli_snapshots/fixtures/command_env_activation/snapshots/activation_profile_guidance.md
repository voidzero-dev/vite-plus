# activation_profile_guidance

## `node profile-guidance.mjs bash`

```
Only Fish is configured:
VP_SHELL=<unset>:
Next Steps:
  Activate Vite+ in this terminal:
  Bash/Zsh: . "<home>/.vite-plus/env"
  Fish: source "<home>/.vite-plus/env.fish"
  Nushell: source "<home>/.vite-plus/env.nu"
  PowerShell: . '<home>/.vite-plus/env.ps1'

  If your shell profile does not already load Vite+, add the command for your shell.
  For PowerShell, add its command to $PROFILE if it is not already there.

  Restart an already-running IDE to load its environment. Run `vp env doctor` to verify.
$ bash --noprofile -ic 'command -v node; node --version'
<workspace>/profiles/system/node
system-node
VP_SHELL=unrecognized:
Next Steps:
  Activate Vite+ in this terminal:
  Bash/Zsh: . "<home>/.vite-plus/env"
  Fish: source "<home>/.vite-plus/env.fish"
  Nushell: source "<home>/.vite-plus/env.nu"
  PowerShell: . '<home>/.vite-plus/env.ps1'

  If your shell profile does not already load Vite+, add the command for your shell.
  For PowerShell, add its command to $PROFILE if it is not already there.

  Restart an already-running IDE to load its environment. Run `vp env doctor` to verify.
$ bash --noprofile -ic 'command -v node; node --version'
<workspace>/profiles/system/node
system-node
Only the Bash login profile is configured:
VP_SHELL=bash:
Next Steps:
  Activate Vite+ in this terminal:
  . "<home>/.vite-plus/env"

  Add the command to ~/.bashrc for interactive non-login Bash sessions.
  Login Bash shells must also load the command through their login profile.

  Restart an already-running IDE to load its environment. Run `vp env doctor` to verify.
$ bash --noprofile -ic 'command -v node; node --version'
<workspace>/profiles/system/node
system-node
Bash .bashrc is configured:
VP_SHELL=bash:
Next Steps:
  Activate Vite+ in this terminal:
  . "<home>/.vite-plus/env"

  Or start an interactive non-login Bash shell to load your configured ~/.bashrc.
  Login Bash shells must also load the command through their login profile.

  Restart an already-running IDE to load its environment. Run `vp env doctor` to verify.
$ bash --noprofile -ic 'command -v node; node --version'
<home>/.vite-plus/bin/node
<version>
```

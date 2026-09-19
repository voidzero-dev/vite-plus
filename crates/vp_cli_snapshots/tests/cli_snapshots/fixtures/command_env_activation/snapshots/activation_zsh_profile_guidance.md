# activation_zsh_profile_guidance

Login startup reorders PATH after .zshenv; .zshrc must activate the shims afterward.

## `node profile-guidance.mjs zsh-env`

```
Only Zsh .zshenv is configured:
VP_SHELL=zsh:
Next Steps:
  Activate Vite+ in this terminal:
  . "<home>/.vite-plus/env"

  Add the command to your .zshrc file to activate future Zsh terminals.

  Restart an already-running IDE to load its environment. Run `vp env doctor` to verify.
$ zsh -lic 'command -v node; node --version'
<workspace>/profiles/system/node
system-node
```

## `node profile-guidance.mjs zsh-interactive`

```
Zsh .zshrc is configured:
VP_SHELL=zsh:
Next Steps:
  Activate Vite+ in this terminal:
  . "<home>/.vite-plus/env"

  Or open a new terminal to load your configured shell profile.

  Restart an already-running IDE to load its environment. Run `vp env doctor` to verify.
$ zsh -lic 'command -v node; node --version'
<home>/.vite-plus/bin/node
<version>
```

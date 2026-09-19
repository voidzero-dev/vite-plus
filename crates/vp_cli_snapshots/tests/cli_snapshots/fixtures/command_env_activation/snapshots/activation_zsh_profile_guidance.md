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
```

## `HOME=${workspace}/profiles/user ZDOTDIR=${workspace}/profiles/user/zsh EXPECTED_NODE=${workspace}/profiles/system/node zsh -lic 'command -v node; test "$(command -v node)" = "$EXPECTED_NODE" || exit 1; node --version'`

```
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
```

## `HOME=${workspace}/profiles/user ZDOTDIR=${workspace}/profiles/user/zsh EXPECTED_NODE=${VP_HOME}/bin/node zsh -lic 'command -v node; test "$(command -v node)" = "$EXPECTED_NODE" || exit 1; node --version'`

```
<home>/.vite-plus/bin/node
<version>
```

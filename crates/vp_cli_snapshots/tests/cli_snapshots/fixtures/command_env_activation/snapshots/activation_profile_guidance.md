# activation_profile_guidance

## `node profile-guidance.mjs unset`

```
Only Fish is configured:
VP_SHELL=<unset>:
Next Steps:
  For Bash, run:
  . "<home>/.vite-plus/env"

  If your ~/.bashrc does not already load Vite+, add this command for interactive non-login Bash sessions.
  Login Bash shells must also load the command through their login profile.

  Restart an already-running IDE to load its environment. Run `vp env doctor` to verify.
$ bash --noprofile -ic 'command -v node; node --version'
<workspace>/profiles/system/node
system-node
```

## `node profile-guidance.mjs unrecognized`

```
Only Fish is configured:
VP_SHELL=unrecognized:
Next Steps:
  For Bash, run:
  . "<home>/.vite-plus/env"

  If your ~/.bashrc does not already load Vite+, add this command for interactive non-login Bash sessions.
  Login Bash shells must also load the command through their login profile.

  Restart an already-running IDE to load its environment. Run `vp env doctor` to verify.
$ bash --noprofile -ic 'command -v node; node --version'
<workspace>/profiles/system/node
system-node
```

## `node profile-guidance.mjs bash-login`

```
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
```

## `node profile-guidance.mjs bash-interactive`

```
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

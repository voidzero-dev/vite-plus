# activation_cmd_path_guidance

## `node profile-guidance.mjs cmd`

```
VP_SELF_SETUP_NO_MODIFY_PATH=<unset>:
VP_SHELL=cmd:
Next Steps:
  Activate Vite+ in this terminal:
  set "PATH=<home>/.vite-plus/bin;%PATH%"

  For future cmd.exe sessions, add this directory to your user PATH if it is missing:
  <home>/.vite-plus/bin
  System Properties -> Environment Variables -> User variables -> Path
  Open a new terminal after updating PATH.

  Restart an already-running IDE to load its environment. Run `vp env doctor` to verify.
```

## `node profile-guidance.mjs cmd-no-modify-path`

```
VP_SELF_SETUP_NO_MODIFY_PATH=1:
VP_SHELL=cmd:
Next Steps:
  Activate Vite+ in this terminal:
  set "PATH=<home>/.vite-plus/bin;%PATH%"

  For future cmd.exe sessions, add this directory to your user PATH if it is missing:
  <home>/.vite-plus/bin
  System Properties -> Environment Variables -> User variables -> Path
  Open a new terminal after updating PATH.

  Restart an already-running IDE to load its environment. Run `vp env doctor` to verify.
```

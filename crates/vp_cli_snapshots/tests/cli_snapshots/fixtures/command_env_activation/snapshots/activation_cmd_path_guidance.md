# activation_cmd_path_guidance

## `node profile-guidance.mjs cmd`

```
VP_SELF_SETUP_NO_MODIFY_PATH=<unset>:
VP_SHELL=cmd:
Next Steps:
  Activate Vite+ in this terminal:
  set "PATH=<home>/.vite-plus/bin;%PATH%;<home>/.vite-plus/fallback-bin"

  For future cmd.exe sessions, add these directories to your user PATH if missing:
  At the start: <home>/.vite-plus/bin
  At the end: <home>/.vite-plus/fallback-bin
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
  set "PATH=<home>/.vite-plus/bin;%PATH%;<home>/.vite-plus/fallback-bin"

  For future cmd.exe sessions, add these directories to your user PATH if missing:
  At the start: <home>/.vite-plus/bin
  At the end: <home>/.vite-plus/fallback-bin
  System Properties -> Environment Variables -> User variables -> Path
  Open a new terminal after updating PATH.

  Restart an already-running IDE to load its environment. Run `vp env doctor` to verify.
```

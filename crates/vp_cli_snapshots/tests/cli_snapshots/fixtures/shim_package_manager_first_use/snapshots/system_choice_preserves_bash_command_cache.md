# system_choice_preserves_bash_command_cache

## `vpt rm -f $VP_HOME/config.json`


## `vpt chmod +x system-bin/pnpm`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} bash --noprofile --norc bash-cache.sh`

The first-use choice must leave Bash's cached pnpm executable usable for the next direct call.

**→ expect-milestone:** `pm-shim-choice:pnpm`

```
vp: Vite+ now can manage package-manager versions for each project.
Existing pnpm: <workspace>/system-bin/pnpm
```

**← write-key:** `down`

**← write-key:** `enter`

```
vp: Vite+ now can manage package-manager versions for each project.
Existing pnpm: <workspace>/system-bin/pnpm

? How should pnpm run? ›
✔ How should pnpm run? · Use system pnpm
system-pnpm
<home>/.vite-plus/bin/pnpm
system-pnpm
Both pnpm calls succeeded with the same Bash command cache
```

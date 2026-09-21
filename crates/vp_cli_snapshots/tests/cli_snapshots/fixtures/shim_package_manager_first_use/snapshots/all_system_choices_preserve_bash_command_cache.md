# all_system_choices_preserve_bash_command_cache

## `vpt rm -f $VP_HOME/config.json`


## `vpt chmod +x system-bin/pnpm`


## `vpt chmod +x system-bin/pnpx`


## `vpt chmod +x system-bin/yarn`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} bash --noprofile --norc bash-cache.sh all`

Choosing system tools for all families must also preserve cached aliases and other managers.

**→ expect-milestone:** `pm-shim-choice:pnpm`

```
vp: Vite+ now can manage package-manager versions for each project.
Existing pnpm: <workspace>/system-bin/pnpm
```

**← write-key:** `down`

**← write-key:** `down`

**← write-key:** `enter`

```
vp: Vite+ now can manage package-manager versions for each project.
Existing pnpm: <workspace>/system-bin/pnpm

? How should pnpm run? ›
✔ How should pnpm run? · Use system package managers
system-pnpm
<home>/.vite-plus/bin/pnpm
system-pnpm
Both pnpm calls succeeded with the same Bash command cache
system-pnpx
system-yarn
Cached aliases and other families still work after choosing all system managers
```

## `vpt print-file $VP_HOME/config.json`

The choice applies to every package-manager family.

```
{
  "packageManagerShimModes": {
    "bun": "system_first",
    "npm": "system_first",
    "pnpm": "system_first",
    "yarn": "system_first"
  }
}
```

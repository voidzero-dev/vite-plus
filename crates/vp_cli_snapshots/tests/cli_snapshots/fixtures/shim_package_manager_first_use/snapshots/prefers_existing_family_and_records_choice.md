# prefers_existing_family_and_records_choice

## `vpt rm -f $VP_HOME/config.json`


## `vpt chmod +x system-bin/pnpm`


## `vpt chmod +x system-bin/yarn`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} pnpm --version`

an upgraded install asks before replacing an existing pnpm

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
✔ How should pnpm run? · Use system pnpm
system-pnpm
```

## `vpt print-file $VP_HOME/config.json`

the explicit system choice records only pnpm

```
{
  "packageManagerShimModes": {
    "pnpm": "system_first"
  }
}
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} pnpm --version`

later pnpm invocations use the recorded choice without prompting

```
system-pnpm
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} yarn --version`

an undecided Yarn family still prompts

**→ expect-milestone:** `pm-shim-choice:yarn`

```
vp: Vite+ now can manage package-manager versions for each project.
Existing yarn: <workspace>/system-bin/yarn
```

**← write-key:** `down`

**← write-key:** `down`

**← write-key:** `enter`

```
vp: Vite+ now can manage package-manager versions for each project.
Existing yarn: <workspace>/system-bin/yarn

? How should yarn run? ›
✔ How should yarn run? · Use system yarn
system-yarn
```

## `vpt print-file $VP_HOME/config.json`

Yarn records its own decision without changing pnpm

```
{
  "packageManagerShimModes": {
    "pnpm": "system_first",
    "yarn": "system_first"
  }
}
```

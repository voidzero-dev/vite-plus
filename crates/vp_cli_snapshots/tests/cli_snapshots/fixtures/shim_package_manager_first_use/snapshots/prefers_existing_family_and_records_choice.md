# prefers_existing_family_and_records_choice

## `vpt rm -f $VP_HOME/config.json`


## `vpt chmod +x system-bin/pnpm`


## `vpt chmod +x system-bin/yarn`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} pnpm --version`


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

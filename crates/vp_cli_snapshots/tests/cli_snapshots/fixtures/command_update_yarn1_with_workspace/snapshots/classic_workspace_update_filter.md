# classic_workspace_update_filter

## `vp install --ignore-scripts`


## `vp update --filter app --latest is-number -- --ignore-scripts`

a single non-interactive filter still updates only the selected workspace

```
yarn workspace <version>
yarn upgrade <version>
warning package.json: No license field
[1/4] Resolving packages...
[2/4] Fetching packages...
[3/4] Linking dependencies...
[4/4] Rebuilding all packages...
warning Ignored scripts due to flag.
success Saved lockfile.
success Saved 1 new dependency.
info Direct dependencies
info All dependencies
└─ is-number@7.0.0

Done in <duration>.

Done in <duration>.
```

## `vpt print-file package.json packages/app/package.json packages/web/package.json`

```
{
  "name": "command-update-yarn1-with-workspace",
  "version": "1.0.0",
  "private": true,
  "packageManager": "yarn@1.22.22",
  "workspaces": ["packages/*"],
  "dependencies": { "is-number": "6.0.0" },
  "devDependencies": { "yocto-queue": "0.1.0" }
}
{
  "name": "app",
  "version": "1.0.0",
  "dependencies": {
    "is-number": "7.0.0"
  }
}
{
  "name": "web",
  "version": "1.0.0",
  "dependencies": { "is-number": "6.0.0" }
}
```

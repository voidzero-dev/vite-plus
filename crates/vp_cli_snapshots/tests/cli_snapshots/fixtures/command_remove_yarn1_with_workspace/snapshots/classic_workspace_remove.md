# classic_workspace_remove

## `vp install --ignore-scripts`


## `vp remove --filter app is-number -- --ignore-scripts`

a single filter removes only from the selected workspace via yarn workspace

```
yarn workspace <version>
yarn remove <version>
warning package.json: No license field
[1/2] Removing module is-number...
[2/2] Regenerating lockfile and installing missing dependencies...
success Uninstalled packages.

Done in <duration>.

Done in <duration>.
```

## `vpt print-file package.json packages/app/package.json packages/web/package.json`

```
{
  "name": "command-remove-yarn1-with-workspace",
  "version": "1.0.0",
  "private": true,
  "packageManager": "yarn@1.22.22",
  "workspaces": ["packages/*"],
  "dependencies": { "is-number": "6.0.0" }
}
{
  "name": "app",
  "version": "1.0.0",
  "dependencies": {}
}
{
  "name": "web",
  "version": "1.0.0",
  "dependencies": { "is-number": "6.0.0" }
}
```

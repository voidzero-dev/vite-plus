# command_remove_npm8_nested_workspace

## `cd packages/app && vp remove -w testnpm2 -- --no-audit`

remove only the outer root declaration when the current member owns a workspace

```

added 4 packages in <duration>
```

## `cd packages/app && vp remove -w --filter docs test-vite-plus-install -- --no-audit`

select the outer docs package when the inner workspace also has a member named docs

```

up to date in <duration>
```

## `vpt print-file package.json packages/app/package.json packages/docs/package.json packages/app/plugins/docs/package.json`

```
{
  "name": "root",
  "private": true,
  "packageManager": "npm@8.1.0",
  "workspaces": [
    "packages/*"
  ]
}
{
  "name": "app",
  "version": "1.0.0",
  "packageManager": "npm@8.1.0",
  "workspaces": [
    "plugins/*"
  ],
  "dependencies": {
    "testnpm2": "1.0.1",
    "test-vite-plus-install": "1.0.0"
  }
}
{
  "name": "docs",
  "version": "1.0.0"
}
{
  "name": "docs",
  "version": "1.0.0",
  "dependencies": {
    "test-vite-plus-install": "1.0.0"
  }
}
```

## `node -p require('./package-lock.json').packages['packages/app'].dependencies`

```
{ 'test-vite-plus-install': '1.0.0', testnpm2: '1.0.1' }
```

## `vpt stat-file packages/app/package-lock.json --assert missing`

```
packages/app/package-lock.json: missing
```

# create_approve_builds_yarn

## `vp create @your-org:with-build-dep --no-interactive --approve-builds --package-manager yarn --directory approved-app`

yarn (Berry) blocks build scripts by default, so --approve-builds enables the gated build (core-js) via dependenciesMeta.built and reinstalls

```
◇ Scaffolded approved-app
• Node <version>  yarn <version>
✓ Dependencies installed in <duration>
→ Next: cd approved-app && vp run
```

## `vpt print-file approved-app/package.json`

core-js recorded under dependenciesMeta.built

```
{
  "name": "approved-app",
  "version": "0.0.0",
  "private": true,
  "scripts": {
    "prepare": "vp config"
  },
  "dependencies": {
    "core-js": "3.39.0"
  },
  "devDependencies": {
    "vite-plus": "catalog:"
  },
  "dependenciesMeta": {
    "core-js": {
      "built": true
    }
  },
  "resolutions": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  },
  "devEngines": {
    "packageManager": {
      "name": "yarn",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vp create @your-org:with-build-dep --no-interactive --package-manager yarn --directory default-app`

default run surfaces the gated build with guidance, leaving it disabled

```

Build scripts were not run for: core-js.

These dependencies may not work until built. Enable them in the workspace root package.json (dependenciesMeta.<pkg>.built: true) and reinstall, or re-create with --approve-builds.
◇ Scaffolded default-app
• Node <version>  yarn <version>
✓ Dependencies installed in <duration>
→ Next: cd default-app && vp run
```

## `vpt print-file default-app/package.json`

no dependenciesMeta, the build was not run

```
{
  "name": "default-app",
  "version": "0.0.0",
  "private": true,
  "scripts": {
    "prepare": "vp config"
  },
  "dependencies": {
    "core-js": "3.39.0"
  },
  "devDependencies": {
    "vite-plus": "catalog:"
  },
  "resolutions": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  },
  "devEngines": {
    "packageManager": {
      "name": "yarn",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

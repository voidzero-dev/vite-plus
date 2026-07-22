# create_approve_builds_yarn

## `vp create @your-org:with-build-dep --no-interactive --approve-builds --package-manager yarn --directory approved-app`

yarn (Berry) blocks build scripts by default, so --approve-builds enables the gated build (core-js) via dependenciesMeta.built and reinstalls

```

➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0016: │ @oxc-project/types@npm:=0.141.0: All versions satisfying "=0.141.0" are quarantined
➤ YN0000: └ Completed
➤ YN0000: · Failed with errors in <duration> <duration>

You may need to run "vp install" manually in <workspace>/approved-app
◇ Scaffolded approved-app
• Node <version>  yarn <version>
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

➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0016: │ @oxc-project/types@npm:=0.141.0: All versions satisfying "=0.141.0" are quarantined
➤ YN0000: └ Completed
➤ YN0000: · Failed with errors in <duration> <duration>

You may need to run "vp install" manually in <workspace>/default-app
◇ Scaffolded default-app
• Node <version>  yarn <version>
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

# create_approve_builds_bun

## `vp create @your-org:with-build-dep --no-interactive --approve-builds --package-manager bun --directory approved-app`

--approve-builds runs `bun pm trust` for the gated build script (core-js)

```
◇ Scaffolded approved-app
• Node <version>  bun <version>
✓ Dependencies installed in <duration>
→ Next: cd approved-app && vp run
```

## `vpt print-file approved-app/package.json`

core-js recorded under trustedDependencies

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
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vite-plus": "<version>"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  },
  "devEngines": {
    "packageManager": {
      "name": "bun",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "trustedDependencies": [
    "core-js"
  ]
}
```

## `vp create @your-org:with-build-dep --no-interactive --package-manager bun --directory default-app`

default run surfaces the gated build with guidance, leaving it untrusted

```

Build scripts were not run for: core-js.

These dependencies may not work until built. Run vp pm approve-builds core-js in the project to approve them, or re-create with --approve-builds.
◇ Scaffolded default-app
• Node <version>  bun <version>
✓ Dependencies installed in <duration>
→ Next: cd default-app && vp run
```

## `vpt print-file default-app/package.json`

no trustedDependencies, the build was not run

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
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vite-plus": "<version>"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  },
  "devEngines": {
    "packageManager": {
      "name": "bun",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `cd default-app && vp pm approve-builds core-js`

the guidance's `vp pm approve-builds` command approves the gated build

```
bun pm trust <version> (0d9b296a)

./node_modules/core-js @3.39.0
 ✓ [postinstall]: node -e "try{require('./postinstall')}catch(e){}"

 1 script ran across 1 package [<duration>]
```

## `vpt print-file default-app/package.json`

core-js is now recorded under trustedDependencies

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
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vite-plus": "<version>"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  },
  "devEngines": {
    "packageManager": {
      "name": "bun",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "trustedDependencies": [
    "core-js"
  ]
}
```

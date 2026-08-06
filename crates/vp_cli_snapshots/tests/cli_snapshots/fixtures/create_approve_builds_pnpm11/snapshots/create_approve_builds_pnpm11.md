# create_approve_builds_pnpm11

## `vp create @your-org:with-build-dep --no-interactive --approve-builds --directory approved-app`

--approve-builds auto-approves and runs the gated build script (core-js)

```
◇ Scaffolded approved-app
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
→ Next: cd approved-app && vp run
```

## `vpt print-file approved-app/pnpm-workspace.yaml`

approval recorded under allowBuilds

```
allowBuilds:
  core-js: true
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
overrides:
  vite: "catalog:"
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: "*"
```

## `vp create @your-org:with-build-dep --no-interactive --directory default-app`

default run surfaces the gated build with guidance, leaving it unapproved

```

Build scripts were not run for: core-js.

These dependencies may not work until built. Run vp pm approve-builds in the project to approve them, or re-create with --approve-builds.
◇ Scaffolded default-app
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
→ Next: cd default-app && vp run
```

## `vpt print-file default-app/pnpm-workspace.yaml`

no allowBuilds, the build was not run

```
allowBuilds:
  core-js: set this to true or false
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
overrides:
  vite: "catalog:"
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: "*"
```

## `cd default-app && vp pm approve-builds core-js`

the guidance's `vp pm approve-builds` command approves the gated build

```
node_modules/.pnpm/core-js@3.39.0/node_modules/core-js: Running postinstall script, done in <duration>
```

## `vpt print-file default-app/pnpm-workspace.yaml`

core-js is now allowed under allowBuilds

```
allowBuilds:
  core-js: true
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
overrides:
  vite: "catalog:"
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: "*"
```

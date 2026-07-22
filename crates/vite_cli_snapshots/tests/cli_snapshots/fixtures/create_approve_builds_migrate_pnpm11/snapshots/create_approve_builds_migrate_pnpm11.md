# create_approve_builds_migrate_pnpm11

## `vp create @your-org:with-build-dep --no-interactive --approve-builds --directory approved-app`

template ships Prettier, so create installs+migrates before the main install; the gated build (core-js) must still be surfaced and approved

```

Prettier detected in workspace packages but no root config found. Package-level Prettier must be migrated manually.
◇ Scaffolded approved-app
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
→ Next: cd approved-app && vp run
```

## `vpt print-file approved-app/pnpm-workspace.yaml`

approval recorded under allowBuilds despite the migration pre-install

```
allowBuilds:
  core-js: true
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
minimumReleaseAgeExclude:
  - "@oxc-project/runtime@0.141.0"
  - "@oxc-project/types@0.141.0"
  - "@oxfmt/binding-android-arm-eabi@0.60.0"
  - "@oxfmt/binding-android-arm64@0.60.0"
  - "@oxfmt/binding-darwin-arm64@0.60.0"
  - "@oxfmt/binding-darwin-x64@0.60.0"
  - "@oxfmt/binding-freebsd-x64@0.60.0"
  - "@oxfmt/binding-linux-arm-gnueabihf@0.60.0"
  - "@oxfmt/binding-linux-arm-musleabihf@0.60.0"
  - "@oxfmt/binding-linux-arm64-gnu@0.60.0"
  - "@oxfmt/binding-linux-arm64-musl@0.60.0"
  - "@oxfmt/binding-linux-ppc64-gnu@0.60.0"
  - "@oxfmt/binding-linux-riscv64-gnu@0.60.0"
  - "@oxfmt/binding-linux-riscv64-musl@0.60.0"
  - "@oxfmt/binding-linux-s390x-gnu@0.60.0"
  - "@oxfmt/binding-linux-x64-gnu@0.60.0"
  - "@oxfmt/binding-linux-x64-musl@0.60.0"
  - "@oxfmt/binding-openharmony-arm64@0.60.0"
  - "@oxfmt/binding-win32-arm64-msvc@0.60.0"
  - "@oxfmt/binding-win32-ia32-msvc@0.60.0"
  - "@oxfmt/binding-win32-x64-msvc@0.60.0"
  - "@oxlint-tsgolint/darwin-arm64@7.0.2001"
  - "@oxlint-tsgolint/darwin-x64@7.0.2001"
  - "@oxlint-tsgolint/linux-arm64@7.0.2001"
  - "@oxlint-tsgolint/linux-x64@7.0.2001"
  - "@oxlint-tsgolint/win32-arm64@7.0.2001"
  - "@oxlint-tsgolint/win32-x64@7.0.2001"
  - "@oxlint/binding-android-arm-eabi@1.75.0"
  - "@oxlint/binding-android-arm64@1.75.0"
  - "@oxlint/binding-darwin-arm64@1.75.0"
  - "@oxlint/binding-darwin-x64@1.75.0"
  - "@oxlint/binding-freebsd-x64@1.75.0"
  - "@oxlint/binding-linux-arm-gnueabihf@1.75.0"
  - "@oxlint/binding-linux-arm-musleabihf@1.75.0"
  - "@oxlint/binding-linux-arm64-gnu@1.75.0"
  - "@oxlint/binding-linux-arm64-musl@1.75.0"
  - "@oxlint/binding-linux-ppc64-gnu@1.75.0"
  - "@oxlint/binding-linux-riscv64-gnu@1.75.0"
  - "@oxlint/binding-linux-riscv64-musl@1.75.0"
  - "@oxlint/binding-linux-s390x-gnu@1.75.0"
  - "@oxlint/binding-linux-x64-gnu@1.75.0"
  - "@oxlint/binding-linux-x64-musl@1.75.0"
  - "@oxlint/binding-openharmony-arm64@1.75.0"
  - "@oxlint/binding-win32-arm64-msvc@1.75.0"
  - "@oxlint/binding-win32-ia32-msvc@1.75.0"
  - "@oxlint/binding-win32-x64-msvc@1.75.0"
  - oxfmt@0.60.0
  - oxlint-tsgolint@7.0.2001
  - oxlint@1.75.0
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

Prettier detected in workspace packages but no root config found. Package-level Prettier must be migrated manually.

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
minimumReleaseAgeExclude:
  - "@oxc-project/runtime@0.141.0"
  - "@oxc-project/types@0.141.0"
  - "@oxfmt/binding-android-arm-eabi@0.60.0"
  - "@oxfmt/binding-android-arm64@0.60.0"
  - "@oxfmt/binding-darwin-arm64@0.60.0"
  - "@oxfmt/binding-darwin-x64@0.60.0"
  - "@oxfmt/binding-freebsd-x64@0.60.0"
  - "@oxfmt/binding-linux-arm-gnueabihf@0.60.0"
  - "@oxfmt/binding-linux-arm-musleabihf@0.60.0"
  - "@oxfmt/binding-linux-arm64-gnu@0.60.0"
  - "@oxfmt/binding-linux-arm64-musl@0.60.0"
  - "@oxfmt/binding-linux-ppc64-gnu@0.60.0"
  - "@oxfmt/binding-linux-riscv64-gnu@0.60.0"
  - "@oxfmt/binding-linux-riscv64-musl@0.60.0"
  - "@oxfmt/binding-linux-s390x-gnu@0.60.0"
  - "@oxfmt/binding-linux-x64-gnu@0.60.0"
  - "@oxfmt/binding-linux-x64-musl@0.60.0"
  - "@oxfmt/binding-openharmony-arm64@0.60.0"
  - "@oxfmt/binding-win32-arm64-msvc@0.60.0"
  - "@oxfmt/binding-win32-ia32-msvc@0.60.0"
  - "@oxfmt/binding-win32-x64-msvc@0.60.0"
  - "@oxlint-tsgolint/darwin-arm64@7.0.2001"
  - "@oxlint-tsgolint/darwin-x64@7.0.2001"
  - "@oxlint-tsgolint/linux-arm64@7.0.2001"
  - "@oxlint-tsgolint/linux-x64@7.0.2001"
  - "@oxlint-tsgolint/win32-arm64@7.0.2001"
  - "@oxlint-tsgolint/win32-x64@7.0.2001"
  - "@oxlint/binding-android-arm-eabi@1.75.0"
  - "@oxlint/binding-android-arm64@1.75.0"
  - "@oxlint/binding-darwin-arm64@1.75.0"
  - "@oxlint/binding-darwin-x64@1.75.0"
  - "@oxlint/binding-freebsd-x64@1.75.0"
  - "@oxlint/binding-linux-arm-gnueabihf@1.75.0"
  - "@oxlint/binding-linux-arm-musleabihf@1.75.0"
  - "@oxlint/binding-linux-arm64-gnu@1.75.0"
  - "@oxlint/binding-linux-arm64-musl@1.75.0"
  - "@oxlint/binding-linux-ppc64-gnu@1.75.0"
  - "@oxlint/binding-linux-riscv64-gnu@1.75.0"
  - "@oxlint/binding-linux-riscv64-musl@1.75.0"
  - "@oxlint/binding-linux-s390x-gnu@1.75.0"
  - "@oxlint/binding-linux-x64-gnu@1.75.0"
  - "@oxlint/binding-linux-x64-musl@1.75.0"
  - "@oxlint/binding-openharmony-arm64@1.75.0"
  - "@oxlint/binding-win32-arm64-msvc@1.75.0"
  - "@oxlint/binding-win32-ia32-msvc@1.75.0"
  - "@oxlint/binding-win32-x64-msvc@1.75.0"
  - oxfmt@0.60.0
  - oxlint-tsgolint@7.0.2001
  - oxlint@1.75.0
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
minimumReleaseAgeExclude:
  - "@oxc-project/runtime@0.141.0"
  - "@oxc-project/types@0.141.0"
  - "@oxfmt/binding-android-arm-eabi@0.60.0"
  - "@oxfmt/binding-android-arm64@0.60.0"
  - "@oxfmt/binding-darwin-arm64@0.60.0"
  - "@oxfmt/binding-darwin-x64@0.60.0"
  - "@oxfmt/binding-freebsd-x64@0.60.0"
  - "@oxfmt/binding-linux-arm-gnueabihf@0.60.0"
  - "@oxfmt/binding-linux-arm-musleabihf@0.60.0"
  - "@oxfmt/binding-linux-arm64-gnu@0.60.0"
  - "@oxfmt/binding-linux-arm64-musl@0.60.0"
  - "@oxfmt/binding-linux-ppc64-gnu@0.60.0"
  - "@oxfmt/binding-linux-riscv64-gnu@0.60.0"
  - "@oxfmt/binding-linux-riscv64-musl@0.60.0"
  - "@oxfmt/binding-linux-s390x-gnu@0.60.0"
  - "@oxfmt/binding-linux-x64-gnu@0.60.0"
  - "@oxfmt/binding-linux-x64-musl@0.60.0"
  - "@oxfmt/binding-openharmony-arm64@0.60.0"
  - "@oxfmt/binding-win32-arm64-msvc@0.60.0"
  - "@oxfmt/binding-win32-ia32-msvc@0.60.0"
  - "@oxfmt/binding-win32-x64-msvc@0.60.0"
  - "@oxlint-tsgolint/darwin-arm64@7.0.2001"
  - "@oxlint-tsgolint/darwin-x64@7.0.2001"
  - "@oxlint-tsgolint/linux-arm64@7.0.2001"
  - "@oxlint-tsgolint/linux-x64@7.0.2001"
  - "@oxlint-tsgolint/win32-arm64@7.0.2001"
  - "@oxlint-tsgolint/win32-x64@7.0.2001"
  - "@oxlint/binding-android-arm-eabi@1.75.0"
  - "@oxlint/binding-android-arm64@1.75.0"
  - "@oxlint/binding-darwin-arm64@1.75.0"
  - "@oxlint/binding-darwin-x64@1.75.0"
  - "@oxlint/binding-freebsd-x64@1.75.0"
  - "@oxlint/binding-linux-arm-gnueabihf@1.75.0"
  - "@oxlint/binding-linux-arm-musleabihf@1.75.0"
  - "@oxlint/binding-linux-arm64-gnu@1.75.0"
  - "@oxlint/binding-linux-arm64-musl@1.75.0"
  - "@oxlint/binding-linux-ppc64-gnu@1.75.0"
  - "@oxlint/binding-linux-riscv64-gnu@1.75.0"
  - "@oxlint/binding-linux-riscv64-musl@1.75.0"
  - "@oxlint/binding-linux-s390x-gnu@1.75.0"
  - "@oxlint/binding-linux-x64-gnu@1.75.0"
  - "@oxlint/binding-linux-x64-musl@1.75.0"
  - "@oxlint/binding-openharmony-arm64@1.75.0"
  - "@oxlint/binding-win32-arm64-msvc@1.75.0"
  - "@oxlint/binding-win32-ia32-msvc@1.75.0"
  - "@oxlint/binding-win32-x64-msvc@1.75.0"
  - oxfmt@0.60.0
  - oxlint-tsgolint@7.0.2001
  - oxlint@1.75.0
overrides:
  vite: "catalog:"
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: "*"
```

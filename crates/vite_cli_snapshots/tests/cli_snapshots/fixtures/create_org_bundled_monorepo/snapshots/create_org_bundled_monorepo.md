# create_org_bundled_monorepo

## `vp create @your-org:workspace --no-interactive --directory my-mono --git`

bundled monorepo: extract tarball, scaffold, inject create.defaultTemplate

```
◇ Scaffolded my-mono
• Node <version>  pnpm <version>
→ Next: cd my-mono && vp run
```

## `vpt print-file my-mono/vite.config.ts`

create.defaultTemplate auto-set to @your-org

```
import { defineConfig } from "vite-plus";

export default defineConfig({
  staged: {
    "*": "vp check --fix",
  },
  create: { defaultTemplate: "@your-org" },
  fmt: {},
  lint: {
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: { "vite-plus/prefer-vite-plus-imports": "error" },
    options: { typeAware: true, typeCheck: true },
  },
  run: { cache: true },
});
```

## `vpt print-file my-mono/pnpm-workspace.yaml`

workspace markers preserved

```
packages:
  - apps/*
  - packages/*
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

## `vpt stat-file my-mono/.git --assert dir`

git-init prompt covers bundled monorepo path

```
my-mono/.git: dir
```

## `vpt print-file my-mono/.gitignore`

node_modules excluded even though tarball shipped no .gitignore

```
node_modules

# dotenv environment variable files
.env
.env.*
!.env.example
```

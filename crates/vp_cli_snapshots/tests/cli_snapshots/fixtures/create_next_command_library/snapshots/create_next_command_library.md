# create_next_command_library

## `vp create vite:library --no-interactive`

verify next command suggests vp run

```
● Using default package name: vite-plus-library
◇ Scaffolded vite-plus-library with TypeScript library
• Node <version>  pnpm <version>
→ Next: cd vite-plus-library && vp run
```

## `vpt print-file vite-plus-library/pnpm-workspace.yaml`

standalone library workspace includes the root package

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
overrides:
  vite@*: "catalog:"
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: "*"
packages:
  - .
```

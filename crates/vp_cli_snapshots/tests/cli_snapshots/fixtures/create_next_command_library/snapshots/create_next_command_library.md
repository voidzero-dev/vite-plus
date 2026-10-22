# create_next_command_library

## `vp create vite:library --no-interactive`

verify next command suggests vp run

```
● Using default package name: vite-plus-library
◇ Scaffolded vite-plus-library with TypeScript library
• Node <version>  pnpm <version>
→ Next: cd vite-plus-library && vp run
```

## `vpt grep-file vite-plus-library/pnpm-workspace.yaml packages:`

standalone library workspace declares its root package

```
vite-plus-library/pnpm-workspace.yaml: found "packages:"
```

## `vpt grep-file vite-plus-library/pnpm-workspace.yaml '  - .'`

standalone library workspace includes the root package

```
vite-plus-library/pnpm-workspace.yaml: found "  - ."
```

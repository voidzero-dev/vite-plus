# create_org_bundled

## `vp create @your-org:demo --no-interactive --directory my-demo-app`

bundled template: extract tarball, copy subdir

```
◇ Scaffolded my-demo-app
• Node <version>  pnpm <version>
→ Next: cd my-demo-app && vp run
```

## `vpt print-file my-demo-app/package.json`

verify package.json name was rewritten

```
{
  "name": "my-demo-app",
  "version": "0.0.0",
  "scripts": {
    "dev": "vp dev",
    "prepare": "vp config"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vpt print-file my-demo-app/src/index.ts`

verify bundled source copied

```
export const name = "demo";
```

## `vpt list-dir my-demo-app/README.md`

verify README copied

```
my-demo-app/README.md
```

# create_generator_monorepo

Scaffolds a generator, installs its deps (so its `bin/index.ts` can import
`bingo`), then runs it through the registered `create.templates` entry. The
`vp install` step is what lets a scaffolded artifact run in the isolated
runner without the legacy symlink-all-node_modules behavior.

## `vp create vite:generator --no-interactive --directory tools/my-generator`

scaffold a generator; auto-registers it in create.templates

```
◇ Scaffolded tools/my-generator with generator scaffold
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
→ Next: cd tools/my-generator && vp run
```

## `vpt print-file vite.config.ts`

create.templates entry appended, existing defaultTemplate preserved

```
import { defineConfig } from "vite-plus";

export default defineConfig({
  create: {
    defaultTemplate: "@acme",
    templates: [
      {
        name: "my-generator",
        description: "A starter for creating a Vite+ code generator.",
        template: "./tools/my-generator",
      },
    ],
  },
});
```

## `vpt print-file tools/my-generator/package.json`

generator package (bingo dependency is the run hint; no marker keyword)

```
{
  "name": "my-generator",
  "version": "0.0.0",
  "private": true,
  "description": "A starter for creating a Vite+ code generator.",
  "keywords": [
    "vite-plus-generator"
  ],
  "bin": "./bin/index.ts",
  "type": "module",
  "scripts": {
    "test": "vp test",
    "dev": "node bin/index.ts"
  },
  "dependencies": {
    "bingo": "^0.9.3",
    "zod": "^3.25.76"
  },
  "devDependencies": {
    "@types/node": "catalog:",
    "typescript": "catalog:"
  },
  "engines": {
    "node": ">=22.18.0"
  }
}
```

## `vp install`

install workspace deps so the generator's bin can import bingo


## `vp create my-generator --no-interactive -- --name demo-pkg --directory demo-pkg --offline`

resolve via the registered create.templates entry

```

Generating project…

Running: node <workspace>/tools/my-generator/bin/index.ts --name demo-pkg --directory demo-pkg --offline --skip-requests
┌  my-generator@0.0.0 │
◇  Running with mode --setup
│
│  --offline enabled. You'll need to git push any changes manually.
│
◇  Inferred default options from system
│
◇  Ran the my-generator template
│
◇  Prepared local Git repository
│
●  Run npx index.ts --remote in ./demo-pkg
│  to create and sync a remote repository on GitHub.
│
└  Thanks for using my-generator! 💝

Monorepo integration...

Installing dependencies...

Dependencies installed

Formatting code...

Code formatted
◇ Scaffolded tools/demo-pkg
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
→ Next: cd tools/demo-pkg && vp run
```

## `vpt print-file tools/demo-pkg/package.json`

generated next to the generator under tools/, not the apps/ parent

```
{
  "name": "demo-pkg",
  "version": "0.0.0",
  "type": "module"
}
```

## `vpt print-file tools/demo-pkg/src/index.ts`

```
export const name = "demo-pkg";
```

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


## `vp exec node assert_noninteractive.mjs`

assert missing arguments fail without prompts, existing files survive, and both modes generate files


## `vp create my-generator --no-interactive -- --name demo-pkg`

missing directory fails without entering Bingo prompts

**Exit code:** 1

```

Generating project…

Running: node <workspace>/tools/my-generator/bin/index.ts --name demo-pkg --skip-requests
Missing --directory. Pass generator options after -- in vp create.
```

## `vp create my-generator --no-interactive -- --directory missing-name`

missing required template option fails before creating its directory

**Exit code:** 1

```

Generating project…

Running: node <workspace>/tools/my-generator/bin/index.ts --directory missing-name --skip-requests
[
  {
    "code": "invalid_type",
    "expected": "string",
    "received": "undefined",
    "path": [
      "name"
    ],
    "message": "Required"
  }
]
```

## `vpt stat-file tools/missing-name --assert missing`

```
tools/missing-name: missing
```

## `vp create my-generator --no-interactive -- --name demo-pkg --directory demo-pkg --offline`

resolve via the registered create.templates entry

```

Generating project…

Running: node <workspace>/tools/my-generator/bin/index.ts --name demo-pkg --directory demo-pkg --offline --skip-requests

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

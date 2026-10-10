# create_generator_interactive_default_description

Submitting the generator description prompt without typing preserves its default.

## `vp create vite:generator --directory tools/my-generator --no-agent --no-editor`

**→ expect-milestone:** `text:text:`

```
VITE+ - The Unified Toolchain for the Web

› Description:
  Generate new components for our monorepo
```

**← write-key:** `enter`

```
VITE+ - The Unified Toolchain for the Web

◇ Description:
  Generate new components for our monorepo

◇ Scaffolded tools/my-generator with generator scaffold
• Node <version>  pnpm <version>
→ Next: cd tools/my-generator && vp run
```

## `vpt print-file tools/my-generator/package.json`

```
{
  "name": "my-generator",
  "version": "0.0.0",
  "private": true,
  "description": "Generate new components for our monorepo",
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

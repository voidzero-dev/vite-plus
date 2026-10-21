# create_generator_bun_no_catalog

## `vpt json-edit package.json packageManager bun@1.4.0`


## `vp create vite:generator --no-interactive --directory tools/my-generator --no-agent --no-editor`


## `vpt print-file tools/my-generator/package.json`

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
    "bingo": "^0.13.2",
    "zod": "^4.4.3"
  },
  "devDependencies": {
    "@types/node": "^24",
    "typescript": "^7.0.0"
  },
  "engines": {
    "node": ">=22.18.0"
  }
}
```

## `vp install`


## `vp create my-generator --no-interactive --no-agent --no-editor -- --directory generated --name generated --offline`


## `vp check tools/generated`

```
pass: All 3 files are correctly formatted (<duration>, <n> threads)
pass: Found no warnings, lint errors, or type errors in 1 file (<duration>, <n> threads)
```

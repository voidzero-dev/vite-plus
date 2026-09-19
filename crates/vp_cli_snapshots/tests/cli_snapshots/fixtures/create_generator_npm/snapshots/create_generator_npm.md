# create_generator_npm

A generator added to an npm workspace uses installable dependency ranges instead of catalog references.

## `vp create vite:generator --no-interactive --directory tools/my-generator`


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
    "bingo": "^0.9.3",
    "zod": "^3.25.76"
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


## `node tools/my-generator/bin/index.ts --help`


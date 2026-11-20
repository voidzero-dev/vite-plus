# migration_zed_settings

## `vp migrate --no-interactive --no-hooks --no-agent --editor zed`


## `vpt print-file vite.config.ts`

Migration preserves the project's lint options.

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
  lint: {
    options: {
      typeAware: false,
      typeCheck: false,
      reportUnusedDisableDirectives: 'warn',
    },
  },
});
```

## `node --input-type=module -e 'import fs from '\''node:fs'\''; const settings = JSON.parse(fs.readFileSync('\''.zed/settings.json'\'', '\''utf8'\'')); console.log(JSON.stringify(settings.lsp, null, 2));'`

Generated Zed settings leave typeAware and unusedDisableDirectives unset so Oxlint uses the project config.

```
{
  "oxlint": {
    "initialization_options": {
      "settings": {
        "run": "onType",
        "fixKind": "safe_fix"
      }
    }
  },
  "oxfmt": {
    "initialization_options": {
      "settings": {
        "fmt.configPath": "./vite.config.ts",
        "run": "onSave"
      }
    }
  }
}
```

## `vpt json-edit .zed/settings.json lsp.oxlint.initialization_options.settings.typeAware true`


## `vpt json-edit .zed/settings.json lsp.oxlint.initialization_options.settings.unusedDisableDirectives deny`


## `vp migrate --full --no-interactive --no-hooks --no-agent --editor zed`


## `node --input-type=module -e 'import fs from '\''node:fs'\''; const settings = JSON.parse(fs.readFileSync('\''.zed/settings.json'\'', '\''utf8'\'')); console.log(JSON.stringify(settings.lsp.oxlint, null, 2));'`

A later migration preserves explicit editor overrides.

```
{
  "initialization_options": {
    "settings": {
      "fixKind": "safe_fix",
      "run": "onType",
      "typeAware": true,
      "unusedDisableDirectives": "deny"
    }
  }
}
```

# migration_preserve_editor_jsonc_comments

## `vp migrate --no-interactive --no-hooks --editor vscode`

merge must preserve existing .vscode JSONC comments

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
```

## `vpt print-file .vscode/settings.json`

top-level and nested comments survive; oxc settings are added without overwriting existing values

```
{
  // Use the project's typescript version
  "typescript.tsdk": "node_modules/typescript/lib",
  "editor.codeActionsOnSave": {
    // keep my organize imports
    "source.organizeImports": "explicit",
    "source.fixAll.oxc": "explicit",
  },
  "editor.defaultFormatter": "oxc.oxc-vscode",
  "[javascript]": {
    "editor.defaultFormatter": "oxc.oxc-vscode"
  },
  "[javascriptreact]": {
    "editor.defaultFormatter": "oxc.oxc-vscode"
  },
  "[typescript]": {
    "editor.defaultFormatter": "oxc.oxc-vscode"
  },
  "[typescriptreact]": {
    "editor.defaultFormatter": "oxc.oxc-vscode"
  },
  "oxc.fmt.configPath": "./vite.config.ts",
  "editor.formatOnSave": true,
  "editor.formatOnSaveMode": "file",
}
```

## `vpt print-file .vscode/extensions.json`

existing recommendation and comment stay; vite-plus extension is appended once

```
{
  "recommendations": [
    // keep my favorite extension
    "dbaeumer.vscode-eslint",
    "VoidZero.vite-plus-extension-pack",
  ],
}
```

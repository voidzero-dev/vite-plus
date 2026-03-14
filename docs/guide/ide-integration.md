# IDE Integration

Vite+ supports VS Code through the [Vite Plus Extension Pack](https://marketplace.visualstudio.com/items?itemName=VoidZero.vite-plus-extension-pack) and the VS Code settings that `vp create` and `vp migrate` can automatically write into your project.

## VS Code

For the best VS Code experience with Vite+, install the [Vite Plus Extension Pack](https://marketplace.visualstudio.com/items?itemName=VoidZero.vite-plus-extension-pack). It currently includes:

- `Oxc` for formatting and linting via `vp check`
- `Vitest` for test runs via `vp test`

When you create or migrate a project, Vite+ prompts whether you want editor config written for VS Code. You can also manually set up the VS Code config:

`.vscode/extensions.json`

```json
{
  "recommendations": ["VoidZero.vite-plus-extension-pack"]
}
```

`.vscode/settings.json`

```json
{
  "editor.defaultFormatter": "oxc.oxc-vscode",
  "oxc.fmt.configPath": "./vite.config.ts",
  "editor.formatOnSave": true,
  "editor.formatOnSaveMode": "file",
  "editor.codeActionsOnSave": {
    "source.fixAll.oxc": "explicit"
  }
}
```

This gives the project a shared default formatter and enables Oxc-powered fix actions on save. Setting `oxc.fmt.configPath` to `./vite.config.ts` keeps editor format-on-save aligned with the `fmt` block in your Vite+ config. Vite+ uses `formatOnSaveMode: "file"` because Oxfmt does not support partial formatting.

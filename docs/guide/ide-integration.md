# IDE Integration

Vite+ supports VS Code and Zed through editor-specific settings that `vp create` and `vp migrate` can automatically write into your project.

## VS Code

For the best VS Code experience with Vite+, install the [Vite Plus Extension Pack](https://marketplace.visualstudio.com/items?itemName=VoidZero.vite-plus-extension-pack). It currently includes:

- `Oxc` for formatting and linting via `vp check`
- `Vitest` for test runs via `vp test`

When you create or migrate a project, Vite+ prompts whether you want editor config written for VS Code. `vp create` additionally sets `npm.scriptRunner` to `vp` so the VS Code NPM Scripts panel runs scripts through the Vite+ task runner. For migrated or existing projects, you can add this setting manually (see below).

You can also manually set up the VS Code config:

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

To let the VS Code NPM Scripts panel run scripts through `vp`, add the following to your `.vscode/settings.json`:

```json
{
  "npm.scriptRunner": "vp"
}
```

This is included automatically by `vp create` but not by `vp migrate`, since existing projects may have team members who do not have `vp` installed locally.

## Zed

For the best Zed experience with Vite+, install the [oxc-zed](https://github.com/oxc-project/oxc-zed) extension from the Zed extensions marketplace. It provides formatting and linting via `vp check`.

When you create or migrate a project, Vite+ prompts you to choose whether you want the editor config written for Zed.

You can also manually set up the Zed config:

`.zed/settings.json`

```json
{
  "lsp": {
    "oxlint": {
      "initialization_options": {
        "settings": {
          "run": "onType",
          "fixKind": "safe_fix",
          "typeAware": true,
          "unusedDisableDirectives": "deny"
        }
      }
    },
    "oxfmt": {
      "initialization_options": {
        "settings": {
          "configPath": "./vite.config.ts",
          "run": "onSave"
        }
      }
    }
  },
  "languages": {
    "JavaScript": {
      "format_on_save": "on",
      "prettier": { "allowed": false },
      "formatter": [{ "language_server": { "name": "oxfmt" } }],
      "code_action": "source.fixAll.oxc"
    },
    "TypeScript": {
      "format_on_save": "on",
      "prettier": { "allowed": false },
      "formatter": [{ "language_server": { "name": "oxfmt" } }]
    },
    "Vue.js": {
      "format_on_save": "on",
      "prettier": { "allowed": false },
      "formatter": [{ "language_server": { "name": "oxfmt" } }]
    }
  }
}
```

Setting `oxfmt.configPath` to `./vite.config.ts` keeps editor format-on-save aligned with the `fmt` block in your Vite+ config. The full generated config covers additional languages (CSS, HTML, JSON, Markdown, etc.) — run `vp create` or `vp migrate` to get the complete file written automatically.

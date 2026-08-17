# IDE Integration

Vite+ supports VS Code and Zed through editor-specific settings that `vp create` and `vp migrate` can automatically write into your project.

## VS Code

For the best VS Code experience with Vite+, install the [Vite Plus Extension Pack](https://marketplace.visualstudio.com/items?itemName=VoidZero.vite-plus-extension-pack). It currently includes:

- `Oxc` for formatting and linting via `vp check`
- `Vitest` for test runs via `vp test`

When you create or migrate a project, Vite+ prompts whether you want editor config written for VS Code. `vp create` additionally sets `npm.scriptRunner` to `vp` so the VS Code NPM Scripts panel runs scripts through the Vite+ task runner. For migrated or existing projects, you can add this setting manually (see below).

You can also manually set up the VS Code config:

```json [.vscode/extensions.json]
{
  "recommendations": ["VoidZero.vite-plus-extension-pack"]
}
```

```json [.vscode/settings.json]
{
  "editor.defaultFormatter": "oxc.oxc-vscode",
  "[javascript]": { "editor.defaultFormatter": "oxc.oxc-vscode" },
  "[javascriptreact]": { "editor.defaultFormatter": "oxc.oxc-vscode" },
  "[typescript]": { "editor.defaultFormatter": "oxc.oxc-vscode" },
  "[typescriptreact]": { "editor.defaultFormatter": "oxc.oxc-vscode" },
  "oxc.disableNestedConfig": true,
  "oxc.fmt.disableNestedConfig": true,
  "editor.formatOnSave": true,
  "editor.formatOnSaveMode": "file",
  "editor.codeActionsOnSave": {
    "source.fixAll.oxc": "explicit"
  }
}
```

This gives the project a shared default formatter and enables Oxc-powered fix actions on save. The language-specific override blocks (`[javascript]`, `[typescript]`, etc.) are required because VS Code prioritizes user-level `[language]` settings over the workspace-level `editor.defaultFormatter` — without them, a global Prettier configuration would silently take over. Setting `oxc.disableNestedConfig` and `oxc.fmt.disableNestedConfig` prevents nested Oxlint and Oxfmt configs from diverging from the root Vite+ config. Vite+ uses `formatOnSaveMode: "file"` because Oxfmt does not support partial formatting.

To let the VS Code NPM Scripts panel run scripts through `vp`, add the following to your `.vscode/settings.json`:

```json [.vscode/settings.json]
{
  "npm.scriptRunner": "vp"
}
```

This is included automatically by `vp create` but not by `vp migrate`, since existing projects may have team members who do not have `vp` installed locally.

## Zed

For the best Zed experience with Vite+, install the [oxc-zed](https://github.com/oxc-project/oxc-zed) extension from the Zed extensions marketplace. It provides formatting and linting via `vp check`.

When you create or migrate a project, Vite+ prompts you to choose whether you want the editor config written for Zed.

You can also manually set up the Zed config:

```json [.zed/settings.json]
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
          "fmt.configPath": "./vite.config.ts",
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

Setting `oxfmt.fmt.configPath` to `./vite.config.ts` keeps editor format-on-save aligned with the `fmt` block in your Vite+ config. The full generated config covers additional languages (CSS, HTML, JSON, Markdown, etc.) — run `vp create` or `vp migrate` to get the complete file written automatically.

## JetBrains (IntelliJ, WebStorm, etc...)

For the best Vite+ experience with JetBrains IDEs such as IntelliJ & WebStorm, install the [Oxc](https://plugins.jetbrains.com/plugin/27061-oxc) plugin from the JetBrains marketplace.

When you create or migrate a project, Vite+ prompts you to choose whether you want the editor config written for JetBrains IDEs.

::: tip Vite+ does not merge with existing config files
Due to some complexities with merging XML files, Vite+ currently does not merge your current files if the files already exist.
You'll be given the opportunity to replace any existing files, instead of merging.
:::

You can also manually set up the IDE configuration to match your Vite+ setup:

```xml [.idea/externalDependencies.xml]
<?xml version="1.0" encoding="UTF-8"?>
<project version="4">
  <component name="ExternalDependencies">
    <plugin id="com.github.oxc.project.oxcintellijplugin" />
  </component>
</project>
```

```xml [.idea/workspace.xml]
<?xml version="1.0" encoding="UTF-8"?>
<project version="4">
  <!-- other settings... -->
  <component name="PropertiesComponent">
    <![CDATA[{
      "keyToString": {
        // other settings
        "javascript.nodejs.core.library.configured.version": "24.18.0", // Replace with your selected Node.js version
        "javascript.nodejs.core.library.typings.version": "24.13.3", // Replace with the version of @types/node that corresponds to your runtime (or omit if you don't want it)
        "javascript.preferred.runtime.type.id": "node",
        "nodejs_interpreter_path": "$USER_HOME$/.vite-plus/bin/node",
        "nodejs_package_manager_path": "pnpm" // Replace with your package manager of choice
      }
    }]]>
  </component>
</project>
```

```xml [.idea/OxfmtSettings.xml]
<?xml version="1.0" encoding="UTF-8"?>
<project version="4">
  <component name="OxfmtSettings">
    <option name="preferOxfmtCodeStyleSettings" value="true" />
  </component>
</project>
```

Often, `.idea` folders are gitignored in a project, even the `externalDependencies.xml` file, which is used to tell your IDE what plugins should be used for the workspace.

Please make sure to add this line to your main `.gitignore` file, to ensure that the file will be included:

```gitignore [.gitignore]
!.idea/externalDependencies.xml
```

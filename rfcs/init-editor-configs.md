# RFC: Init Editor Configs

## Summary

Add editor configuration file generation (starting with VSCode) to `vp create` and `vp migrate` flows.

This follows the same pattern as agent instructions (`--agent` / `--no-agent`), providing `--editor` / `--no-editor` options.

## Motivation

IDE configs such as VSCode require complicated JSON that users have to manually set up.
Since Vite+ uses Oxc as its formatter, projects benefit from having:

- `.vscode/settings.json` — Oxc as default formatter, format on save, etc.
- `.vscode/extensions.json` — Recommended extensions (oxc-vscode)

Currently, users must create these files manually.
Vite+ should generate them automatically during project creation and migration, just like it already does for agent instructions.

## Command Syntax

```bash
# Create with editor config
vp create vite:application --editor vscode

# Migrate with editor config
vp migrate --editor vscode

# Skip editor config (migrate only)
vp migrate --no-editor
```

In interactive mode, users are prompted to select their editor (None / VSCode) after the agent selection prompt.

## Generated Files

### `.vscode/settings.json`

Based on [oxc-vscode's own `.vscode/settings.json`](https://github.com/oxc-project/oxc-vscode/blob/main/.vscode/settings.json).

```json
{
  "editor.defaultFormatter": "oxc.oxc-vscode",
  "editor.formatOnSave": true,
  "editor.formatOnSaveMode": "file",
  "editor.codeActionsOnSave": {
    "source.fixAll.oxc": "explicit"
  },
  "oxc.typeAware": true
}
```

### `.vscode/extensions.json`

```json
{
  "recommendations": ["oxc.oxc-vscode"]
}
```

## Behavior

### Existing file handling

When a config file already exists:

- **Interactive mode**: Prompt with Merge / Skip options
  - Merge: Add new keys without overwriting existing user settings. For `extensions.json`, deduplicate recommendations array.
  - Skip: Leave unchanged
- **Non-interactive mode**: Merge automatically (safe because existing keys are never overwritten)

### Non-interactive defaults

- `--editor vscode`: Write configs
- `--no-editor`: Skip
- Neither specified: Skip (conservative default)

## Implementation Architecture

### New file: `packages/cli/src/utils/editor.ts`

Mirrors `packages/cli/src/utils/agent.ts` structure:

| agent.ts                          | editor.ts                |
| --------------------------------- | ------------------------ |
| `AGENTS` array                    | `EDITORS` array          |
| `selectAgentTargetPath()`         | `selectEditor()`         |
| `detectExistingAgentTargetPath()` | `detectExistingEditor()` |
| `writeAgentInstructions()`        | `writeEditorConfigs()`   |

Key difference from agent.ts: Uses JSON merge (via `utils/json.ts`) instead of file copy/append, since IDE configs are structured JSON.

### Integration into `create/bin.ts`

- Add `editor?: string` to `Options` interface
- Add `'editor'` to mri `string` array
- Add `--editor NAME` to help text
- Call `selectEditor()` and `writeEditorConfigs()` after agent instructions at each write site (monorepo path ~L535, single project path ~L588)

### Integration into `migration/bin.ts`

- Add `editor?: string | false` to `MigrationOptions` interface
- Add `--editor NAME` and `--no-editor` to help text
- Call `selectEditor()` and `writeEditorConfigs()` after agent instructions (~L225)

### Merge strategy

- `settings.json`: 2-level deep merge. Existing keys preserved, new keys added. Nested objects (e.g., `[typescript]`) also merged with existing keys preserved.
- `extensions.json`: `recommendations` array union with deduplication.

### Key files to modify

1. `packages/cli/src/utils/editor.ts` — New file, core logic
2. `packages/cli/src/create/bin.ts` — Add option and integration
3. `packages/cli/src/migration/bin.ts` — Add option and integration

### Reused utilities

- `packages/cli/src/utils/json.ts` — `readJsonFile`, `writeJsonFile`
- `@voidzero-dev/vite-plus-prompts` — `select`, `isCancel`, `log`

## Extensibility

The `EDITORS` array is designed to support additional editors in the future:

```typescript
export const EDITORS = [
  {
    id: 'vscode',
    label: 'VSCode',
    targetDir: '.vscode',
    files: ['settings.json', 'extensions.json'],
  },
  // Future: { id: 'jetbrains', label: 'JetBrains', targetDir: '.idea', files: [...] },
] as const;
```

## Snap Tests

Existing help-related snap tests will update automatically when help text changes. Dedicated snap tests can be added for:

- `migration-editor-vscode` — Verify `.vscode/` generation during migration
- `migration-no-editor` — Verify `--no-editor` skips generation

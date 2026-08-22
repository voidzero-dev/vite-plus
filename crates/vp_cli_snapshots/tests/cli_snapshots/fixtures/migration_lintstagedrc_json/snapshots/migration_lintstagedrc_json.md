# migration_lintstagedrc_json

## `vp migrate -h`

migration help message

```
VITE+ - The Unified Toolchain for the Web

Usage: vp migrate [PATH] [OPTIONS]

Migrate standalone Vite, Vitest, Oxlint, Oxfmt, and Prettier projects to unified Vite+.

Arguments:
  [PATH]  Target directory to migrate (default: current directory)

Options:
  --agent <NAME>    Write coding agent instructions to AGENTS.md, CLAUDE.md, etc.
  --no-agent        Skip writing coding agent instructions
  --editor <NAME>   Write editor config files into the project
  --no-editor       Skip writing editor config files
  --hooks           Set up pre-commit hooks (default in non-interactive mode)
  --no-hooks        Skip pre-commit hooks setup
  --interactive     Enable interactive prompts
  --no-interactive  Run in non-interactive mode (skip prompts and use defaults)
  --full            Also run the full setup for an existing Vite+ project
  -h, --help        Show this help message

Examples:
  vp migrate                    # Migrate the current package
  vp migrate my-app             # Migrate a directory
  vp migrate --no-interactive   # Use defaults without prompts

Migration Prompt:
  Give this to a coding agent when you want it to drive the migration:

  Migrate this project to Vite+.
  Vite+ replaces the split tools for runtime management, package management,
  development, builds, tests, linting, formatting, and packaging.
  Run `vp help` and `vp help migrate` before you make changes.
  Run `vp migrate --no-interactive` in the workspace root.
  Make sure that the project uses Vite 8+ and Vitest 4.1+.

  After the migration, check imports, configuration, and package aliases.
  Then run `vp install`, `vp check`, `vp test`, and `vp build`.
  Report all required manual work in the migration summary.

Documentation: https://viteplus.dev/guide/migrate
```

## `vp migrate --no-interactive`

migration work with lintstagedrc.json

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
! Warnings:
  - .lintstagedrc.json found but "staged" already exists in vite.config.ts — please merge manually
```

## `vpt print-file .lintstagedrc.json`

check lintstagedrc.json (should be deleted after inlining)

```
{
  "*.js": "oxlint --fix"
}
```

## `vpt print-file package.json`

check package.json

```
{
  "name": "migration-lintstagedrc",
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "scripts": {
    "prepare": "vp config"
  }
}
```

## `vpt print-file pnpm-workspace.yaml`

check pnpm-workspace.yaml has overrides and catalog

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
overrides:
  vite@*: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```

## `vpt print-file vite.config.ts`

check staged config migrated to vite.config.ts

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  staged: {
    "*.@(js|ts|tsx|yml|yaml|md|json|html|toml)": [
      "vp fmt --staged",
      "eslint --fix"
    ],
    "*.@(js|ts|tsx)": [
      "vp lint --fix"
    ]
  },
});
```

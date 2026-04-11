# RFC: Vite+ Migration Command

## Background

When transitioning to Vite+, projects typically use standalone tools like vite, oxlint, oxfmt, and vitest, each with their own dependencies and configuration files. The `vp migrate` command automates the process of consolidating these tools into the unified Vite+ toolchain.

**Problem**: Manual migration is error-prone and time-consuming:

- Multiple dependency entries to update in package.json
- Various configuration files to merge (vite.config.ts, .oxlintrc, .oxfmtrc, etc.)
- Risk of missing configurations or incorrect merging
- Tedious process when migrating multiple packages in a monorepo

**Solution**: Automated migration using [ast-grep](https://ast-grep.github.io/) for code transformation and [brush-parser](https://github.com/reubeno/brush) for shell script rewriting.

**Related Commands**:

- `vp create` - Uses this same migration engine after generating code (see [code-generator.md](./code-generator.md))
- `vp migrate` - This command, for migrating existing projects

## Goals

1. **Dependency Consolidation**: Replace standalone vite, vitest, oxlint, oxfmt dependencies with unified vite-plus
2. **Configuration Unification**: Merge .oxlintrc, .oxfmtrc into vite.config.ts
3. **Safe**: Preview changes before applying
4. **Intelligent**: Preserve custom configurations and user overrides
5. **Monorepo-Aware**: Migrate multiple packages efficiently

## Scope

**What this command migrates**:

- ✅ **Dependencies**: vite, vitest, oxlint, oxfmt → vite-plus
- ✅ **Overrides**: Force vite → vite-plus (for all dependencies)
  - pnpm (no existing `pnpm` config): Writes `overrides`, `peerDependencyRules`, and `catalog` to `pnpm-workspace.yaml`
  - pnpm (existing `pnpm` config): Adds `pnpm.overrides` and `pnpm.peerDependencyRules` in `package.json`
  - npm/bun: Adds `overrides.vite` mapping in `package.json`
  - yarn: Adds `resolutions.vite` mapping in `package.json`
  - **Benefit**: Code keeps `import from 'vite'` - automatically resolves to vite-plus
- ✅ **Configuration files**:
  - .oxlintrc → vite.config.ts (lint section)
  - .oxfmtrc → vite.config.ts (format section)
- ✅ **tsconfig.json cleanup**: Removes deprecated `esModuleInterop: false` (causes oxlint tsgolint errors)

**What this command optionally migrates** (prompted):

- ✅ **Git hooks**: husky + lint-staged → `vp config` + `vp staged`
  - Rewrites `prepare: "husky"` → `prepare: "vp config"`
  - Migrates lint-staged config into `staged` in vite.config.ts
  - Replaces `.husky/pre-commit` with `.vite-hooks/pre-commit` using `vp staged`
  - Removes `husky` and `lint-staged` from devDependencies
- ✅ **ESLint → oxlint** (via `@oxlint/migrate`): converts ESLint flat config to `.oxlintrc.json`, which is then merged into `vite.config.ts` by the existing flow
- ✅ **Prettier → oxfmt** (via `vp fmt --migrate=prettier`): converts Prettier config to `.oxfmtrc.json`, which is then merged into `vite.config.ts` by the existing flow

**What this command does NOT migrate**:

- ❌ Package.json scripts → vite-task.json (different feature)
- ❌ Build tool changes (webpack/rollup → vite)

These are **consolidation migrations**, not **feature migrations**.

### Re-migration

When a project already has `vite-plus` in its dependencies, `vp migrate` skips the full dependency/config migration and only runs remaining partial migrations:

- **ESLint → Oxlint**: If `eslint` is still present with a flat config, offers ESLint migration
- **Prettier → Oxfmt**: If `prettier` is still present with a config file, offers Prettier migration
- **Git hooks**: If `husky` and/or `lint-staged` are still present, offers hooks migration

All checks run independently — a project may need one, some, or none.

## Command Usage

```bash
vp migrate
```

## Migration Process

The migration uses a **two-phase architecture**: all user prompts are collected upfront (Phase 1), then all work is executed without interruption (Phase 2). This lets the user see the full picture before any changes begin.

### Phase 1: Collect User Decisions

All prompts are presented sequentially before any work begins:

1. **Confirm migration**: "Migrate this project to Vite+?"
2. **Package manager**: Select or auto-detect (pnpm/npm/yarn)
3. **Pre-commit hooks**: "Set up pre-commit hooks?" + preflight validation (read-only check for git root, existing hook tools)
4. **Agent selection**: "Which agents are you using?" (multiselect)
5. **Agent file conflicts**: Per existing file — "Agent instructions already exist at X. Append or Skip?" (only for files without auto-update markers)
6. **Editor selection**: "Which editor are you using?"
7. **Editor file conflicts**: Per existing file — "X already exists. Merge or Skip?"
8. **ESLint migration**: If ESLint config detected — "Migrate ESLint rules to Oxlint?"
9. **Prettier migration**: If Prettier config detected — "Migrate Prettier to Oxfmt?"
10. **Migration plan summary**: Display all planned actions before execution

In non-interactive mode (`--no-interactive`), Phase 1 uses defaults (no prompts shown, no summary displayed).

```bash
$ vp migrate

VITE+ - The Unified Toolchain for the Web

◆ Migrate this project to Vite+?
│ Yes

◆ Which package manager would you like to use?
│ pnpm (recommended)

◆ Set up pre-commit hooks?
│ Yes

◆ Which agents are you using?
│ Claude Code

◆ CLAUDE.md already exists.
│ Append

◆ Which editor are you using?
│ VSCode

◆ .vscode/settings.json already exists.
│ Merge

◆ Migrate ESLint rules to Oxlint using @oxlint/migrate?
│ Yes

◆ Migrate Prettier to Oxfmt?
│ Yes

Migration plan:
- Install pnpm and dependencies
- Rewrite configs and dependencies for Vite+
- Migrate ESLint rules to Oxlint
- Migrate Prettier to Oxfmt
- Set up pre-commit hooks
- Write agent instructions (CLAUDE.md, append)
- Write editor config (.vscode/, merge)
```

### Phase 2: Execute Without Prompts

All work runs sequentially with spinner feedback — no further user interaction:

1. **Download package manager** + version validation
2. **Upgrade yarn** if needed (yarn <4.10.0)
3. **Run `vp install`** to prepare dependencies
4. **Check vite/vitest versions** (abort if unsupported)
5. **Migrate ESLint → Oxlint** (if approved in Phase 1, via `@oxlint/migrate`)
   5b. **Migrate Prettier → Oxfmt** (if approved in Phase 1, via `vp fmt --migrate=prettier`)
6. **Rewrite configs** (dependencies, overrides, config file merging)
7. **Install git hooks** (if approved)
8. **Write agent instructions** (using pre-resolved conflict decisions)
9. **Write editor configs** (using pre-resolved conflict decisions)
10. **Reinstall dependencies** (final `vp install`)

```bash
pnpm@latest installing...
pnpm@<semver> installed
Migrating ESLint config to Oxlint...
ESLint config migrated to .oxlintrc.json
Replacing ESLint comments with Oxlint equivalents...
ESLint comments replaced
✔ Removed eslint.config.mjs
✔ Created vite.config.ts in vite.config.ts
✔ Merged .oxlintrc.json into vite.config.ts
✔ Merged staged config into vite.config.ts
Wrote agent instructions to AGENTS.md
✔ Migration completed!
```

## Migration Rules

### Package.json Dependencies & Overrides

**Before:**

```json
{
  "name": "my-package",
  "dependencies": {
    "react": "^18.2.0"
  },
  "devDependencies": {
    "vite": "^8.0.0",
    "vitest": "^4.0.0",
    "oxlint": "^0.1.0",
    "oxfmt": "^0.1.0",
    "@vitest/browser": "^4.0.0",
    "@vitest/browser-playwright": "^4.0.0",
    "@vitejs/plugin-react": "^4.2.0"
  }
}
```

**After (npm/bun) -- `package.json`:**

```json
{
  "name": "my-package",
  "dependencies": {
    "react": "^18.2.0"
  },
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@latest",
    "vitest": "npm:@voidzero-dev/vite-plus-test@latest",
    "@vitejs/plugin-react": "^4.2.0"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@latest",
    "vitest": "npm:@voidzero-dev/vite-plus-test@latest"
  }
}
```

**After (pnpm, no existing `pnpm` config) -- `package.json`:**

```json
{
  "name": "my-package",
  "dependencies": {
    "react": "^18.2.0"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vitest": "catalog:",
    "@vitejs/plugin-react": "^4.2.0",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@<semver>"
}
```

**After (pnpm, no existing `pnpm` config) -- `pnpm-workspace.yaml`:**

```yaml
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@latest
  vitest: npm:@voidzero-dev/vite-plus-test@latest
  vite-plus: latest
overrides:
  vite: 'catalog:'
  vitest: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
    - vitest
  allowedVersions:
    vite: '*'
    vitest: '*'
```

**After (pnpm, existing `pnpm` config) -- `package.json`:**

Projects that already have a `pnpm` field in `package.json` (e.g., with `overrides` or `onlyBuiltDependencies`) keep using `package.json` for pnpm config:

```json
{
  "name": "my-package",
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@latest",
    "vitest": "npm:@voidzero-dev/vite-plus-test@latest",
    "vite-plus": "latest"
  },
  "pnpm": {
    "overrides": {
      "vite": "npm:@voidzero-dev/vite-plus-core@latest",
      "vitest": "npm:@voidzero-dev/vite-plus-test@latest"
    },
    "peerDependencyRules": {
      "allowAny": ["vite", "vitest"],
      "allowedVersions": { "vite": "*", "vitest": "*" }
    }
  }
}
```

**Important**:

- `overrides.vite` ensures any dependency requiring `vite` gets `vite-plus` instead
- For pnpm without existing config, overrides and peerDependencyRules are written to `pnpm-workspace.yaml`
- For pnpm with existing `pnpm` config in `package.json`, the existing location is respected
- rewrite `import from 'vite'` to `import from 'vite-plus'`
- rewrite `import from 'vite/{name}'` to `import from 'vite-plus/{name}'`, e.g.: `import from 'vite/module-runner'` to `import from 'vite-plus/module-runner'`
- rewrite `import from 'vitest'` to `import from 'vite-plus/test'`
- rewrite `import from 'vitest/config'` to `import from 'vite-plus'`
- rewrite `import from 'vitest/{name}'` to `import from 'vite-plus/test/{name}'`, e.g.: `import from 'vitest/node'` to `import from 'vite-plus/test/node'`
- rewrite `import from '@vitest/browser'` to `import from 'vite-plus/test/browser'`
- rewrite `import from '@vitest/browser/{name}'` to `import from 'vite-plus/test/browser/{name}'`, e.g.: `import from '@vitest/browser/context'` to `import from 'vite-plus/test/browser/context'`
- rewrite `import from '@vitest/browser-playwright'` to `import from 'vite-plus/test/browser-playwright'`
- rewrite `import from '@vitest/browser-playwright/{name}'` to `import from 'vite-plus/test/browser-playwright/{name}'`

**Note**: For Yarn, use `resolutions` instead of `overrides`.

### Oxlint Configuration

**Before (.oxlintrc):**

```json
{
  "rules": {
    "no-unused-vars": "error",
    "no-console": "warn"
  },
  "ignorePatterns": ["dist", "node_modules"]
}
```

**After (merged into vite.config.ts):**

```typescript
import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [],

  // Oxlint configuration
  lint: {
    options: {
      typeAware: true,
      typeCheck: true,
    },
    rules: {
      'no-unused-vars': 'error',
      'no-console': 'warn',
    },
    ignorePatterns: ['dist', 'node_modules'],
  },
});
```

> **Note**: If `tsconfig.json` contains `compilerOptions.baseUrl`, `typeAware` and `typeCheck` are not injected because oxlint's TypeScript checker does not yet support `baseUrl`. Run `npx @andrewbranch/ts5to6 --fixBaseUrl .` to migrate away from `baseUrl`.

### Oxfmt Configuration

**Before (.oxfmtrc):**

```json
{
  "printWidth": 100,
  "tabWidth": 2,
  "semi": true,
  "singleQuote": true,
  "trailingComma": "es5"
}
```

**After (merged into vite.config.ts):**

```typescript
import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [],

  // Oxfmt configuration
  fmt: {
    printWidth: 100,
    tabWidth: 2,
    semi: true,
    singleQuote: true,
    trailingComma: 'es5',
  },
});
```

### import namespace change to vite-plus

effect files:

- vitest.config.ts
- vite.config.ts

**Before (import from 'vitest/config'):**

```typescript
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
  },
});
```

**After (import from 'vite-plus'):**

```typescript
import { defineConfig } from 'vite-plus';

export default defineConfig({
  test: {
    globals: true,
  },
});
```

**Before (import from 'vite'):**

```typescript
import { defineConfig } from 'vite';

export default defineConfig({
  test: {
    globals: true,
  },
});
```

**After (import from 'vite-plus'):**

```typescript
import { defineConfig } from 'vite-plus';

export default defineConfig({
  test: {
    globals: true,
  },
});
```

### Complete Example

**Before:**

```
my-package/
├── package.json              # Has vite, vitest, oxlint, oxfmt
├── vite.config.ts            # Vite config
├── vitest.config.ts          # Vitest config
├── .oxlintrc                 # Oxlint config
├── .oxfmtrc                  # Oxfmt config
└── src/
```

**After:**

```
my-package/
├── package.json              # Only has vite-plus
├── vitest.config.ts          # Vitest config
├── vite.config.ts            # Unified config (all merged)
└── src/
```

**vite.config.ts (after migration):**

```typescript
// Import from 'vite' still works - overrides maps it to vite-plus
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite-plus';

export default defineConfig({
  // Vite configuration
  plugins: [react()],
  server: {
    port: 3000,
  },
  build: {
    target: 'esnext',
  },

  // lint configuration (merged from .oxlintrc)
  lint: {
    rules: {
      'no-unused-vars': 'error',
      'no-console': 'warn',
    },
    ignorePatterns: ['dist', 'node_modules'],
  },

  // format configuration (merged from .oxfmtrc)
  fmt: {
    printWidth: 100,
    tabWidth: 2,
    semi: true,
    singleQuote: true,
    trailingComma: 'es5',
  },
});
```

**vitest.config.ts (after migration):**

```typescript
import { defineConfig } from 'vite-plus';

export default defineConfig({
  test: {
    globals: true,
  },
});
```

## Monorepo Configuration Migration

### for pnpm

For monorepo projects and standalone projects without existing `pnpm` config in `package.json`, overrides, peerDependencyRules, and catalog are written to `pnpm-workspace.yaml`. Projects with existing `pnpm` config in `package.json` keep using `package.json`.

`pnpm-workspace.yaml`

```yaml
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@latest
  vitest: npm:@voidzero-dev/vite-plus-test@latest
  vite-plus: latest
overrides:
  vite: 'catalog:'
  vitest: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
    - vitest
  allowedVersions:
    vite: '*'
    vitest: '*'
```

### for npm

`package.json`

```json
{
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@latest",
    "vitest": "npm:@voidzero-dev/vite-plus-test@latest"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@latest",
    "vitest": "npm:@voidzero-dev/vite-plus-test@latest"
  }
}
```

### for yarn 4.10.0+ (need catalog support)

`.yarnrc.yml`

```yaml
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@latest
  vitest: npm:@voidzero-dev/vite-plus-test@latest
```

`package.json`

```json
{
  "resolutions": {
    "vite": "catalog:",
    "vitest": "catalog:"
  }
}
```

### for yarn v1(not supported yet)

TODO: Add support for yarn v1

## Success Criteria

A successful migration should:

1. ✅ Replace all standalone tool dependencies with vite-plus
2. ✅ **Add package.json overrides** to force vite → vite-plus (for transitive deps)
3. ✅ **Transform vitest imports** to vite/test (since vitest is removed)
4. ✅ Merge all configurations into vite.config.ts
5. ✅ Preserve all user customizations and settings
6. ✅ Remove redundant configuration files
7. ✅ Provide clear feedback and next steps
8. ✅ Handle monorepo migrations efficiently
9. ✅ Be safe and transparent about what changes

## ESLint Migration

When an ESLint flat config (`eslint.config.{js,mjs,cjs,ts,mts,cts}`) and `eslint` dependency are detected, `vp migrate` offers to convert the ESLint configuration to oxlint using [`@oxlint/migrate`](https://www.npmjs.com/package/@oxlint/migrate).

**Flow**: ESLint → oxlint (via `@oxlint/migrate`) → vite+ (existing merge flow)

**Steps**:

1. Run `vpx @oxlint/migrate --merge --type-aware --with-nursery --details` to generate `.oxlintrc.json`
2. Run `vpx @oxlint/migrate --replace-eslint-comments` to replace `eslint-disable` comments
3. Delete the ESLint config file
4. Remove `eslint` from `devDependencies`
5. Rewrite `eslint` scripts in `package.json` to `vp lint`, stripping ESLint-only flags
6. Rewrite `eslint` references in lint-staged configs (package.json `lint-staged` field and standalone config files like `.lintstagedrc.json`)
7. The existing migration flow picks up `.oxlintrc.json` and merges it into `vite.config.ts`

**Script Rewriting** (powered by [brush-parser](https://github.com/reubeno/brush) for shell AST parsing):

| Before                                     | After                                        |
| ------------------------------------------ | -------------------------------------------- |
| `eslint .`                                 | `vp lint .`                                  |
| `eslint --cache --ext .ts --fix .`         | `vp lint --fix .`                            |
| `NODE_ENV=test eslint --cache .`           | `NODE_ENV=test vp lint .`                    |
| `cross-env NODE_ENV=test eslint --cache .` | `cross-env NODE_ENV=test vp lint .`          |
| `eslint . && vite build`                   | `vp lint . && vite build`                    |
| `if [ -f .eslintrc ]; then eslint .; fi`   | `if [ -f .eslintrc ]; then vp lint . fi`     |
| `npx eslint .`                             | `npx eslint .` (npx/bunx wrappers preserved) |

Stripped ESLint-only flags: `--cache`, `--ext`, `--parser`, `--parser-options`, `--plugin`, `--rulesdir`, `--resolve-plugins-relative-to`, `--output-file`, `--env`, `--no-eslintrc`, `--no-error-on-unmatched-pattern`, `--debug`, `--no-inline-config`

The rewriter handles:

- **Compound commands**: `&&`, `||`, `|`, `if/then/fi`, `while/do/done`, `for`, `case`, brace groups `{ ...; }`, subshells `(...)`
- **Environment variable prefixes**: `NODE_ENV=test eslint .`
- **cross-env wrappers**: `cross-env NODE_ENV=test eslint .`
- **No-op safety**: Scripts without `eslint` are returned unchanged (no formatting corruption from AST round-tripping)

**Legacy ESLint Config Handling**:

If only a legacy ESLint config (`.eslintrc*`) is detected without a flat config (`eslint.config.*`), the migration warns and skips ESLint migration. The warning guides users to upgrade to ESLint v9 first, since `@oxlint/migrate` only supports flat configs:

> Legacy ESLint configuration detected (.eslintrc). Automatic migration to Oxlint requires ESLint v9+ with flat config format (eslint.config.\*). Please upgrade to ESLint v9 first: https://eslint.org/docs/latest/use/migrate-to-9.0.0

**Behavior**:

- Interactive mode: prompts user for confirmation upfront (Phase 1), executes later (Phase 2)
- Non-interactive mode: auto-runs without prompting
- Failure is non-blocking — warns and continues with the rest of migration
- Re-runnable: if user declines initially, running `vp migrate` again offers eslint migration

## Prettier Migration

When a Prettier configuration file (`.prettierrc*`, `prettier.config.*`, or `"prettier"` key in package.json) and `prettier` dependency are detected, `vp migrate` offers to convert the Prettier configuration to oxfmt using `vp fmt --migrate=prettier`.

**Flow**: Prettier → oxfmt (via `vp fmt --migrate=prettier`) → vite+ (existing merge flow)

**Steps**:

1. Run `vp fmt --migrate=prettier` to generate `.oxfmtrc.json` from Prettier config (if a standalone config file exists, not `package.json#prettier`)
2. Delete all Prettier config files (`.prettierrc*`, `prettier.config.*`)
3. Remove `"prettier"` key from package.json if present
4. Remove `prettier` and `prettier-plugin-*` from `devDependencies`/`dependencies`
5. Rewrite `prettier` scripts in `package.json` to `vp fmt`, stripping Prettier-only flags
6. Rewrite `prettier` references in lint-staged configs
7. Warn about `.prettierignore` if present (Oxfmt supports it, but `ignorePatterns` is recommended)
8. The existing migration flow picks up `.oxfmtrc.json` and merges it into `vite.config.ts`

**Script Rewriting** (powered by [brush-parser](https://github.com/reubeno/brush) for shell AST parsing):

| Before                                            | After                                                  |
| ------------------------------------------------- | ------------------------------------------------------ |
| `prettier .`                                      | `vp fmt .`                                             |
| `prettier --write .`                              | `vp fmt .`                                             |
| `prettier --check .`                              | `vp fmt --check .`                                     |
| `prettier --list-different .`                     | `vp fmt --check .`                                     |
| `prettier -l .`                                   | `vp fmt --check .`                                     |
| `prettier --write --single-quote --tab-width 4 .` | `vp fmt .`                                             |
| `prettier --config .prettierrc --write .`         | `vp fmt .`                                             |
| `prettier --plugin prettier-plugin-tailwindcss .` | `vp fmt .`                                             |
| `cross-env NODE_ENV=test prettier --write .`      | `cross-env NODE_ENV=test vp fmt .`                     |
| `prettier --write . && eslint --fix .`            | `vp fmt . && eslint --fix .`                           |
| `npx prettier --write .`                          | `npx prettier --write .` (npx/bunx wrappers preserved) |

**Stripped Prettier-only flags**:

- Value flags: `--config`, `--ignore-path`, `--plugin`, `--parser`, `--cache-location`, `--cache-strategy`, `--log-level`, `--stdin-filepath`, `--cursor-offset`, `--range-start`, `--range-end`, `--config-precedence`, `--tab-width`, `--print-width`, `--trailing-comma`, `--arrow-parens`, `--prose-wrap`, `--end-of-line`, `--html-whitespace-sensitivity`, `--quote-props`, `--embedded-language-formatting`, `--experimental-ternaries`
- Boolean flags: `--write`, `--cache`, `--no-config`, `--no-editorconfig`, `--with-node-modules`, `--require-pragma`, `--insert-pragma`, `--no-bracket-spacing`, `--single-quote`, `--no-semi`, `--jsx-single-quote`, `--bracket-same-line`, `--use-tabs`, `--debug-check`, `--debug-print-doc`, `--debug-benchmark`, `--debug-repeat`

**Converted flags**: `--list-different` / `-l` → `--check`

**Kept flags**: `--check`, `--fix`, `--no-error-on-unmatched-pattern`, positional args (file paths/globs)

**Behavior**:

- Interactive mode: prompts user for confirmation upfront (Phase 1), executes later (Phase 2)
- Non-interactive mode: auto-runs without prompting
- Failure is non-blocking — warns and continues with the rest of migration
- Re-runnable: if user declines initially, running `vp migrate` again offers prettier migration

## tsconfig.json Cleanup

During migration, `vp migrate` scans all `tsconfig*.json` files in the project directory (non-recursive) and removes deprecated options that would cause lint errors.

**Currently removed options**:

- `"esModuleInterop": false` — This option has been removed by typescript. When present, `vp lint --type-aware` fails with: `Option 'esModuleInterop=false' has been removed.`

**Behavior**:

- Only `esModuleInterop: false` is removed — `true` is left alone
- Uses `jsonc-parser` for JSONC-aware editing that preserves comments and formatting
- Scans all `tsconfig*.json` variants (e.g., `tsconfig.json`, `tsconfig.app.json`, `tsconfig.node.json`)
- Runs automatically as part of the config rewrite phase — no user prompt needed

## References

### Code Transformation

- [ast-grep](https://ast-grep.github.io/) - Structural search and replace tool
- [Turborepo Codemods](https://turborepo.com/docs/reference/turbo-codemod) - Similar migration approach
- [jscodeshift](https://github.com/facebook/jscodeshift) - Alternative AST transformation tool

### Tools

- [@ast-grep/napi](https://www.npmjs.com/package/@ast-grep/napi) - Node.js bindings for ast-grep
- [@oxlint/migrate](https://www.npmjs.com/package/@oxlint/migrate) - ESLint to oxlint migration tool
- [brush-parser](https://github.com/reubeno/brush) - Shell AST parser for script rewriting (Rust)
- [@clack/prompts](https://www.npmjs.com/package/@clack/prompts) - Beautiful CLI prompts
- [typescript](https://www.typescriptlang.org/) - For parsing TypeScript configs

### Inspiration

- [Vue 2 to Vue 3 Migration](https://v3-migration.vuejs.org/) - Similar migration tool
- [React Codemod](https://github.com/reactjs/react-codemod) - React migration scripts
- [Angular Update Guide](https://update.angular.io/) - Automated Angular migrations

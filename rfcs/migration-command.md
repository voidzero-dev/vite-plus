# RFC: Vite+ Migration Command

## Background

When transitioning to vite+, projects typically use standalone tools like vite, oxlint, oxfmt, and vitest, each with their own dependencies and configuration files. The `vite migration` command automates the process of consolidating these tools into the unified vite+ toolchain.

**Problem**: Manual migration is error-prone and time-consuming:

- Multiple dependency entries to update in package.json
- Various configuration files to merge (vite.config.ts, .oxlintrc, .oxfmtrc, etc.)
- Risk of missing configurations or incorrect merging
- Tedious process when migrating multiple packages in a monorepo

**Solution**: Automated migration using [ast-grep](https://ast-grep.github.io/) for intelligent code transformation.

**Related Commands**:

- `vite gen` - Uses this same migration engine after generating code (see [code-generator.md](./code-generator.md))
- `vite migration` - This command, for migrating existing projects

## Goals

1. **Dependency Consolidation**: Replace standalone vite, vitest, oxlint, oxfmt dependencies with unified vite-plus
2. **Configuration Unification**: Merge .oxlintrc, .oxfmtrc into vite.config.ts
3. **Safe & Reversible**: Preview changes before applying, support rollback
4. **Intelligent**: Preserve custom configurations and user overrides
5. **Monorepo-Aware**: Migrate multiple packages efficiently

## Scope

**What this command migrates**:

- ✅ **Dependencies**: vite, vitest, oxlint, oxfmt → vite-plus
- ✅ **Overrides**: Force vite → vite-plus (for all dependencies)
  - npm/pnpm/bun: Adds `overrides.vite` mapping
  - yarn: Adds `resolutions.vite` mapping
  - **Benefit**: Code keeps `import from 'vite'` - automatically resolves to vite-plus
- ✅ **Configuration files**:
  - .oxlintrc → vite.config.ts (lint section)
  - .oxfmtrc → vite.config.ts (format section)

**What this command does NOT migrate**:

- ❌ ESLint → oxlint (different tools, not a version upgrade)
- ❌ Prettier → oxfmt (different tools, not a version upgrade)
- ❌ Package.json scripts → vite-task.json (different feature)
- ❌ TypeScript configuration changes
- ❌ Build tool changes (webpack/rollup → vite)

These are **consolidation migrations**, not **feature migrations**.

## Command Usage

```bash
# Migrate current directory
vite migration

# Aliases
vite migrate
```

## Migration Process

### Step 1: Detection

Analyze the project to detect which tools are being used:

```typescript
interface DetectionResult {
  hasVite: boolean;
  hasVitest: boolean;
  hasOxlint: boolean;
  hasOxfmt: boolean;
  dependencies: {
    vite?: string;
    vitest?: string;
    oxlint?: string;
    oxfmt?: string;
  };
  configs: {
    viteConfig?: string; // vite.config.ts
    oxlintConfig?: string; // .oxlintrc
    oxfmtConfig?: string; // .oxfmtrc, oxfmt.config.json
  };
}
```

### Step 2: Preview

Show user what will change:

```bash
$ vite migration

◇  Analyzing project...
│
◆  Detected standalone tools:
│  ✓ vite ^5.0.0
│  ✓ vitest ^1.0.0
│  ✓ oxlint ^0.1.0
│  ✓ oxfmt ^0.1.0
│
◆  Configuration files found:
│  • vite.config.ts
│  • vitest.config.ts
│  • .oxlintrc
│  • .oxfmtrc
│
◆  Migration plan:
│
│  Dependencies (package.json):
│  - vite: ^5.0.0
│  - vitest: ^1.0.0
│  - oxlint: ^0.1.0
│  - oxfmt: ^0.1.0
│  + vite-plus: ^0.1.0
│
│  Configuration:
│  ✓ Merge vitest.config.ts → vite.config.ts
│  ✓ Merge .oxlintrc → vite.config.ts
│  ✓ Merge .oxfmtrc → vite.config.ts
│  ✓ Remove redundant config files
│
◆  Proceed with migration?
│  ● Yes / ○ No / ○ Preview changes
```

### Step 3: Transformation

Apply migrations using ast-grep:

```bash
◇  Applying migrations...
│  ✓ Updated package.json dependencies
│  ✓ Added package.json overrides (vite → vite-plus)
│  ✓ Updated vitest imports in 18 files (vitest → vite/test)
│  ✓ Merged vitest.config.ts → vite.config.ts
│  ✓ Merged .oxlintrc → vite.config.ts
│  ✓ Merged .oxfmtrc → vite.config.ts
│  ✓ Removed vitest.config.ts
│  ✓ Removed .oxlintrc
│  ✓ Removed .oxfmtrc
│
└  Migration completed!

Next steps:
  1. Review vite.config.ts to ensure configurations are correct
  2. Run 'vite install' to update dependencies
  3. Run 'vite build' and 'vite test' to verify everything works
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

**After:**

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

**Important**:

- `overrides.vite` ensures any dependency requiring `vite` gets `vite-plus` instead
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
    rules: {
      'no-unused-vars': 'error',
      'no-console': 'warn',
    },
    ignorePatterns: ['dist', 'node_modules'],
  },
});
```

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

`pnpm-workspace.yaml`

```yaml
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@latest
  vitest: npm:@voidzero-dev/vite-plus-test@latest

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
7. ✅ Create backups before applying changes
8. ✅ Validate the result works correctly (build and test still work)
9. ✅ Provide clear feedback and next steps
10. ✅ Support rollback if something goes wrong
11. ✅ Handle monorepo migrations efficiently
12. ✅ Be safe and transparent about what changes

## References

### Code Transformation

- [ast-grep](https://ast-grep.github.io/) - Structural search and replace tool
- [Turborepo Codemods](https://turborepo.com/docs/reference/turbo-codemod) - Similar migration approach
- [jscodeshift](https://github.com/facebook/jscodeshift) - Alternative AST transformation tool

### Tools

- [@ast-grep/napi](https://www.npmjs.com/package/@ast-grep/napi) - Node.js bindings for ast-grep
- [@clack/prompts](https://www.npmjs.com/package/@clack/prompts) - Beautiful CLI prompts
- [typescript](https://www.typescriptlang.org/) - For parsing TypeScript configs

### Inspiration

- [Vue 2 to Vue 3 Migration](https://v3-migration.vuejs.org/) - Similar migration tool
- [React Codemod](https://github.com/reactjs/react-codemod) - React migration scripts
- [Angular Update Guide](https://update.angular.io/) - Automated Angular migrations

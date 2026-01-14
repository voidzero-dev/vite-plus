# Migration Guide

This guide will help you migrate your existing Vite and Vitest projects to Vite+.

## Prerequisites

Before migrating to Vite+, ensure your project meets the following version requirements:

### Vite 8.x Required

Vite+ requires **Vite 8.x** or higher. If you're currently using Vite 7.x or earlier, you must upgrade first.

**To upgrade from Vite 7 to Vite 8:**

1. Refer to the official [Vite Migration Guide](https://main.vite.dev/guide/migration.html) for detailed instructions.

2. Key breaking changes in Vite 8:
   - **Build targets updated**: Chrome 111+, Edge 111+, Firefox 114+, Safari 16.4+
   - **Rolldown & Oxc adoption**: Vite 8 uses Rolldown and Oxc-based tools instead of esbuild and Rollup
   - **Configuration changes**:
     - Migrate `esbuild` options to `oxc`
     - Migrate `optimizeDeps.esbuildOptions` to `optimizeDeps.rolldownOptions`
     - Migrate `build.rollupOptions` to `build.rolldownOptions`

3. Update your `package.json`:
   ```json
   {
     "devDependencies": {
       "vite": "^8.0.0"
     }
   }
   ```

### Vitest 4.x Required

Vite+ requires **Vitest 4.x** or higher. If you're currently using Vitest 3.x or earlier, you must upgrade first.

**To upgrade from Vitest 3 to Vitest 4:**

1. Refer to the official [Vitest Migration Guide](https://vitest.dev/guide/migration.html) for detailed instructions.

2. Key breaking changes in Vitest 4:
   - **V8 coverage overhaul**: Uses AST-based analysis instead of `v8-to-istanbul`
   - **Coverage options removed**: `coverage.all` and `coverage.extensions` are removed; use `coverage.include` instead
   - **Test exclusions narrowed**: Only `node_modules` and `.git` are excluded by default
   - **Module Runner replacement**: `vite-node` is replaced with Vite's Module Runner
   - **Configuration changes**:
     - `workspace` renamed to `projects`
     - `maxThreads`/`maxForks` become `maxWorkers`
     - Pool options moved to top level

3. Update your `package.json`:
   ```json
   {
     "devDependencies": {
       "vitest": "^4.0.0"
     }
   }
   ```

## Automatic Migration

Vite+ provides an automated migration command that handles most of the migration work for you.

### Running the Migration

```bash
# Migrate current directory
vite migration

# Alias
vite migrate
```

### What the Migration Does

The `vite migration` command automatically:

1. **Updates dependencies**: Replaces standalone `vite`, `vitest`, `oxlint`, and `oxfmt` with unified Vite+ packages
2. **Configures overrides**: Adds package manager overrides to ensure all dependencies use Vite+ versions
3. **Rewrites imports**: Updates `import from 'vite'` and `import from 'vitest/config'` to `import from 'vite-plus'`
4. **Merges configurations**: Consolidates `.oxlintrc` and `.oxfmtrc` into `vite.config.ts`
5. **Updates scripts**: Rewrites npm scripts to use Vite+ commands

### Migration Preview

When you run the migration command, you'll see a preview of changes:

```bash
$ vite migration

◇  Analyzing project...
│
◆  Detected standalone tools:
│  ✓ vite ^8.0.0
│  ✓ vitest ^4.0.0
│  ✓ oxlint ^0.1.0
│
◆  Configuration files found:
│  • vite.config.ts
│  • vitest.config.ts
│  • .oxlintrc
│
◆  Migration plan:
│
│  Dependencies (package.json):
│  - vite: ^8.0.0
│  - vitest: ^4.0.0
│  - oxlint: ^0.1.0
│  + vite-plus: latest
│
│  Configuration:
│  ✓ Merge .oxlintrc → vite.config.ts
│  ✓ Update imports in config files
│
◆  Proceed with migration?
│  ● Yes / ○ No / ○ Preview changes
```

## Manual Migration Steps

If you prefer to migrate manually or need to understand what changes are made, follow these steps:

### 1. Update Dependencies

#### For Standalone Projects

**pnpm:**

```json
{
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@latest",
    "vitest": "npm:@voidzero-dev/vite-plus-test@latest",
    "vite-plus": "latest"
  },
  "pnpm": {
    "overrides": {
      "vite": "npm:@voidzero-dev/vite-plus-core@latest",
      "vitest": "npm:@voidzero-dev/vite-plus-test@latest"
    }
  }
}
```

**npm:**

```json
{
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@latest",
    "vitest": "npm:@voidzero-dev/vite-plus-test@latest",
    "vite-plus": "latest"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@latest",
    "vitest": "npm:@voidzero-dev/vite-plus-test@latest"
  }
}
```

**yarn:**

```json
{
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@latest",
    "vitest": "npm:@voidzero-dev/vite-plus-test@latest",
    "vite-plus": "latest"
  },
  "resolutions": {
    "vite": "npm:@voidzero-dev/vite-plus-core@latest",
    "vitest": "npm:@voidzero-dev/vite-plus-test@latest"
  }
}
```

#### For pnpm Monorepos

Add to `pnpm-workspace.yaml`:

```yaml
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@latest
  vitest: npm:@voidzero-dev/vite-plus-test@latest
  'vite-plus': latest

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

#### For npm Monorepos

Add to root `package.json`:

```json
{
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@latest",
    "vitest": "npm:@voidzero-dev/vite-plus-test@latest",
    "vite-plus": "latest"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@latest",
    "vitest": "npm:@voidzero-dev/vite-plus-test@latest"
  }
}
```

#### For Yarn 4.10+ Monorepos

Add to `.yarnrc.yml`:

```yaml
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@latest
  vitest: npm:@voidzero-dev/vite-plus-test@latest
  'vite-plus': latest
```

Add to root `package.json`:

```json
{
  "resolutions": {
    "vite": "npm:@voidzero-dev/vite-plus-core@latest",
    "vitest": "npm:@voidzero-dev/vite-plus-test@latest"
  }
}
```

### 2. Configure Registry Access

Add to `.npmrc`:

```ini
@voidzero-dev:registry=https://npm.pkg.github.com/
//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}
```

### 3. Update Configuration Files

#### Update vite.config.ts

**Before:**

```typescript
import { defineConfig } from 'vite';

export default defineConfig({
  // your config
});
```

**After:**

```typescript
import { defineConfig } from 'vite-plus';

export default defineConfig({
  // your config
});
```

#### Update vitest.config.ts

**Before:**

```typescript
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    // your test config
  },
});
```

**After:**

```typescript
import { defineConfig } from 'vite-plus';

export default defineConfig({
  test: {
    // your test config
  },
});
```

### 4. Merge Linter Configuration

If you have an `.oxlintrc` file, merge it into `vite.config.ts`:

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

**After (vite.config.ts):**

```typescript
import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    rules: {
      'no-unused-vars': 'error',
      'no-console': 'warn',
    },
    ignorePatterns: ['dist', 'node_modules'],
  },
});
```

### 5. Merge Formatter Configuration

If you have an `.oxfmtrc` file, merge it into `vite.config.ts`:

**Before (.oxfmtrc):**

```json
{
  "printWidth": 100,
  "tabWidth": 2,
  "semi": true,
  "singleQuote": true
}
```

**After (vite.config.ts):**

```typescript
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {
    printWidth: 100,
    tabWidth: 2,
    semi: true,
    singleQuote: true,
  },
});
```

### 6. Update lint-staged Configuration {#lint-staged}

If you use lint-staged, update your configuration to use Vite+ commands:

**Before:**

```json
{
  "*.{js,ts,tsx}": ["oxlint --fix", "oxfmt --write"]
}
```

**After:**

```json
{
  "*.{js,ts,tsx}": ["vite lint --fix", "vite fmt"]
}
```

::: warning
The automatic migration only supports JSON format lint-staged configurations (`.lintstagedrc.json` or `.lintstagedrc`). If you use other formats like `.lintstagedrc.yaml`, `.lintstagedrc.js`, or `lint-staged.config.mjs`, you'll need to update them manually.
:::

## Post-Migration Steps

After migration completes:

1. **Verify the build**:
   ```bash
   vite run build
   ```
   ::: tip
   `vite run` automatically installs dependencies if needed, so you don't need to run `pnpm install` / `npm install` / `yarn install` manually.
   :::

2. **Run tests**:
   ```bash
   vite test
   ```

3. **Check linting**:
   ```bash
   vite lint
   ```

4. **Review `vite.config.ts`**: Ensure all merged configurations are correct.

## Troubleshooting

### Version Check Failed

If you see an error like:

```
❌ vite@7.x.x is not supported by auto migration
Please upgrade vite to version >=8.0.0 first
```

You need to upgrade to Vite 8 first. See the [Vite Migration Guide](https://main.vite.dev/guide/migration.html).

Similarly for Vitest:

```
❌ vitest@3.x.x is not supported by auto migration
Please upgrade vitest to version >=4.0.0 first
```

Upgrade to Vitest 4 first. See the [Vitest Migration Guide](https://vitest.dev/guide/migration.html).

### Configuration Merge Failed

If the automatic configuration merge fails:

```
❌ Failed to merge .oxlintrc into vite.config.ts
Please complete the merge manually
```

You'll need to manually add the configuration to your `vite.config.ts`. See the [Configuration Guide](/config/) for the full configuration reference.

### Import Rewrite Issues

If you have complex import patterns that weren't automatically updated, search for and replace:

- `from 'vite'` → `from 'vite-plus'`
- `from 'vitest/config'` → `from 'vite-plus'`
- `from 'vitest'` → `from 'vite-plus/test'`

## What's Not Migrated

The migration command does **not** handle:

- **ESLint → oxlint**: These are different tools, not version upgrades
- **Prettier → oxfmt**: These are different tools, not version upgrades
- **Package scripts → vite-task.json**: Task configuration is a separate feature
- **TypeScript configuration changes**
- **Build tool migrations** (webpack/rollup → vite)

For these scenarios, refer to the relevant documentation sections.

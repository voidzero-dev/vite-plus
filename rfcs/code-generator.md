# RFC: Vite+ Code Generator

## Background

Reference:

- [Nx Code Generation](https://nx.dev/docs/features/generate-code)
- [Turborepo Code Generation](https://turborepo.com/docs/guides/generating-code)
- [Bingo Framework](https://www.create.bingo/about)

The code generator function is an essential feature for a company's continuously iterated monorepo. Without it, creating a new project can only be done by manually copying and pasting and then modifying the files to make them usable, which likely leads to missing changes.

## Comparison of Existing Solutions

| Feature                      | Nx                                        | Turbo                                        | Bingo                                                                               |
| ---------------------------- | ----------------------------------------- | -------------------------------------------- | ----------------------------------------------------------------------------------- |
| Generator approach           | Nx plugin's generator feature             | Based on [PLOP](https://plopjs.com/) wrapper | Repository templating engine with type-safe options                                 |
| Monorepo reuse               | ✅                                        | ✅                                           | ✅                                                                                  |
| Template engine              | [EJS](https://github.com/nrwl/nx)         | Handlebars                                   | [Template Literals](https://www.create.bingo/build/concepts/templates) / Handlebars |
| Input validation             | schema.json                               | validate(input): true \| string              | Zod schemas (type-safe)                                                             |
| Advanced features            | Tree API for file operations              | Medium                                       | Network requests, shell scripts, blocks/presets                                     |
| Complexity of implementation | High (through @nx/devkit tree operations) | Medium                                       | Medium (TypeScript-based)                                                           |
| Type safety                  | Medium (JSON Schema)                      | Low (JavaScript)                             | High (Zod + TypeScript)                                                             |

## Why Bingo's Approach?

Bingo offers several advantages:

1. **Type Safety**: Uses Zod schemas with TypeScript for compile-time validation
2. **Modern Architecture**: Built from the ground up with modern JavaScript/TypeScript
3. **Flexible**: Supports simple templates to complex block-based systems
4. **Extensible**: Can execute network requests and shell scripts beyond file generation
5. **Two Modes**: Setup (create new) and Transition (update existing) modes
6. **Testable**: Built-in testing utilities for template development
7. **Existing Ecosystem**: Can directly run existing bingo templates from npm

## Integration Strategy: Dual-Mode Support

**Key Decision**: `vite new` supports BOTH bingo templates AND any existing `create-*` template through intelligent migration:

### Two Ways to Write Generators

**Option 1: Bingo Templates (Recommended for Custom Generators)**

Best for creating **reusable local generators** within your monorepo:

```bash
# Run workspace-local bingo generator
vite new @company/generator-ui-lib

# Or any bingo template from npm
vite new create-typescript-app
```

**Why use bingo**:

- ✅ **Easier to write**: Type-safe with Zod schemas
- ✅ **Better DX**: Full control over file generation
- ✅ **Testable**: Built-in testing utilities
- ✅ **Quick start**: Use `@vite-plus/create-generator` to scaffold a generator
- ✅ **Perfect for**: Company-specific patterns and standards

**Option 2: Universal Templates (Any create-\* package)**

Run **ANY** existing `create-*` template from the ecosystem:

```bash
# Run any existing template
vite new create-vite
vite new create-next-app
vite new create-nuxt
```

**Why use universal templates**:

- ✅ **No work required**: Use existing templates as-is
- ✅ **Zero learning curve**: Use familiar templates
- ✅ **Huge ecosystem**: Thousands of templates available
- ✅ **No maintenance**: Template authors maintain them

### Auto-Migration for ALL Templates

**Important**: Regardless of whether you use bingo or universal templates, **all generated code goes through the same auto-detect and migrate logic**:

```
ANY template (bingo or universal)
  ↓
Template generates code
  ↓
Vite+ auto-detects vite-related tools:
  • Standalone vite, vitest, oxlint, oxfmt
  ↓
Auto-migrate to unified vite-plus:
  • Dependencies: vite + vitest + oxlint + oxfmt → vite-plus
  • Configs: Merge vitest.config.ts, .oxlintrc, .oxfmtrc → vite.config.ts
  ↓
Monorepo integration:
  • Prompt for workspace dependencies
  • Update workspace config (pnpm-workspace.yaml, package.json, etc.)
```

**Scope of Auto-Migration**:

- ✅ Consolidate vite/vitest/oxlint/oxfmt dependencies → vite-plus
- ✅ Merge tool configurations into vite.config.ts
- ❌ Does NOT migrate ESLint → oxlint (if template uses ESLint, it stays)
- ❌ Does NOT create vite-task.json (optional, separate feature)
- ❌ Does NOT change TypeScript config (remains as generated)

**Bingo templates get the same migration treatment** - the difference is just that bingo makes it easier to write generators with type safety and testing.

### How It Works

```
┌──────────────────────────┐
│  vite new <name>         │
└───────────┬──────────────┘
            │
      ┌─────▼─────────┐
      │ Detect & Load │
      │ Template Type │
      └─────┬─────────┘
            │
    ┌───────┴────────┐
    │                │
┌───▼──────┐   ┌─────▼──────┐
│  Bingo   │   │ Universal  │
│ Template │   │ (create-*) │
└───┬──────┘   └─────┬──────┘
    │                │
    └────────┬───────┘
             │
      ┌──────▼─────────┐
      │ Execute        │
      │ Template       │
      └──────┬─────────┘
             │
      ┌──────▼─────────────┐
      │ Auto-Detect        │
      │ Generated Code     │
      │ (ALL templates)    │
      └──────┬─────────────┘
             │
      ┌──────▼─────────────┐
      │ Auto-Migrate       │
      │ vite-tools         │
      │ → vite-plus        │
      └──────┬─────────────┘
             │
      ┌──────▼─────────────┐
      │ Monorepo           │
      │ Integration        │
      │ • Workspace deps   │
      │ • Update configs   │
      └────────────────────┘
```

## Monorepo-Specific Enhancements

After any template runs, Vite+ adds monorepo-specific features:

### 1. Auto-Migration to vite-plus Unified Toolchain (for ALL templates)

**After any template runs** (bingo or universal), Vite+ automatically detects standalone vite-related tools and offers to consolidate them into the unified vite-plus dependency.

**Purpose**: Simplify dependency management by consolidating vite, vitest, oxlint, and oxfmt into a single vite-plus package.

```bash
$ vite new create-vite --template react-ts

# create-vite runs normally...
✔ Project name: › my-app
✔ Select a framework: › React
✔ Select a variant: › TypeScript

Scaffolding project in ./packages/my-app...

# After template completes, Vite+ detects standalone tools
◇  Template completed! Detecting vite-related tools...
│
◆  Detected standalone vite tools:
│  ✓ vite ^5.0.0
│  ✓ vitest ^1.0.0
│
◆  Upgrade to vite-plus unified toolchain?
│
│  This will:
│  • Replace vite + vitest dependencies → single vite-plus dependency
│  • Merge vitest.config.ts → vite.config.ts (test section)
│  • Remove vitest.config.ts
│
│  Benefits:
│  • Simplified dependency management (1 instead of 2+ dependencies)
│  • Unified configuration in vite.config.ts
│  • Better integration with Vite+ task runner and caching
│
│  ● Yes / ○ No
│
◇  Migrating to vite-plus...
│  ✓ Updated package.json (vite + vitest → vite-plus)
│  ✓ Merged vitest.config.ts → vite.config.ts
│  ✓ Removed vitest.config.ts
│
└  Migration completed!
```

**Scope of Auto-Migration**:

This is a **dependency consolidation** feature, not a tool replacement feature.

✅ **What it does**:

- Consolidate standalone vite/vitest/oxlint/oxfmt dependencies → single vite-plus dependency
- Merge vitest.config.ts → vite.config.ts (test section)
- Merge .oxlintrc → vite.config.ts (oxlint section)
- Merge .oxfmtrc → vite.config.ts (oxfmt section)
- Remove redundant standalone config files

❌ **What it does NOT do**:

- Does NOT migrate ESLint → oxlint (different tools, not consolidation)
- Does NOT migrate Prettier → oxfmt (different tools, not consolidation)
- Does NOT create vite-task.json (separate feature, not required)
- Does NOT change TypeScript configuration (remains as generated)
- Does NOT modify build tools (webpack/rollup → vite)

**Why this design**:

- Templates that use vite/vitest/oxlint/oxfmt can be simplified to use vite-plus
- Templates that use other tools (ESLint, Prettier, Jest) remain unchanged
- Users keep their chosen tools, just with optimized vite-related dependencies

**Migration Engine powered by [ast-grep](https://ast-grep.github.io/)**:

- Structural search and replace for accurate code transformation
- YAML-based rules for easy maintenance
- Safe, reversible transformations
- **Note**: Uses the same migration engine as `vite migration` command (see [migration-command.md](./migration-command.md))

### 2. Target Directory Selection (Monorepo)

When running `vite new` in a monorepo workspace, Vite+ prompts users to select which parent directory to create the new package in:

```bash
$ vite new create-vite

◆  Where should we create the new package?
│  ○ apps/        (Applications)
│  ● packages/    (Shared packages)
│  ○ services/    (Backend services)
│  ○ tools/       (Development tools)
│
◇  Selected: packages/
│
# Template runs...
✔ Project name: › my-lib
```

**How it works**:

- Detects package manager from lock files (pnpm-lock.yaml, package-lock.json, yarn.lock, bun.lockb)
- Reads workspace configuration:
  - **pnpm**: Read `pnpm-workspace.yaml` → `packages` field
  - **npm/yarn**: Read root `package.json` → `workspaces` field
  - **bun**: Read root `package.json` → `workspaces` field
- Extracts parent directories from patterns (e.g., `apps/*`, `packages/*`)
- Prompts user to select one
- Passes the selected directory to the template (if template supports directory options)
- Or changes working directory before running template

**Benefits**:

- Clear organization in monorepo
- Users don't need to remember directory structure
- Consistent with workspace organization
- Can be skipped with `--directory` flag: `vite new create-vite --directory=packages`

### 3. Workspace Dependency Prompts

Inspired by [Turbo's generator](https://turborepo.com/docs/guides/generating-code), Vite+ prompts users to select existing workspace packages as dependencies:

```bash
$ vite new @company/generator-ui-lib --name=design-system

◇ Library name: design-system
◇ Framework: React
◇ Include Storybook? Yes

◆ Add workspace packages as dependencies?
│  ◼ @company/theme
│  ◼ @company/utils
│  ◻ @company/icons
│  ◻ @company/hooks
└

✅ Created design-system with dependencies:
   - @company/theme@workspace:*
   - @company/utils@workspace:*
```

This feature:

- **Auto-discovers** all packages in the workspace
- **Interactive selection** with multi-select checkbox UI
- **Smart defaults** based on package type or naming conventions
- **Proper version ranges** using `workspace:*` protocol
- **Updates package.json** automatically after generation

## Core Concepts

### 1. Templates

A template describes how to initialize or modify a repository given a set of options.

```typescript
import { createTemplate } from 'bingo';
import { z } from 'zod';

export default createTemplate({
  // Define options using Zod schemas for type safety
  options: {
    name: z.string().describe('Package name'),
    directory: z.enum(['apps', 'packages']).default('packages'),
    framework: z.enum(['react', 'vue', 'svelte']).default('react'),
  },

  // Optional: Prepare default values
  async prepare({ fs, options }) {
    return {
      name: options.name || (await fs.readdir('.').then((d) => d[0])),
    };
  },

  // Core production function
  async produce({ options }) {
    const projectPath = `${options.directory}/${options.name}`;

    return {
      files: {
        [`${projectPath}/package.json`]: JSON.stringify(
          {
            name: options.name,
            version: '0.0.1',
            dependencies: {
              [options.framework]: 'latest',
            },
          },
          null,
          2,
        ),
        [`${projectPath}/src/index.ts`]: `export const app = '${options.name}';`,
      },
      scripts: [
        { phase: 0, commands: [`cd ${projectPath}`, 'vite install'] },
        { phase: 1, commands: ['vite build'] },
      ],
      suggestions: [
        `✅ Created ${options.name} in ${projectPath}`,
        `Next: cd ${projectPath} && vite dev`,
      ],
    };
  },

  // Setup mode: additional logic for new repositories
  async setup({ options }) {
    return {
      requests: [
        {
          url: 'https://api.github.com/repos/:owner/:repo/labels',
          method: 'POST',
          body: { name: 'vite-plus', color: 'ff6b6b' },
        },
      ],
    };
  },

  // Transition mode: logic for updating existing repositories
  async transition({ options }) {
    return {
      scripts: [
        {
          phase: 0,
          commands: ['rm -rf old-config'],
          silent: true, // Don't error if file doesn't exist
        },
      ],
    };
  },
});
```

### 2. Creations

A Creation is an in-memory representation of repository changes that templates produce.

**Structure:**

```typescript
interface Creation {
  // Direct changes (always applied)
  files?: Files; // Hierarchical file structure
  requests?: Request[]; // Network API calls
  scripts?: Script[]; // Shell commands

  // Indirect guidance
  suggestions?: string[]; // Tips for manual steps
}
```

**Files Format:**

```typescript
// Strings become files, objects become directories
const files = {
  'README.md': '# My App',
  src: {
    'index.ts': 'export {}',
    utils: {
      'helpers.ts': 'export const helper = () => {}',
    },
  },
};
```

**Scripts with Phases:**

```typescript
const scripts = [
  // Phase 0: runs first
  { phase: 0, commands: ['vite install'] },
  { phase: 0, commands: ['git init'] },

  // Phase 1: runs after phase 0 completes
  { phase: 1, commands: ['vite build'] },
  { phase: 1, commands: ['vite format'] },
];
```

### 3. Modes

Templates operate in two modes:

**Setup Mode**: Creates a brand new repository

- Runs `setup()` function for additional creations
- Creates GitHub repository (if configured)
- Initializes git with initial commit

**Transition Mode**: Updates an existing repository

- Runs `transition()` function for migration logic
- Optionally cleans up old files
- Preserves existing git history

The CLI automatically infers the correct mode based on whether running in an existing repo.

### 4. Blocks and Presets

For complex templates with many configurable features, use the Stratum engine:

```typescript
import { createStratumTemplate } from 'bingo-stratum';

export default createStratumTemplate({
  // Define blocks (individual features)
  blocks: {
    linting: createBlock({
      about: { name: 'ESLint Configuration' },
      produce: ({ options }) => ({
        files: {
          '.eslintrc.json': JSON.stringify({ extends: ['eslint:recommended'] }),
        },
      }),
    }),

    testing: createBlock({
      about: { name: 'Vitest Setup' },
      produce: ({ options }) => ({
        files: {
          'vitest.config.ts': 'export default {}',
        },
      }),
    }),
  },

  // Define presets (block combinations)
  presets: {
    minimal: { blocks: [] },
    common: { blocks: ['linting'] },
    everything: { blocks: ['linting', 'testing'] },
  },

  // Suggested default
  suggested: 'common',
});
```

### 5. Inputs

Inputs are composable units for data retrieval and processing:

```typescript
import { createInput } from 'bingo';

const readPackageJson = createInput({
  async produce({ fs }) {
    const content = await fs.readFile('package.json', 'utf-8');
    return JSON.parse(content);
  },
});

const detectFramework = createInput({
  async produce({ take }) {
    const pkg = await take(readPackageJson);

    if (pkg.dependencies?.react) return 'react';
    if (pkg.dependencies?.vue) return 'vue';
    if (pkg.dependencies?.svelte) return 'svelte';

    return 'vanilla';
  },
});
```

## Interactive Mode

When running `vite new` without specifying a template, users enter interactive mode with a beautiful template selection interface.

### Template Selection Menu

Interactive mode presents a curated list of known templates:

```bash
$ vite new

┌  🎨 Vite+ Code Generator
│
◆  Which template would you like to use?
│  ○ Vite+ Monorepo (Create a new Vite+ monorepo project)
│  ○ Vite+ Generator (Scaffold a new code generator)
│  ○ Vite (Create vite applications and libraries)
│  ○ TanStack Start (Create TanStack applications and libraries)
│  ● Other (Enter a custom template package name)
└
```

### Known Templates with Auto-Configuration

Interactive mode includes pre-configured templates with automatic argument injection:

| Template Option     | Built-in Alias           | Description                                |
| ------------------- | ------------------------ | ------------------------------------------ |
| **Vite+ Monorepo**  | `vite:monorepo`          | Create a new Vite+ monorepo project        |
| **Vite+ Generator** | `vite:generator`         | Scaffold a new code generator              |
| **Vite**            | `create-vite`            | Create vite applications and libraries     |
| **TanStack Start**  | `@tanstack/create-start` | Create TanStack applications and libraries |
| **Other**           | _(user input)_           | Custom template package name               |

### Custom Template Input

When selecting "Other", users can input any npm template:

```bash
◆  Which template would you like to use?
│  ● Other (Enter a custom template package name)
│
◇  Enter the template package name:
│  create-next-app
│
◇  Discovering template: create-next-app
...
```

### Benefits

- **Discoverability**: Users can explore available templates without documentation
- **Ease of Use**: No need to remember exact template names or arguments
- **Guided Experience**: Clear hints help users choose the right template
- **Flexibility**: "Other" option allows any npm template
- **Consistency**: Same post-processing (migration, monorepo integration) applies to all

## CLI Usage

```bash
# Interactive mode - prompts for template selection
vite new

# Built-in Vite+ templates
vite new vite:monorepo                               # Vite+ monorepo
vite new vite:generator                              # Vite+ generator scaffold
vite new vite:application                            # Vite+ application
vite new vite:library                                # Vite+ library

# Run known templates directly
vite new create-vite                                 # Vite apps/libs
vite new @tanstack/create-start                      # TanStack apps/libs

# Run ANY template from npm
vite new create-next-app          # Next.js
vite new create-nuxt              # Nuxt
vite new create-typescript-app    # TypeScript (bingo)
vite new @company/generator-api   # Workspace-local bingo generator

# Run built-in Vite+ generators
vite new vite:monorepo
vite new vite:generator
vite new vite:application
vite new vite:library

# Pass through template options (use -- separator)
vite new create-vite -- --template react-ts
vite new create-next-app -- --typescript --app

# Control migrations (Vite+ options, before --)
vite new create-vite --no-migrate                    # Skip all migrations
vite new create-vite --migrate=vite-plus             # Only migrate to vite-plus

# Control target directory (Vite+ options, before --)
vite new create-vite --directory=packages            # Skip directory selection

# Control workspace dependencies (Vite+ options, before --)
vite new create-vite --deps=@company/utils,@company/logger  # Pre-select
vite new create-vite --no-prompt                     # Skip workspace dependency prompt

# Combine Vite+ options and template options
vite new create-vite --directory=apps --no-migrate --deps=@company/utils -- --template react-ts

# List available templates
vite new --list               # Shows built-in and popular templates
vite new --list --all         # Shows all installed templates

# Dry run (show what would be generated/migrated)
vite new create-vite --dry-run

# Combine with template options
vite new create-vite --dry-run -- --template vue-ts

# Help
vite new --help

# Aliases
vite g
vite newerate
```

## @vite-plus/create-generator Scaffold

To make it easier for users to create custom generators, we provide `@vite-plus/create-generator` - a bingo template that scaffolds a complete generator package.

### What It Generates

```
tools/generators/{generator-name}/
├── package.json              # Pre-configured with bingo, zod, bin entry
├── bin/
│   └── index.js              # CLI entrypoint
├── src/
│   ├── template.ts           # Main template with example code
│   └── template.test.ts      # Test examples using bingo/testers
├── tsconfig.json             # TypeScript configuration
└── README.md                 # Usage and customization guide
```

### Scaffold Template

```typescript
// Generated src/template.ts includes helpful examples and comments
import { createTemplate } from 'bingo';
import { z } from 'zod';

export default createTemplate({
  about: {
    name: '{Generator Name}',
    description: '{Description}',
  },

  // TODO: Define your options using Zod schemas
  options: {
    name: z.string().describe('Package name'),
    // Add more options as needed
  },

  // TODO: Customize the file generation logic
  async produce({ options }) {
    return {
      files: {
        // Define files to generate
        [`{output-path}/package.json`]: JSON.stringify(
          {
            name: options.name,
            version: '0.1.0',
          },
          null,
          2,
        ),
      },
      scripts: [
        // Optional: Add scripts to run after generation
      ],
      suggestions: [
        // Optional: Add suggestions for users
        `✅ Created ${options.name}`,
      ],
    };
  },
});
```

### Usage

```bash
# Step 1: Create the generator scaffold
vite new @vite-plus/create-generator

# Step 2: Customize the template
cd tools/generators/your-generator
# Edit src/template.ts

# Step 3: Test your generator
vite new @company/your-generator

# Step 4: Run tests
vite test
```

### Benefits

The scaffold saves you from:

- ✅ Setting up package.json with correct bin entry
- ✅ Configuring TypeScript for the generator
- ✅ Writing boilerplate bingo template code
- ✅ Setting up test infrastructure
- ✅ Creating README documentation

You get:

- ✅ Working example template with comments
- ✅ Complete test setup
- ✅ TypeScript configuration
- ✅ Ready to customize for your needs

## Technical Implementation Details

### Detecting Generated Project Directory

**Challenge**: After running a template command, we need to know which directory was created to apply migrations.

**Solution**: Use `fspy` (a Rust file system monitoring crate) to monitor file operations during template execution, then derive the project directory from file paths.

#### What fspy Achieves

**Core Functionality**:

- Monitor specific file operations (read/write of package.json) in real-time during template execution
- Capture paths when package.json is written or read
- Provide event stream of package.json operations
- Efficient event-based watching without polling

**How Vite+ Uses It**:

1. Start fspy watcher to monitor package.json operations before executing template
2. Execute template (template creates package.json in new project)
3. Capture package.json write/create path (e.g., `packages/my-app/package.json`)
4. Stop watcher when template completes
5. **Derive project directory** from package.json path (e.g., from `packages/my-app/package.json` → extract `packages/my-app`)
6. Use detected directory for subsequent migrations and workspace integration

**Deduction Logic**:

- Monitor for package.json file write/create operations
- When package.json is written, capture its full path
- Extract parent directory from the path (everything before `/package.json`)
- This is the project directory

**Example**:

```
Captured file operation:
- packages/my-app/package.json      ← write

Derived project directory: packages/my-app
```

**Benefits of Using fspy**:

- ✅ Real-time detection during template execution
- ✅ Accurate - every template creates package.json
- ✅ Efficient - only monitors package.json, not all files
- ✅ Simple - package.json path directly reveals project directory
- ✅ Works for all templates regardless of output format
- ✅ Rust-based performance

**Why This is Necessary**:

After detecting the project directory, we can:

1. **Apply migrations** in the correct location
2. **Update package.json** in the right project
3. **Register path** in workspace configuration (pnpm-workspace.yaml or package.json)
4. **Display next steps** with correct `cd` command

## Implementation Architecture

### Directory Structure

```
packages/
└── vite-generator/
    ├── src/
    │   ├── index.ts            # Main entry point
    │   ├── cli.ts              # CLI command handler
    │   ├── runner.ts           # Universal template runner
    │   ├── discovery.ts        # Template/package discovery
    │   ├── executor.ts         # Template execution with fspy monitoring
    │   ├── detector.ts         # Detect generated code patterns
    │   ├── migrator.ts         # Apply migrations with ast-grep
    │   ├── workspace.ts        # Monorepo integration
    │   ├── dependencies.ts     # Workspace dependency selection
    │   └── directory.ts        # Project directory detection
    ├── migrations/             # Migration rules (YAML)
    │   ├── vite-build.yaml
    │   ├── eslint-to-oxlint.yaml
    │   ├── vitest-config.yaml
    │   └── typescript-config.yaml
    ├── package.json
    └── tsconfig.json
```

**Key Dependencies**:

- `fspy` - File system monitoring for detecting created directories
- `@ast-grep/napi` - AST-based code transformation
- `@clack/prompts` - Beautiful CLI prompts
- `commander` - CLI argument parsing
- `yaml` - Parse pnpm-workspace.yaml
- `minimatch` or `micromatch` - Glob pattern matching for workspace patterns

### Template Discovery

Templates can be located in multiple places:

1. **Built-in scaffolds**: `@vite-plus/create-generator` - Scaffold for creating new generators
2. **Workspace packages**: Generators within the monorepo (e.g., `@company/generator-api`, `tools/create-microservice`)
3. **npm packages**: Any template from npm - bingo templates, create-\* templates, etc.
4. **Built-in Vite+**: Optional monorepo-specific generators (e.g., `vite:application`)

**Resolution Order:**

```
1. Check if name is "@vite-plus/create-generator" → generator scaffold
2. Check if name starts with "vite:" → built-in Vite+ generator
3. Check workspace packages for matching name → workspace-local generator
4. Check node_modules/{name}/package.json → installed template (any type)
5. Check if it's an npm package name → offer to install from registry
6. Error: template not found
```

**Template Type Detection**:

- **Bingo template**: Has `bingo` dependency or `bingo-template` keyword
- **Universal template**: Has `bin` entry in package.json
- Both types are executed the same way and get the same post-processing

**Example Workspace Structure:**

```
monorepo/
├── apps/                  # Applications (user can select)
│   └── web-app/
├── packages/              # Shared packages (user can select)
│   └── shared-lib/
├── services/              # Backend services (user can select)
├── tools/                 # Development tools (user can select)
│   └── generators/
│       ├── ui-lib/             # @company/generator-ui-lib
│       └── react-component/    # @company/generator-component
├── pnpm-workspace.yaml    # For pnpm
├── pnpm-lock.yaml         # Indicates pnpm usage
└── package.json           # Root package.json (workspaces field for npm/yarn/bun)
```

**Workspace Configuration Examples:**

**For pnpm (pnpm-workspace.yaml):**

```yaml
packages:
  - 'apps/*'
  - 'packages/*'
  - 'services/*'
  - 'tools/*'
```

**For npm/yarn/bun (package.json):**

```json
{
  "name": "my-monorepo",
  "private": true,
  "workspaces": ["apps/*", "packages/*", "services/*", "tools/*"]
}
```

**Detection Logic:**

1. Check for lock files to determine package manager
2. Read workspace config from appropriate file
3. Extract parent directories: `apps`, `packages`, `services`, `tools`
4. Prompt user to select one

### Template Execution Pipeline

Vite+ acts as an intelligent wrapper that:

1. **Pre-processing**:
   - Detect template type (bingo vs universal)
   - **If in monorepo**: Prompt for target directory (apps, packages, services, etc.)
   - Discover workspace packages for dependency selection
   - Capture pre-generation snapshot (for universal templates)

2. **Execution**:
   - Parse CLI arguments: options before `--` are for Vite+, options after `--` are for template
   - Execute the template using Node.js: `node node_modules/{template}/bin/index.js [args-after---]`
   - Pass through template arguments (everything after `--`)
   - Template runs with full interactivity

3. **Post-processing** (same for ALL templates):
   - **Detect & Migrate**: Analyze generated code with ast-grep
     - Detect standalone vite/vitest/oxlint/oxfmt
     - Prompt user to upgrade to vite-plus unified toolchain
     - Apply migration if confirmed
   - **Monorepo Integration**:
     - Prompt for workspace dependencies to add
     - Update generated package.json with selected dependencies
     - Update workspace config if needed (add to pnpm-workspace.yaml or package.json workspaces)
     - Run `vite install` to link workspace dependencies

**Implementation Note**:

- Vite+ CLI parses options before `--` (e.g., `--no-migrate`, `--deps`)
- Options after `--` are passed through to the template as-is
- No Rust-JS bridge needed - we shell out to Node.js to run templates

### Template Execution Flow

```
1. User runs: vite new [template-name] [vite-options] -- [template-options]
   ↓
2. Vite+ parses CLI arguments (split on -- separator)
   ↓
3. IF no template-name provided: Enter interactive mode
   ├─ Show template selection menu (Vite+ Monorepo, Vite+ Generator, Vite, TanStack, Other)
   ├─ Handle special templates with auto-argument injection
   └─ Continue with selected template
   ↓
4. Vite+ checks if running in a monorepo workspace
   ↓
5. IF in monorepo: Prompt user to select target directory (apps, packages, etc.)
   ↓
6. Vite+ discovers and identifies template type (bingo vs universal)
   ↓
7. Vite+ captures pre-generation snapshot (file list)
   ↓
8. Vite+ loads workspace packages for dependency selection
   ↓
9. Vite+ starts fspy watcher to monitor package.json operations
   ↓
10. Vite+ executes template: node node_modules/{template}/bin/index.js [template-options]
    (with cwd set to selected directory or passing directory as argument)
    ↓
11. Template runs (handles all prompts, validation, file generation)
    ↓
12. Template completes successfully
    ↓
13. Vite+ stops fspy watcher and derives project directory from package.json path
    ↓
14. Vite+ post-processes in detected project directory (same for ALL templates):

   AUTO-MIGRATE TO VITE-PLUS:
   ├─ Detect standalone vite/vitest/oxlint/oxfmt
   ├─ Prompt to upgrade to vite-plus unified toolchain
   └─ If yes, apply migration with ast-grep:
       ├─ Dependencies: vite + vitest + oxlint + oxfmt → vite-plus
       ├─ Merge vitest.config.ts → vite.config.ts
       ├─ Merge .oxlintrc → vite.config.ts
       ├─ Merge .oxfmtrc → vite.config.ts
       └─ Remove standalone config files

   MONOREPO INTEGRATION:
   ├─ Prompt user to select workspace dependencies
   ├─ Update package.json with workspace:* dependencies
   ├─ Check if project path matches workspace patterns
   ├─ If not matched: Update workspace config (pnpm-workspace.yaml or package.json)
   ├─ Run vite install to link workspace dependencies
   └─ Display next steps and tips
```

This approach is **simple and robust**:

- ✅ No need to embed a JavaScript runtime in Rust
- ✅ No need to maintain compatibility with template APIs
- ✅ Any template works out of the box (bingo or universal)
- ✅ Template authors can publish to npm normally
- ✅ Adds Vite+ optimization through intelligent migration
- ✅ Seamless monorepo integration

**Implementation Notes**:

- **GitHub Templates**: Uses degit via npx for zero-config cloning
- **Error Handling**: Provides context-specific troubleshooting tips
- **Update Mode**: Foundation in place for transition mode generators
- **Caching**: Relies on native npm/pnpm caching mechanisms

## Usage Examples

### Example 1: Universal Template (create-vite) with Auto-Execution

When a template is not installed locally, Vite+ automatically uses the appropriate package manager runner:

```bash
$ vite new create-vite -- --template react-ts

┌  🎨 Vite+ Code Generator
│
◇  Discovering template: create-vite
│
●  Template not installed locally, will run using pnpm dlx
│
◇  Executing template...
│
●  Running: pnpm dlx create-vite --template react-ts
│
# Template runs interactively via pnpm dlx...

# Vite+ prompts for target directory in monorepo
◆  Where should we create the new package?
│  ○ apps/        (Applications)
│  ● packages/    (Shared packages)
│  ○ services/    (Backend services)
│
◇  Selected: packages/
│
# Template prompts
✔ Project name: › my-react-app
✔ Select a framework: › React
✔ Select a variant: › TypeScript

Scaffolding project in ./packages/my-react-app...

Done. Now run:
  cd my-react-app
  vite install
  vite dev

# Vite+ detects standalone vite tools
◇  Template completed! Detecting vite-related tools...
│
◆  Detected standalone vite tools:
│  ✓ vite ^5.0.0
│  ✓ vitest ^1.0.0
│
◆  Upgrade to vite-plus unified toolchain?
│
│  This will:
│  • Replace vite + vitest with single vite-plus dependency
│  • Merge vitest.config.ts into vite.config.ts
│  • Remove standalone vitest.config.ts
│
│  Benefits:
│  • Unified dependency management
│  • Single configuration file
│  • Better integration with Vite+ task runner
│
│  ● Yes / ○ No
│
◇  Migrating to vite-plus...
│  ✓ Updated package.json (vite + vitest → vite-plus)
│  ✓ Merged vitest.config.ts → vite.config.ts
│  ✓ Removed vitest.config.ts
│
◆  Add workspace packages as dependencies?
│
│  ◼ @company/ui-components - Shared React components
│  ◼ @company/utils - Utility functions
│  ◻ @company/api-client - API client library
│
◇  Selected: @company/ui-components, @company/utils
│
◇  Updating packages/my-react-app/package.json...
◇  Added dependencies:
│  - @company/ui-components@workspace:*
│  - @company/utils@workspace:*
│
◇  Checking workspace configuration...
◇  Project matches pattern 'packages/*' ✓
◇  Running vite install...
│
└  Done!

🎉 Successfully created my-react-app with vite-plus

Next steps:
  cd packages/my-react-app
  vite dev
```

### Example 2: Complete Monorepo Integration Flow

This example shows the full monorepo integration with workspace dependency selection:

```bash
$ cd my-monorepo
$ vite new create-vite -- --template react-ts

┌  🎨 Vite+ Code Generator
│
◆  Where should we create the new package?
│  ○ apps/
│  ● packages/
│  ○ tools/
│
◇  Selected: packages/
│
◇  Discovering template: create-vite
│
●  Template not installed locally, will run using pnpm dlx
│
◇  Executing template...
│
●  Running: pnpm dlx create-vite --template react-ts
│
# create-vite runs interactively...
✔ Project name: › ui-components
✔ Select a framework: › React
✔ Select a variant: › TypeScript

Scaffolding project in ./ui-components...

Done. Now run:
  cd ui-components
  npm install
  npm run dev

◆  Template executed successfully
│
◆  Detected project directory: packages/ui-components
│
◇  Auto-migration to Vite+...
│
●  Detected standalone vite tools: vite, vitest
│
◆  Upgrade to vite-plus unified toolchain?
│  ● Yes / ○ No
│
●  This will:
│  • Replace vite + vitest with single vite dependency
│  • Update script commands to use vite CLI
│  • Use catalog: version
│
◆  Migrated to vite-plus ✓
│  • Removed: vite, vitest
│  • Added: vite (catalog:)
│
◇  Monorepo integration...
│
◆  Add workspace packages as dependencies?
│  ◼ @company/utils - Utility functions
│  ◼ @company/theme - Design tokens and theme
│  ◻ @company/icons - Icon library
│  ◻ @company/api-client - API client
│
◇  Selected: @company/utils, @company/theme
│
◆  Added 2 workspace dependencies
│  • @company/utils@workspace:*
│  • @company/theme@workspace:*
│
◆  Project matches workspace pattern ✓
│
◒  Running vite install...
│
◆  Dependencies linked
│
└  ✨ Generation completed!

Next steps:
  cd packages/ui-components
  vite dev
```

### Example 3: Creating a Generator Scaffold

Use the built-in `vite:generator` to quickly scaffold a new generator:

```bash
$ vite new vite:generator

┌  🎨 Vite+ Code Generator
│
◇  Discovering template: vite:generator
│
◆  Found builtin template: vite:generator
│
◇  Creating generator scaffold...
│
◇  Generator name:
│  ui-lib
│
◇  Package name:
│  @company/generator-ui-lib
│
◇  Description:
│  Generate new UI component libraries
│
◇  Where to create?
│  tools/generators/ui-lib
│
◆  Generator scaffold created
│  • package.json
│  • bin/index.js
│  • src/template.ts
│  • README.md
│  • tsconfig.json
│
◆  Detected project directory: tools/generators/ui-lib
│
◇  Monorepo integration...
│
◆  Project doesn't match existing workspace patterns
│
◆  Update workspace configuration to include this project?
│  ● Yes
│
◆  Updated workspace configuration
│
◒  Running vite install...
│
◆  Dependencies linked
│
└  ✨ Generation completed!

Summary:
  • Template: vite:generator (builtin)
  • Created: tools/generators/ui-lib
  • Actions: Updated workspace config

Next steps:
  cd tools/generators/ui-lib
  # Edit src/template.ts to customize your generator
  # Then test it with: vite new @company/generator-ui-lib
```

The generated scaffold includes:

- **package.json**: Pre-configured with bingo, zod, bin entry, keywords
- **bin/index.js**: Executable entry point that runs the template
- **src/template.ts**: Example bingo template with TODO comments for customization
- **README.md**: Usage instructions and development guide
- **tsconfig.json**: TypeScript configuration extending monorepo root

### Example 4: Built-in vite:application Generator

Use the built-in `vite:application` generator for a Vite+ optimized project:

```bash
$ vite new vite:application

┌  🎨 Vite+ Code Generator
│
◇  Discovering template: vite:application
│
◆  Found builtin template: vite:application
│
◇  Creating vite application...
│
◆  Select a framework:
│  ● React (React with TypeScript)
│  ○ Vue (Vue with TypeScript)
│  ○ Svelte (Svelte with TypeScript)
│  ○ Solid (Solid with TypeScript)
│  ○ Vanilla (Vanilla TypeScript)
│
◇  Project name:
│  my-app
│
◇  Generating react-ts project...
│
# create-vite runs...
│
◆  Project generated with Vite+ configuration
│  • Added vite-task.json with build/test/lint/dev tasks
│
◆  Detected project directory: my-app
│
◇  Auto-migration to Vite+...
│
●  Detected standalone vite tools: vite
│
◆  Migrated to vite-plus ✓
│
└  ✨ Generation completed!

Summary:
  • Template: vite:application (builtin)
  • Created: my-app
  • Actions: Migrated to vite-plus

Next steps:
  cd my-app
  vite dev
```

The generated project includes:

- Standard create-vite project structure
- **vite-task.json**: Pre-configured tasks (build, test, lint, dev)
- **Migrated**: Already using vite-plus instead of standalone vite
- **Ready**: Immediately usable with Vite+ task runner

### Example 5: Bingo Template (create-typescript-app)

```bash
# Use create-typescript-app (a popular bingo template)
vite new create-typescript-app

┌  vite new create-typescript-app
│
# Vite+ prompts for target directory first
◆  Where should we create the new package?
│  ○ apps/
│  ● packages/
│  ○ tools/
│
◇  Selected: packages/
│
# Bingo's interactive prompts
◇  Repository name: my-lib
◇  Repository owner: mycompany
◇  Which preset? › common
│
└  Template completed successfully!

# Vite+ ALSO detects standalone vite tools (even for bingo templates)
◇  Template completed! Detecting vite-related tools...
│
◆  Detected standalone vite tools:
│  ✓ vite ^5.0.0
│  ✓ vitest ^1.0.0
│
◆  Upgrade to vite-plus unified toolchain?
│
│  This will:
│  • Replace vite + vitest with single vite-plus dependency
│  • Merge vitest.config.ts into vite.config.ts
│
│  ● Yes / ○ No
│
◇  Migrating to vite-plus...
│  ✓ Updated package.json dependencies
│  ✓ Merged vitest.config.ts → vite.config.ts
│  ✓ Removed vitest.config.ts
│
◆  Add workspace packages as dependencies?
│
│  ◼ @mycompany/utils - Utility functions
│  ◼ @mycompany/logger - Logging library
│  ◻ @mycompany/database - Database client
│
◇  Selected: @mycompany/utils, @mycompany/logger
│
◇  Updating packages/my-lib/package.json...
◇  Checking workspace configuration...
◇  Project matches pattern 'packages/*' ✓
◇  Running vite install...
│
└  Done!

🎉 Successfully created my-lib with Vite+ optimizations

Next steps:
  cd packages/my-lib
  vite dev
```

**Notice**: Even though create-typescript-app is a bingo template, it still gets the same auto-migration treatment to optimize for Vite+!

### Example 3: Creating a Workspace-Local Bingo Generator

#### Quick Start with @vite-plus/create-generator

Use the official scaffold to quickly create a new generator:

```bash
# Create a new generator in your monorepo
vite new @vite-plus/create-generator

┌  @vite-plus/create-generator
│
◇  Generator name: ui-lib
◇  Generator package name: @company/generator-ui-lib
◇  Description: Generate new UI component libraries for our monorepo
◇  Where to create? › tools/generators/ui-lib
│
◇  Creating generator scaffold...
│  ✓ Created package.json
│  ✓ Created bin/index.js
│  ✓ Created src/template.ts
│  ✓ Created src/template.test.ts
│  ✓ Created README.md
│
◇  Installing dependencies...
│
└  Done!

✅ Generator created at tools/generators/ui-lib

Next steps:
  1. cd tools/generators/ui-lib
  2. Edit src/template.ts to define your generator logic
  3. Test with: vite new @company/generator-ui-lib
```

#### Generated Structure

In a real monorepo, write custom bingo generators as proper packages alongside your apps and libraries:

```
monorepo/
├── apps/
│   ├── api-gateway/
│   └── web-app/
├── packages/                    # Generated packages go here
├── tools/
│   └── generators/
│       └── ui-lib/              # The generator package
│           ├── package.json
│           ├── bin/
│           │   └── index.js
│           ├── src/
│           │   └── template.ts
│           └── templates/
│               ├── package.json.hbs
│               └── src/
│                   └── index.ts.hbs
└── pnpm-workspace.yaml
```

**Generator Package Configuration** (auto-generated by `@vite-plus/create-generator`):

```json
// tools/generators/ui-lib/package.json
{
  "name": "@company/generator-ui-lib",
  "version": "1.0.0",
  "type": "module",
  "private": true,
  "description": "Generate new UI component libraries for our monorepo",
  "bin": {
    "create-ui-lib": "./bin/index.js"
  },
  "keywords": ["bingo-template", "vite-plus-generator"],
  "scripts": {
    "test": "vitest"
  },
  "dependencies": {
    "bingo": "^0.5.0",
    "zod": "^3.22.0"
  },
  "devDependencies": {
    "bingo-testers": "^0.5.0",
    "vitest": "^1.0.0",
    "@types/node": "^20.0.0",
    "typescript": "^5.3.0"
  }
}
```

**Template Implementation** (scaffold provided by `@vite-plus/create-generator`):

The scaffold includes a complete, working example that you can customize:

```typescript
// tools/generators/ui-lib/src/template.ts
import { createTemplate } from 'bingo';
import { z } from 'zod';

// This file is scaffolded by @vite-plus/create-generator
// Edit the options and produce() function to customize your generator

export default createTemplate({
  about: {
    name: 'UI Library Generator',
    description: 'Create a new React component library with TypeScript',
  },

  options: {
    name: z
      .string()
      .regex(/^[a-z][a-z0-9-]*$/, 'Must be lowercase with hyphens')
      .describe('Library name (e.g., design-system, ui-components)'),

    framework: z.enum(['react', 'vue', 'svelte']).default('react').describe('UI framework'),

    storybook: z.boolean().default(true).describe('Include Storybook for component documentation'),

    cssInJs: z.boolean().default(false).describe('Include CSS-in-JS library (styled-components)'),
  },

  async produce({ options }) {
    const libPath = `packages/${options.name}`;
    const packageName = `@company/${options.name}`;

    return {
      files: {
        [`${libPath}/package.json`]: JSON.stringify(
          {
            name: packageName,
            version: '0.1.0',
            type: 'module',
            private: true,
            main: './dist/index.js',
            module: './dist/index.mjs',
            types: './dist/index.d.ts',
            exports: {
              '.': {
                import: './dist/index.mjs',
                require: './dist/index.js',
                types: './dist/index.d.ts',
              },
            },
            scripts: {
              dev: 'vite',
              build: 'vite build && tsc --emitDeclarationOnly',
              test: 'vitest',
              lint: 'oxlint',
              ...(options.storybook && { storybook: 'storybook dev -p 6006' }),
            },
            peerDependencies: {
              [options.framework]: '^18.0.0',
            },
            dependencies: {
              ...(options.cssInJs && { 'styled-components': '^6.0.0' }),
            },
            devDependencies: {
              '@types/react': '^18.0.0',
              '@vitejs/plugin-react': '^4.2.0',
              typescript: '^5.3.0',
              vitest: '^1.0.0',
              vite: '^5.0.0',
              ...(options.storybook && {
                '@storybook/react': '^7.6.0',
                '@storybook/react-vite': '^7.6.0',
              }),
            },
          },
          null,
          2,
        ),

        [`${libPath}/tsconfig.json`]: JSON.stringify(
          {
            extends: '../../tsconfig.base.json',
            compilerOptions: {
              outDir: './dist',
              rootDir: './src',
              declaration: true,
              declarationMap: true,
            },
            include: ['src/**/*'],
          },
          null,
          2,
        ),

        [`${libPath}/vite.config.ts`]: `
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  build: {
    lib: {
      entry: './src/index.ts',
      formats: ['es', 'cjs'],
      fileName: (format) => \`index.\${format === 'es' ? 'mjs' : 'js'}\`,
    },
    rollupOptions: {
      external: ['react', 'react-dom'],
    },
  },
});
        `.trim(),

        [`${servicePath}/src/index.ts`]: `
import express from 'express';
${options.authentication ? "import { authMiddleware } from './middleware/auth.js';" : ''}
${options.database !== 'none' ? `import { initDatabase } from './database.js';` : ''}

const app = express();
const PORT = ${options.port};

// Middleware
app.use(express.json());
${options.authentication ? 'app.use(authMiddleware);' : ''}

// Routes
app.get('/health', (req, res) => {
  res.json({
    status: 'ok',
    service: '${options.name}',
    timestamp: new Date().toISOString(),
  });
});

app.get('/api/${options.name}', (req, res) => {
  res.json({ message: 'Hello from ${options.name}!' });
});

// Start server
async function start() {
  ${options.database !== 'none' ? 'await initDatabase();' : ''}

  app.listen(PORT, () => {
    console.log(\`🚀 ${options.name} running on http://localhost:\${PORT}\`);
  });
}

start().catch(console.error);
        `.trim(),

        ...(options.database !== 'none' && {
          [`${servicePath}/src/database.ts`]: `
import { ${getDatabaseClient(options.database)} } from '${getDatabasePackage(options.database)}';

export async function initDatabase() {
  // TODO: Initialize ${options.database} connection
  console.log('📦 Database connected');
}
          `.trim(),
        }),

        ...(options.authentication && {
          [`${servicePath}/src/middleware/auth.ts`]: `
import { Request, Response, NextFunction } from 'express';

export function authMiddleware(req: Request, res: Response, next: NextFunction) {
  // TODO: Implement JWT authentication
  next();
}
          `.trim(),
        }),

        [`${servicePath}/.env.example`]: [
          `PORT=${options.port}`,
          `NODE_ENV=development`,
          options.database !== 'none' &&
            `DATABASE_URL=${getDatabaseUrl(options.database, options.name)}`,
        ]
          .filter(Boolean)
          .join('\n'),

        [`${servicePath}/README.md`]: `
# ${packageName}

API microservice for ${options.name}.

## Development

\`\`\`bash
# Install dependencies
vite install

# Start dev server
vite dev

# Run tests
vite test

# Build
vite build
\`\`\`

## Configuration

- Port: ${options.port}
- Database: ${options.database}
- Authentication: ${options.authentication ? 'Enabled' : 'Disabled'}
        `.trim(),
      },

      scripts: [
        {
          phase: 0,
          commands: [`cd ${libPath}`, 'vite install'],
        },
      ],

      suggestions: [
        `✅ Created ${options.name} component library in ${libPath}`,
        ``,
        `Next steps:`,
        `  1. cd ${libPath}`,
        `  2. Add your components to src/components/`,
        `  3. Export them in src/index.ts`,
        `  4. vite build`,
        options.storybook && `  5. npm run storybook (view component docs)`,
        ``,
        `The library is ready to be used in other packages!`,
      ].filter(Boolean),
    };
  },
});
```

**CLI Entrypoint** (auto-generated by `@vite-plus/create-generator`):

```javascript
#!/usr/bin/env node
// tools/generators/ui-lib/bin/index.js
import { runTemplate } from 'bingo';
import template from '../src/template.js';

runTemplate(template);
```

**README** (auto-generated by `@vite-plus/create-generator`):

```markdown
# @company/generator-ui-lib

Generate new UI component libraries for our monorepo.

## Usage

From monorepo root:

\`\`\`bash
vite new @company/generator-ui-lib
\`\`\`

With options:

\`\`\`bash
vite new @company/generator-ui-lib --name=design-system --framework=react
\`\`\`

## Development

\`\`\`bash

# Run tests

vite test

# Test the generator

vite new @company/generator-ui-lib
\`\`\`

## Customization

Edit `src/template.ts` to customize:

- Options schema (using Zod)
- File generation logic
- Scripts and suggestions
```

**Usage in the Monorepo:**

```bash
# Run from monorepo root
vite new @company/generator-ui-lib

# Bingo generator prompts
┌  @company/generator-ui-lib
│
◇  Library name: design-system
◇  Framework: React
◇  Include Storybook? Yes
◇  Include CSS-in-JS? No
│
└  Template completed!

# Vite+ ALSO detects standalone vite tools (even for bingo templates!)
◇  Template completed! Detecting vite-related tools...
│
◆  Detected standalone vite tools:
│  ✓ vite ^5.0.0
│  ✓ vitest ^1.0.0
│
◆  Upgrade to vite-plus unified toolchain?
│
│  This will:
│  • Replace vite + vitest with vite-plus
│  • Merge configs into vite.config.ts
│
│  ● Yes / ○ No
│
◇  Migrating to vite-plus...
│  ✓ Updated package.json dependencies
│  ✓ Merged vitest.config.ts → vite.config.ts
│  ✓ Removed standalone config files
│
◆  Add workspace packages as dependencies?
│
│  ◼ @company/theme - Design tokens and theme
│  ◼ @company/utils - Utility functions
│  ◼ @company/icons - Icon library
│  ◻ @company/hooks - React hooks
│
◇  Selected: @company/theme, @company/utils, @company/icons
│
◇  Updating packages/design-system/package.json...
◇  Checking workspace configuration...
◇  Project matches pattern 'packages/*' ✓
◇  Running vite install...
│
└  Done!

✅ Created design-system component library with Vite+ optimizations

Next steps:
  1. cd packages/design-system
  2. Add components to src/components/
  3. Export in src/index.ts
  4. vite build
  5. npm run storybook (view docs)

# CLI options
vite new @company/generator-ui-lib --name=icons --no-migrate  # Skip migrations
vite new @company/generator-ui-lib --name=hooks --deps=@company/utils  # Pre-select deps
```

**Key Point**: Even your own bingo generators benefit from auto-migration! You can generate code using standalone vite/vitest/oxlint, and Vite+ will automatically consolidate them into vite-plus.

**Tip**: Use `vite new @vite-plus/create-generator` to quickly scaffold a new generator in your monorepo!

**Testing the Generator:**

```typescript
// tools/generators/ui-lib/src/template.test.ts
import { testTemplate } from 'bingo/testers';
import { describe, expect, it } from 'vitest';
import template from './template.js';

describe('UI Library Generator', () => {
  it('generates library with storybook', async () => {
    const result = await testTemplate(template, {
      options: {
        name: 'design-system',
        framework: 'react',
        storybook: true,
        cssInJs: false,
      },
    });

    expect(result.files['packages/design-system/package.json']).toContain('@company/design-system');
    expect(result.files['packages/design-system/package.json']).toContain('@storybook/react');
    expect(result.files['packages/design-system/src/components/Button.tsx']).toBeDefined();
    expect(result.files['packages/design-system/.storybook/main.ts']).toBeDefined();
  });

  it('generates library without storybook', async () => {
    const result = await testTemplate(template, {
      options: {
        name: 'icons',
        framework: 'react',
        storybook: false,
        cssInJs: false,
      },
    });

    expect(result.files['packages/icons/.storybook/main.ts']).toBeUndefined();
    expect(result.files['packages/icons/src/components/Button.stories.tsx']).toBeUndefined();
  });
});
```

### Built-in Vite+ Generator (Optional)

For monorepo-specific needs, Vite+ can provide thin wrappers:

```bash
# Built-in generator that configures vite-task.json automatically
vite new vite:library --name=shared-utils

# This could wrap an existing bingo template and add:
# - vite-task.json with build/test/lint tasks
# - Proper workspace structure
# - TypeScript configuration for monorepo
```

## Technical Considerations

### 1. Process Execution & Directory Detection

- **Subprocess Management**: Use Node.js `child_process.spawn()` to execute templates
- **Stdio Handling**: Use `inherit` mode to pass through stdin/stdout/stderr for interactive prompts
- **Directory Detection**: Use `fspy` to monitor package.json operations during template execution
  - Watch specifically for package.json write/create operations
  - Capture package.json file path when written
  - Derive project root from package.json path (extract parent directory)
  - Handle multiple package.json writes (choose the first top-level one)
  - Handle in-place generation (package.json created in cwd)
- **Exit Codes**: Properly handle template exit codes and surface errors
- **Working Directory**: Use `cwd` option to ensure template runs in the correct directory

### 2. Package Manager Detection & Workspace Config

- **Auto-detect**: Determine package manager from lock files
  - `pnpm-lock.yaml` → pnpm
  - `package-lock.json` → npm
  - `yarn.lock` → yarn
  - `bun.lockb` → bun
- **Read Workspace Config**: Based on detected package manager
  - **pnpm**: Read `pnpm-workspace.yaml`, parse `packages` array
  - **npm/yarn/bun**: Read root `package.json`, parse `workspaces` array
- **Respect Workspace**: Use the same package manager as the monorepo
- **Installation**: When prompting to install, use the detected package manager

### 3. Template Detection

- **package.json Parsing**: Read package.json to check for bingo dependency
- **Bin Entry**: Look for bin field to find the executable
- **Keywords**: Check for "bingo-template" keyword as fallback
- **Validation**: Warn if package doesn't look like a valid bingo template

### 4. Monorepo Integration

- **Workspace Detection**: Check if running in a monorepo workspace
- **Directory Selection**: Prompt user to select target directory (apps, packages, services, etc.)
  - Read workspace config (pnpm-workspace.yaml or package.json workspaces)
  - Extract parent directories from workspace patterns
  - Present as interactive selection
  - Can be overridden with `--directory` flag
- **Workspace Package Discovery**: Load all packages from workspace with their metadata
  - Use detected package manager's workspace patterns
  - Resolve glob patterns to find all packages
- **Dependency Selection UI**: Multi-select prompt for choosing workspace dependencies
- **Smart Filtering**: Filter packages by type (exclude generators, include libraries)
- **Version Protocol**: Use `workspace:*` for workspace dependencies
- **Package.json Updates**: Parse and update generated package.json with selected deps
- **Workspace Registration**: Check if detected project matches existing workspace patterns
  - If matches (e.g., `packages/my-app` matches `packages/*`): ✅ No update needed
  - If not matched: Update workspace config file
    - **pnpm**: Add pattern to pnpm-workspace.yaml `packages` array
    - **npm/yarn/bun**: Add pattern to package.json `workspaces` array
- **Dependency Installation**: Run `vite install` to link workspace dependencies
- **Path Normalization**: Ensure paths are relative to workspace root
- **Idempotency**: Don't duplicate patterns if already exist

### 5. Error Handling

- **Clear Messages**: Distinguish between Vite+ errors and template errors
- **Installation Failures**: Handle vite install failures gracefully
- **Partial Completion**: If template creates files but errors, inform user
- **Troubleshooting**: Provide hints for common issues (Node.js not found, etc.)

### 6. Testing

Testing can leverage bingo's own test utilities:

```typescript
// Template authors test using bingo's testing tools
import { testTemplate } from 'bingo/testers';
import { expect, test } from 'vitest';
import template from './template';

test('generates React app', async () => {
  const result = await testTemplate(template, {
    options: { name: 'my-app', framework: 'react' },
  });

  expect(result.files['package.json']).toContain('react');
});
```

Vite+ template runner logic is also tested using TypeScript/Vitest:

```typescript
// In vite_generator package
import { describe, expect, it } from 'vitest';
import { detectBingoTemplate, loadWorkspacePackages } from './discovery';

describe('Template Detection', () => {
  it('detects bingo template from package.json', async () => {
    const pkg = {
      name: 'create-typescript-app',
      dependencies: { bingo: '^0.5.0' },
      bin: { 'create-typescript-app': './bin/index.js' },
    };

    expect(detectBingoTemplate(pkg)).toBe(true);
  });

  it('detects bingo template from keywords', async () => {
    const pkg = {
      name: 'my-template',
      keywords: ['bingo-template'],
      bin: { 'my-template': './index.js' },
    };

    expect(detectBingoTemplate(pkg)).toBe(true);
  });
});

describe('Workspace Package Discovery', () => {
  it('loads all workspace packages', async () => {
    const packages = await loadWorkspacePackages('/path/to/monorepo');

    expect(packages).toContainEqual({
      name: '@company/logger',
      path: 'packages/logger',
      description: 'Logging library',
    });
  });

  it('filters out generator packages', async () => {
    const packages = await loadWorkspacePackages('/path/to/monorepo', {
      excludeGenerators: true,
    });

    expect(packages.every((pkg) => !pkg.name.includes('generator'))).toBe(true);
  });
});
```

## Comparison: Bingo vs Universal Templates

| Aspect                 | Bingo Templates              | Universal Templates              |
| ---------------------- | ---------------------------- | -------------------------------- |
| **Writing Experience** | ✅ Type-safe (Zod), testable | ⚠️ No standard, varies           |
| **Examples**           | @company/generator-ui-lib    | create-vite, create-next-app     |
| **Customization**      | ✅ Full control              | ⚠️ Limited to template options   |
| **Auto-Migration**     | ✅ Yes (same as universal)   | ✅ Yes (same as bingo)           |
| **Ecosystem**          | ~5-10 bingo templates        | Thousands of create-\* templates |
| **Learning Curve**     | Medium (learn bingo)         | Zero (use familiar templates)    |
| **Maintenance**        | Maintained by you            | Maintained by template authors   |
| **Best For**           | Custom company generators    | Quick starts, standard setups    |

**Key Point**: Both bingo and universal templates get the **same auto-migration to vite-plus**. The difference is only in the authoring experience:

- **Choose bingo** when you want to write custom generators with type safety and testing
- **Choose universal** when you want to use existing templates from the ecosystem
- **Both get auto-migrated** to vite-plus unified toolchain automatically!

## Open Questions

1. **Auto-installation UX**: Should we auto-install templates or always prompt first?
2. **Template Caching**: Should we cache installed templates or always fetch latest?
3. **Built-in Generators**: Which built-in generators (if any) should we provide?
4. **Directory Selection**:
   - Should we infer descriptions from directory names (e.g., "apps" → "Applications")?
   - Should we support adding custom directories via `--directory=custom/path`?
   - Should we remember last selected directory for next generation?
5. **Dependency Selection**:
   - Should we filter packages by type (e.g., exclude test utilities)?
   - Should we have smart defaults based on generator type?
   - Should we support dependency groups (e.g., "common backend libs")?
6. **Version Protocol**: Always use `workspace:*` or allow specific version ranges?
7. **Migration Safety**:
   - Should we create backups before applying migrations?
   - How to handle migration conflicts?
   - Support rollback of migrations?
8. **ast-grep Integration**:
   - Should migration rules be YAML or TypeScript?
   - Should we allow custom user-defined migrations?
9. **Extensibility**:
   - Plugin system for third-party migrations?
   - How to share migrations across teams?

## Success Criteria

A successful implementation should:

### Template Support

1. ✅ Run ANY bingo template from npm without modification (via npx/pnpm dlx)
2. ✅ Run ANY create-\* or other universal templates (via npx/pnpm dlx)
3. ✅ Support workspace-local bingo generators (scan workspace patterns)
4. ✅ Auto-detect template type (bingo vs universal)
5. ✅ Parse CLI arguments correctly (Vite+ options before `--`, template options after `--`)
6. ✅ Pass through template options correctly (everything after `--`)
7. ✅ Handle interactive prompts properly (stdio inheritance)
8. ✅ Detect generated project directory (directory scanning for package.json)
9. ✅ Built-in @vite-plus/create-generator for scaffolding new generators
10. ✅ Built-in vite:application and vite:library placeholders

### Auto-Migration to vite-plus

9. ✅ Automatically detect standalone vite/vitest/oxlint/oxfmt (in detected project directory)
10. ✅ Prompt to upgrade to vite-plus unified toolchain
11. ✅ Consolidate dependencies (vite + vitest + oxlint + oxfmt → unified vite)
12. ✅ Update script commands (vitest → vite test, oxlint → vite lint, etc.)
13. ✅ Provide clear before/after explanations
14. ✅ Be safe and reversible
15. ⏳ Merge configurations (vitest.config.ts, .oxlintrc, .oxfmtrc → vite.config.ts) - Future enhancement with ast-grep

### Monorepo Integration

16. ✅ Detect monorepo workspace and prompt for target directory
17. ✅ Support `--directory` flag to skip directory selection
18. ✅ Integrate with workspace (auto-update pnpm-workspace.yaml or package.json workspaces if needed)
19. ✅ Prompt for workspace dependencies with multi-select UI
20. ✅ Use `workspace:*` protocol for internal dependencies
21. ✅ Run `vite install` to link dependencies
22. ✅ Work with npm, pnpm, yarn, and bun package managers
23. ✅ Smart package filtering (exclude generators from dependency selection)
24. ✅ Support `--deps` flag for pre-selecting dependencies
25. ✅ Workspace pattern matching and auto-update

### Developer Experience

26. ✅ No installation required (use npx/pnpm dlx/yarn dlx/bunx)
27. ✅ Clear error messages distinguishing Vite+ vs template errors
28. ✅ Beautiful interactive prompts with @clack/prompts
29. ✅ Show progress and feedback during generation/migration
30. ✅ Display helpful next steps after completion (with correct directory path)
31. ✅ Interactive mode with curated template selection
32. ✅ Automatic argument injection for known templates

## Benefits of This Approach

### For Vite+ Users

- 🎯 **Maximum Choice**: Use bingo templates OR any create-\* template
- 🚀 **Zero Learning Curve**: Use familiar templates (create-vite, create-next-app, etc.)
- 🔧 **Automatic Optimization**: Intelligent migration to Vite+ toolchain
- 🌍 **Entire Ecosystem**: Access to thousands of existing templates
- 💼 **Company Generators**: Build reusable bingo templates for your team
- 🔄 **Future-proof**: Works with templates created in the future

### For Template Authors

**Bingo Template Authors:**

- ✍️ **Full Control**: Type-safe, testable, company-specific generators
- 📦 **Monorepo-first**: Perfect for workspace-local generators
- 🧪 **Built-in Testing**: Use bingo's testing utilities
- 🚀 **Quick Start**: Use `@vite-plus/create-generator` to scaffold new generators

**Universal Template Authors:**

- ⚡ **Zero Effort**: Templates work as-is
- 👥 **Wider Audience**: Automatically compatible with Vite+
- 🔄 **No Maintenance**: Vite+ handles the optimization

### For Vite+ Maintainers

- 🎯 **Best of Both Worlds**: Support both approaches
- 🐛 **Simpler Architecture**: Templates run as-is, no complex API
- 📚 **Leverage Docs**: Point to existing template documentation
- ⚡ **Immediate Value**: Add value through intelligent post-processing
- 🔧 **Extensible**: Easy to add new migrations as Vite+ evolves

## Related RFCs

- [migration-command.md](./migration-command.md) - `vite migration` command for migrating existing projects
  - Shares the same migration engine and rules
  - `vite new` runs migrations after template generation
  - `vite migration` runs migrations on existing projects

## References

### Template Frameworks

- [Bingo Framework](https://www.create.bingo/) - Type-safe repository templates
- [Bingo FAQs](https://www.create.bingo/faqs/)
- [create-typescript-app](https://github.com/JoshuaKGoldberg/create-typescript-app) - Production bingo template
- [create-vite](https://github.com/vitejs/vite/tree/main/packages/create-vite) - Official Vite templates

### Code Transformation

- [ast-grep](https://ast-grep.github.io/) - Structural search and replace tool
- [Turborepo Codemods](https://turborepo.com/docs/reference/turbo-codemod) - Similar migration approach
- [jscodeshift](https://github.com/facebook/jscodeshift) - Alternative AST transformation tool

### Inspiration

- [Nx Generators](https://nx.dev/docs/features/generate-code) - Nx's generator system
- [Turborepo Code Generation](https://turborepo.com/docs/guides/generating-code) - Turbo's PLOP-based approach
- [PLOP Documentation](https://plopjs.com/documentation/) - Micro-generator framework

### Tools

- [Zod](https://zod.dev/) - TypeScript schema validation
- [@clack/prompts](https://www.npmjs.com/package/@clack/prompts) - Beautiful CLI prompts

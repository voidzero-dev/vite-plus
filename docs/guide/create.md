# Creating a Project

`vp create` interactively scaffolds new Vite+ projects, monorepos, and apps inside existing workspaces.

## Overview

The `create` command is the fastest way to start with Vite+. It can be used in a few different ways:

- Start a new Vite+ monorepo
- Create a new standalone application or library
- Add a new app or library inside an existing project

This command can be used with built-in templates, community templates, or remote GitHub templates.

## Usage

```bash
vp create
vp create <template>
vp create <template> -- <template-options>
```

## Built-in Templates

Vite+ ships with these built-in templates:

- `vite:monorepo` creates a new monorepo
- `vite:application` creates a new application
- `vite:library` creates a new library
- `vite:generator` creates a new generator

## Template Sources

`vp create` is not limited to the built-in templates.

- Use shorthand templates like `vite`, `@tanstack/start`, `svelte`, `next-app`, `nuxt`, `react-router`, and `vue`
- Use full package names like `create-vite` or `create-next-app`
- Use local templates such as `./tools/create-ui-component` or `@your-org/generator-*`
- Use remote templates such as `github:user/repo` or `https://github.com/user/template-repo`

Run `vp create --list` to see the built-in templates and the common shorthand templates Vite+ recognizes.

## Options

- `--directory <dir>` writes the generated project into a specific target directory
- `--agent <name>` creates agent instructions files during scaffolding
- `--editor <name>` writes editor config files
- `--hooks` enables pre-commit hook setup
- `--no-hooks` skips hook setup
- `--no-interactive` runs without prompts
- `--verbose` shows detailed scaffolding output
- `--list` prints the available built-in and popular templates

## Template Options

Arguments after `--` are passed directly to the selected template.

This matters when the template itself accepts flags. For example, you can forward Vite template selection like this:

```bash
vp create vite -- --template react-ts
```

## Examples

```bash
# Interactive mode
vp create

# Create a Vite+ monorepo, application, library, or generator
vp create vite:monorepo
vp create vite:application
vp create vite:library
vp create vite:generator

# Use shorthand community templates
vp create vite
vp create @tanstack/start
vp create svelte

# Use full package names
vp create create-vite
vp create create-next-app

# Use remote templates
vp create github:user/repo
vp create https://github.com/user/template-repo
```

## Organization Templates

An organization can publish a curated set of templates under a single npm scope by shipping an `@org/create` package whose `package.json` carries a `createConfig.templates` manifest. Once published, `vp create @org` opens an interactive picker over those templates.

### Pick from an org

```bash
# Open an interactive picker over @your-org/create's manifest
vp create @your-org

# Run a specific manifest entry directly
vp create @your-org:web

# Pin to an exact version or a dist-tag
vp create @your-org@1.2.3
vp create @your-org:web@next

# Set the org as the default for a repo (see create.defaultTemplate config)
vp create
```

Behind the scenes, `vp create @org` maps to `@org/create` (the existing npm `create-*` convention). If that package has no `createConfig.templates` field, Vite+ falls back to running the package normally — so adopting the manifest is zero-risk for orgs that already publish `@org/create`.

Private registries work automatically: Vite+ reads `.npmrc` files from the project root and `~/`, honoring `@your-org:registry=...` scope mappings and `//host/:_authToken=...` credentials.

### Authoring `@org/create`

There are two common layouts. Pick the one that matches the org's template count and release cadence.

**Bundled (recommended for most orgs).** All templates live as subdirectories of `@org/create` itself. Manifest entries use relative `./path` values. One repo, one publish, one versioning story — the same pattern used by `create-vite` and `create-next-app`.

```
@your-org/create/
├── package.json              # "createConfig": { "templates": [{ "template": "./templates/web" }, ...] }
├── templates/
│   ├── web/
│   │   ├── package.json
│   │   └── src/...
│   └── library/...
└── README.md
```

**Manifest-only.** When the org already publishes independent `@org/template-*` packages (or hosts them on GitHub), `@org/create` stays a thin index.

```
@your-org/create/
├── package.json              # "createConfig": { "templates": [{ "template": "@your-org/template-web" }, ...] }
└── README.md
```

The two layouts can be mixed — a manifest can point most entries at external packages and keep a few as bundled subdirectories.

Optionally, provide a `bin` script so `npm create @org` (the legacy path) keeps working for non-Vite+ users. `vp create @org` reads the manifest directly and never runs the `bin`.

### Manifest schema

The manifest lives at `createConfig.templates` in `@org/create`'s `package.json`:

```json
{
  "name": "@your-org/create",
  "version": "1.0.0",
  "createConfig": {
    "templates": [
      {
        "name": "monorepo",
        "description": "Monorepo",
        "template": "@your-org/template-monorepo",
        "monorepo": true
      },
      {
        "name": "web",
        "description": "Web app template (Vite + React)",
        "template": "@your-org/template-web"
      },
      {
        "name": "demo",
        "description": "Bundled demo template",
        "template": "./templates/demo"
      }
    ]
  }
}
```

Each entry supports:

| Field         | Required | Notes                                                                                                                                                                                                                                        |
| ------------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `name`        | yes      | Kebab-case identifier. Used by `vp create @org:<name>` for direct selection. Must be unique within the array.                                                                                                                                |
| `description` | yes      | One-line description shown in the picker.                                                                                                                                                                                                    |
| `template`    | yes      | An npm specifier (`@org/template-foo`, optionally `@version`), a GitHub URL (`github:user/repo`), a `vite:*` builtin, a local workspace package name, or a relative path (`./templates/foo`) that resolves against the `@org/create` root. |
| `monorepo`    | no       | If `true`, marks this entry as a monorepo-creating template. Hidden from the picker when `vp create` runs inside an existing monorepo, mirroring the built-in `vite:monorepo` filter.                                                        |

An invalid manifest is a hard error, not a silent fall-through — a maintainer who shipped a manifest should hear about the offending field, e.g. `@your-org/create: createConfig.templates[2].template must be a non-empty string`.

### Bundled subdirectory templates

Relative `./...` paths resolve against the enclosing `@org/create` package root — **not** the user's cwd. The referenced directory is copied verbatim into the target project (no template-engine processing). Paths that escape the package root are rejected.

### Make the org the default in a repo

Commit this in `vite.config.ts` at the project root:

```ts
import { defineConfig } from 'vite-plus';

export default defineConfig({
  create: { defaultTemplate: '@your-org' },
});
```

Now `vp create` (with no argument) drops straight into the `@your-org` picker. See [`create.defaultTemplate`](/config/create) for details.

The picker always appends a trailing **Vite+ built-in templates** entry so `vite:monorepo` / `vite:application` / `vite:library` / `vite:generator` stay reachable from the picker — selecting it routes to the standard built-in flow. For scripts and CI, explicit specifiers (`vp create vite:library`) bypass the configured default.

### Non-interactive inspection

`vp create @org --no-interactive` prints the manifest as a table and exits 1:

```
A template name is required when running `vp create @your-org` in non-interactive mode.

Available templates in @your-org/create:

  NAME     DESCRIPTION                          TEMPLATE
  web      Web app template (Vite + React)      @your-org/template-web
  library  TypeScript library template          @your-org/template-library
  demo     Bundled demo template                ./templates/demo

Examples:
  # Scaffold a specific template from the org
  vp create @your-org:web --no-interactive

  # Or use a Vite+ built-in template
  vp create vite:application --no-interactive
```

### Publishing checklist

1. Create `@org/create` (scoped npm package) if you don't already have one.
2. Add a `createConfig.templates` array to `package.json`. (Bundle the templates under `./templates/...` or point at external packages.)
3. (Optional) Provide a `bin` launcher for `npm create @org` compatibility.
4. Publish.
5. Verify: `vp create @org --no-interactive` prints the manifest table; `vp create @org` opens the picker.
6. (Optional) Commit `create: { defaultTemplate: '@org' }` in your internal template repos.

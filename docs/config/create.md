# Create Config

`vp create` reads the `create` block in `vite.config.ts` to set per-repo defaults. See the [Creating a Project guide](/guide/create#organization-templates) for the full `@org` template workflow.

## `create.defaultTemplate`

When `vp create` is invoked with no `TEMPLATE` argument, Vite+ uses this value as if the user had typed it. Typically set to an npm scope whose `@scope/create` package publishes a `createConfig.templates` manifest — so bare `vp create` drops into the org picker.

```ts
import { defineConfig } from 'vite-plus';

export default defineConfig({
  create: {
    defaultTemplate: '@your-org',
  },
});
```

Any value accepted by `vp create` as a first argument works here: `@your-org` for an org picker, `@your-org:web` for a direct manifest entry, `vite:application` for a built-in, or the `name` of a local `create.templates` entry (see below).

## `create.templates`

Declare local templates available to `vp create` inside a monorepo. Each entry is listed in the `vp create` picker, and selecting it (or passing its `name` as the template argument) runs the resolved `template`.

```ts
import { defineConfig } from 'vite-plus';

export default defineConfig({
  create: {
    templates: [
      {
        name: 'component',
        description: 'Internal UI component',
        template: './tools/create-component',
      },
      { name: 'service', description: 'Backend service', template: 'service-generator' },
    ],
  },
});
```

Each entry has:

| Field         | Required | Notes                                                                                                                                                                                                                                            |
| ------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `name`        | yes      | Identifier shown in the picker and accepted as `vp create <name>`. Must be unique within the array.                                                                                                                                              |
| `description` | yes      | One-line description shown in the picker.                                                                                                                                                                                                        |
| `template`    | yes      | A workspace package name, a relative `./path` to a local package's directory (resolved against the workspace root), a `vite:*` built-in, a GitHub URL, or a full npm package name (e.g. `create-foo`). It is run as-is (not shorthand-expanded). |

`create.templates` is the source of truth for local templates: only entries listed here appear in the picker. Vite+ does not infer templates from package.json keywords. A `create.templates` entry whose `template` resolves to a local package without a `bin` is reported as an error rather than falling through to an unrelated npm package.

[`vp create vite:generator`](/guide/create#code-generators) adds an entry here automatically (idempotently, preserving `defaultTemplate`); you can also edit the list by hand.

`create.defaultTemplate` can name a local entry, so bare `vp create` opens it directly.

## Precedence

CLI argument > `create.defaultTemplate` > the standard built-in picker.

Explicit specifiers always win, so scripts and CI can bypass the configured default:

```bash
# Uses create.defaultTemplate
vp create

# Explicitly ignores the default
vp create vite:library
```

The org picker also appends a trailing "Vite+ built-in templates" entry — selecting it routes to the `vite:monorepo` / `vite:application` / `vite:library` / `vite:generator` flow, so built-ins stay reachable interactively even when a default is configured.

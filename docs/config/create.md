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

Any value accepted by `vp create` as a first argument works here — `@your-org` for an org picker, `@your-org:web` for a direct manifest entry, `vite:application` for a built-in, etc.

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

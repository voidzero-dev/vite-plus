# Configuring Vite+

Vite+ keeps project configuration in one place: `vite.config.ts`, allowing you to consolidate many top-level configuration files in a single file. You can keep using your Vite configuration such as `server` or `build`, and add Vite+ blocks for the rest of your workflow:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  server: {},
  build: {},
  preview: {},

  create: {},
  run: {},
  fmt: {},
  lint: {},
  check: {},
  test: {},
  pack: {},
  staged: {},
});
```

## Vite+ Specific Configuration

Vite+ extends the basic Vite configuration with these additions:

- [`create`](/config/create) for project and template scaffolding defaults
- [`run`](/config/run) for Vite Task
- [`fmt`](/config/fmt) for Oxfmt
- [`lint`](/config/lint) for Oxlint
- [`check`](/config/check) for `vp check` defaults
- [`test`](/config/test) for Vitest
- [`pack`](/config/pack) for tsdown
- [`staged`](/config/staged) for staged-file checks
- [`defaultPackage`](#defaultpackage) for the default target of bare app commands at a workspace root

## defaultPackage

Default target directory for `vp dev` / `vp build` / `vp preview` / `vp pack` when they are invoked bare in the directory containing the config, an implicit [`vp -C <dir>`](/guide/monorepo#app-commands):

```ts [vite.config.ts]
export default {
  defaultPackage: './frontend',
};
```

The value must be a static string literal: vp reads it without executing the config, so it also works at repository roots without a vite-plus install (for example a Laravel or Rails repo whose Vite app lives in `frontend/`). An explicit `-C` or positional target always wins over the config.

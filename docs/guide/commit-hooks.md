# Commit Hooks

Use `vp config` to install commit hooks, and `vp staged` to run checks on staged files.

## Overview

Vite+ supports commit hooks and staged-file checks without additional tooling.

Use:

- `vp config` to set up project hooks and related integrations
- `vp staged` to run checks against the files currently staged in Git

If you use [`vp create`](/guide/create) or [`vp migrate`](/guide/migrate), Vite+ prompts you to set this up for your project automatically.

## Commands

### `vp config`

`vp config` configures Vite+ for the current project. It installs Git hooks, sets up the hook directory, and can also handle related project integration such as agent setup. By default, hooks are written to `.vite-hooks`:

```bash
vp config
vp config --hooks-dir .vite-hooks
```

### `vp staged`

`vp staged` runs staged-file checks using the `staged` config from `vite.config.ts`. If you set up Vite+ to handle your commit hooks, it will automatically run when you commit your local changes.

```bash
vp staged
vp staged --verbose
vp staged --fail-on-changes
```

## Configuration

Define staged-file checks in the `staged` block in `vite.config.ts`:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    '*.{js,ts,tsx,vue,svelte}': 'vp check --fix',
  },
});
```

This is the default Vite+ approach and should replace separate `lint-staged` configuration in most projects. Because `vp staged` reads from `vite.config.ts`, your staged-file checks stay in the same place as your lint, format, test, build, and task-runner config.

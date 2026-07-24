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
vp config --no-hooks
vp config --no-agent
```

Use `--no-hooks` when you want `vp config` to leave existing Git hook setup unchanged. Use
`--no-agent` when you want it to skip updates to existing coding agent instruction files. You
can pass both flags when you want `vp config` to skip both setup steps.

You can also set `VITE_GIT_HOOKS=0` to disable hook installation from lifecycle scripts such as
`prepare` or `postinstall`.

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

## Removing commit hooks

To fully remove Vite+ commit hooks, undo each thing `vp config` set up:

1. Unset the Git hooks path that points at the Vite+ dispatcher:

```bash
git config --unset core.hooksPath
```

2. Remove the hooks directory (use your `--hooks-dir` value if you changed it):

```bash
rm -rf .vite-hooks
```

3. Remove `vp config` from the `prepare` script in `package.json`. Otherwise the next install re-runs `vp config` and reinstalls the hooks.

4. Remove the `staged` block from `vite.config.ts` if it exists

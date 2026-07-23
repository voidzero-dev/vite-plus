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

## Disabling Hooks in Specific Environments

The installed hooks check the environment on every run, so you can disable them per machine or per process without uninstalling anything. This is useful when commits happen outside development — for example on a production server where a flat-file CMS commits content changes.

### Environment variable

Set `VITE_GIT_HOOKS=0` in the environment of the process that runs `git commit`, and every Vite+ hook exits immediately without running:

```bash
VITE_GIT_HOOKS=0 git commit -m "content update"
```

`HUSKY=0` is honored the same way for compatibility. Setting `VITE_GIT_HOOKS=0` in an environment also keeps `vp config` from reinstalling hooks there when a lifecycle script such as `prepare` runs.

### Init script

Before checking the environment variable, each hook sources an init script if one exists:

1. `$XDG_CONFIG_HOME/vite-plus/hooks-init.sh` (defaults to `~/.config/vite-plus/hooks-init.sh`)
2. `$XDG_CONFIG_HOME/husky/init.sh` as a fallback

To disable hooks for a whole machine, create the init script and export the variable there:

```sh [~/.config/vite-plus/hooks-init.sh]
export VITE_GIT_HOOKS=0
```

Because the hook itself reads this file, it works even when the committing process does not inherit your shell environment — a daemon or web server making commits is still covered.

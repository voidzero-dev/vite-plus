# Commit Hooks

Use `vp config` to install the Git hook dispatcher, and `vp staged` to run checks on staged
files.

## Overview

Vite+ supports commit hooks and staged-file checks without additional tooling.

Use:

- `vp config` to install generated hook infrastructure and related integrations
- `vp staged` to run checks against the files currently staged in Git

If you use [`vp create`](/guide/create) or [`vp migrate`](/guide/migrate), Vite+ prompts you to set this up for your project automatically.

## Commands

### `vp config`

`vp config` configures Vite+ for the current project. It installs the generated Git hook
dispatcher and can also handle related project integration such as agent setup. By default,
project hooks are read from `.vite-hooks`:

```bash
vp config
vp config --hooks-dir .vite-hooks
vp config --no-hooks
vp config --no-agent
```

Use `--no-hooks` when you want `vp config` to leave the Git hook dispatcher unchanged. Use
`--no-agent` when you want it to skip updates to existing coding agent instruction files. You can
pass both flags when you want `vp config` to skip both setup steps.

You can also set `VP_GIT_HOOKS=0` to disable hook installation from lifecycle scripts such as
`prepare` or `postinstall`.

Project-owned hook scripts such as `.vite-hooks/pre-commit` should be committed to the repository.
The generated dispatcher and shims under `.vite-hooks/_` are ignored and recreated by `vp config`.
`vp config` does not create or modify project hook scripts or staged-file configuration.

### `vp staged`

`vp staged` runs staged-file checks using the `staged` config from `vite.config.ts`. To run it
before each commit, add it to the project-owned pre-commit hook:

```bash
vp staged
vp staged --verbose
vp staged --fail-on-changes
```

```sh [.vite-hooks/pre-commit]
vp staged
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

This is the default Vite+ approach and should replace separate `lint-staged` configuration in most
projects. When you opt into hooks during `vp create`, Vite+ scaffolds both this configuration and
the corresponding pre-commit hook. During `vp migrate`, existing hook policy is preserved, and
the default is introduced only when no existing hook policy is found. Because
`vp staged` reads from `vite.config.ts`, your staged-file checks stay in the same place as your
lint, format, test, build, and task-runner config.

## Disabling Hooks in Specific Environments

The installed hooks check the environment on every run, so you can disable them per machine or per process without uninstalling anything. This is useful when commits happen outside development, for example through a flat file CMS or other processes.

### Environment variable

Set `VP_GIT_HOOKS=0` in the environment of the process that runs `git commit`, and every Vite+ hook exits immediately without running:

```bash
VP_GIT_HOOKS=0 git commit -m "content update"
```

`HUSKY=0` is honored the same way for ecosystem tooling compatibility. Setting `VP_GIT_HOOKS=0` in an environment also keeps `vp config` from reinstalling hooks there when a lifecycle script such as `prepare` runs.

### Init script

Before checking the environment variable, each hook sources an init script if one exists:

1. `$XDG_CONFIG_HOME/vite-plus/hooks-init.sh` (defaults to `~/.config/vite-plus/hooks-init.sh`)
2. `$XDG_CONFIG_HOME/husky/init.sh` as a fallback

To disable hooks for a whole machine, create the init script and export the variable there:

```sh [~/.config/vite-plus/hooks-init.sh]
export VP_GIT_HOOKS=0
```

Because the hook itself reads this file, it works even when the committing process does not inherit your shell environment, for example if a daemon or web server is making commits.

## Removing commit hooks

To stop using the Vite+ hook dispatcher:

1. Remove `vp config` from the `prepare` or `postinstall` script in `package.json`.

2. Unset the Git hooks path that points at the Vite+ dispatcher:

```bash
git config --unset core.hooksPath
```

3. Remove the generated dispatcher directory (use your `--hooks-dir` value if you changed it):

```bash
rm -rf .vite-hooks/_
```

Project-owned scripts such as `.vite-hooks/pre-commit` and the `staged` block in `vite.config.ts`
can remain for later use, or you can remove them separately if the project no longer needs them.

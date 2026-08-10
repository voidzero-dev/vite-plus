# Commit Hooks

Use `vp hooks` to manage the Git hook dispatcher, `vp config` for project setup
(hooks + agent integration), and `vp staged` to run checks on staged files.

## Overview

Vite+ supports commit hooks and staged-file checks without additional tooling.

Use:

- `vp hooks enable` / `disable` / `status` to manage the generated hook dispatcher
- `vp config` to install the dispatcher (when not disabled) and update agent integration
- `vp staged` to run checks against the files currently staged in Git

If you use [`vp create`](/guide/create) or [`vp migrate`](/guide/migrate), Vite+ prompts you to set this up for your project automatically.

### Quick start

```bash
# Install or refresh the dispatcher
vp hooks enable

# Check what is active in this clone
vp hooks status

# Turn hooks off in this clone (survives npm install / prepare)
vp hooks disable

# Turn them back on
vp hooks enable
```

## Commands

### `vp hooks`

Manage the Vite+ Git hook dispatcher for the current repository:

```bash
vp hooks enable
vp hooks enable --hooks-dir .custom-hooks
vp hooks disable
vp hooks status
```

| Command   | Behavior                                                                                                                                                                                                           |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `enable`  | Install or refresh the generated dispatcher under `<hooks-dir>/_` and set `core.hooksPath`. Clears a previous disable preference.                                                                                  |
| `disable` | Tear down the dispatcher (unset `core.hooksPath` when it points at Vite+, remove `<hooks-dir>/_`) and **persist** the disable decision in local git config so `vp config` / lifecycle scripts do not reinstall it. |
| `status`  | Show preference, `core.hooksPath`, dispatcher presence, and project-owned hook scripts.                                                                                                                            |

By default, project hooks live in `.vite-hooks`. Pass `--hooks-dir` to use another subdirectory. After the first successful enable, the directory is remembered in local git config for later `enable` / `disable` / `status` / `vp config` calls in this clone.

`status` reports preference as:

- `not set` — no disable preference and no prior enable in this clone
- `enabled` — enable has run (or the dispatcher is currently owned)
- `disabled (local)` — after `vp hooks disable`

Check the `Dispatcher` and `core.hooksPath` lines to see whether hooks are actually active.

`disable` / `enable` do **not** delete project-owned hook scripts (for example `.vite-hooks/pre-commit`), the `staged` block in `vite.config.ts`, or lifecycle scripts that call `vp config`.

### `vp config`

`vp config` configures Vite+ for the current project. It installs the generated Git hook
dispatcher (unless hooks were disabled with `vp hooks disable`) and can also handle related
project integration such as agent setup. The hooks directory defaults to `.vite-hooks`, or the
last directory used by `vp hooks` / `vp config` in this clone:

```bash
vp config
vp config --hooks-dir .vite-hooks
vp config --no-hooks
vp config --no-agent
```

Use `--no-hooks` when you want `vp config` to leave the Git hook dispatcher unchanged. Use
`--no-agent` when you want it to skip updates to existing coding agent instruction files. You can
pass both flags when you want `vp config` to skip both setup steps. After `vp hooks disable`,
`vp config` skips reinstalling the dispatcher and points you at `vp hooks enable` instead of
prompting again.

You can also set `VP_GIT_HOOKS=0` to disable hook installation from lifecycle scripts such as
`prepare` or `postinstall`.

Project-owned hook scripts such as `.vite-hooks/pre-commit` should be committed to the repository.
The generated dispatcher and shims under `.vite-hooks/_` are ignored and recreated by `vp config`
or `vp hooks enable`. Neither command creates or modifies project hook scripts or staged-file
configuration.

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

`HUSKY=0` is honored the same way for ecosystem tooling compatibility. Setting `VP_GIT_HOOKS=0` in an environment also keeps `vp config` / `vp hooks enable` from reinstalling hooks there when a lifecycle script such as `prepare` runs.

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

To stop using the Vite+ hook dispatcher in this clone (and keep `prepare` / `vp config` from
reinstalling it):

```bash
vp hooks disable
# or, if you used a custom directory:
vp hooks disable --hooks-dir .custom-hooks
```

This:

1. Unsets `core.hooksPath` when it points at the Vite+ dispatcher
2. Removes the generated `<hooks-dir>/_` directory
3. Records a **local** disable preference so lifecycle scripts skip reinstall until you run
   `vp hooks enable` again

To re-enable:

```bash
vp hooks enable
```

If you no longer want hooks for the project at all (shared with teammates), also remove `vp config`
from the `prepare` or `postinstall` script in `package.json`.

### Manual equivalent

If you prefer to do it by hand:

```bash
git config --unset core.hooksPath
rm -rf .vite-hooks/_
# optional: prevent prepare/vp config from reinstalling in this clone
git config --local vp.hooks.disabled true
# optional: remembered hooks directory (set by enable/disable)
# git config --local vp.hooks.dir .vite-hooks
```

Project-owned scripts such as `.vite-hooks/pre-commit` and the `staged` block in `vite.config.ts`
can remain for later use, or you can remove them separately if the project no longer needs them.
`vp hooks disable` does **not** delete those project-owned files.

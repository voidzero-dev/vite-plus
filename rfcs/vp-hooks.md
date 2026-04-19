# RFC: `vp hooks`

## Summary

Add `vp hooks`, a secure hook system for selected built-in `vp` commands.

Projects can attach local tasks or package.json scripts to built-in commands such as `vp check`, `vp fmt`, `vp lint`, `vp dev`, `vp build`, and a future `vp release`, without replacing the built-in implementation itself.

Unlike the earlier centralized `commandHooks` idea, hooks are configured next to the command they extend:

```ts
import { defineConfig } from 'vite-plus';

export default defineConfig({
  check: {
    hooks: {
      pre: ['codegen'],
    },
  },

  fmt: {
    singleQuote: true,
    hooks: {
      pre: ['generated:verify'],
    },
  },

  dev: {
    hooks: {
      pre: ['dev:setup'],
      finally: ['dev:teardown'],
    },
  },

  build: {
    sourcemap: true,
    hooks: {
      pre: ['generated:verify'],
    },
  },
});
```

Hooks execute through `vp run`, and are intentionally narrower than npm lifecycle scripts:

- no install-time hooks
- no inline shell snippets in hook config
- no hooks for package-manager commands
- explicit trust for automatic execution

This gives real projects npm-like `pre` / `post` extensibility while keeping the feature local, auditable, and much safer than `preinstall` / `postinstall`.

## Motivation

Built-in `vp` commands are intentionally opinionated:

- `vp check` runs the default fast-check pipeline
- `vp fmt` runs the built-in formatter
- `vp lint` runs the built-in linter
- `vp dev` starts the built-in dev server
- `vp build` runs the built-in production build

That default is valuable, but real projects often need a small amount of orchestration around those commands:

- run code generation before `vp check`
- validate generated files before `vp fmt`
- start or verify local services before `vp dev`
- verify release assets before `vp build`
- publish metadata or notifications around a future `vp release`

Today, users have three imperfect options:

1. Stop using the built-in command and wrap everything in a custom script
2. Tell teammates to remember extra commands before or after `vp check` / `vp dev`
3. Overload `package.json` scripts and ask users to run `vp run <name>` instead of the built-in command

Pain points:

- users lose the canonical built-in UX, docs, and help output
- teams end up with different entrypoints for the same workflow
- built-in commands cannot be safely extended in a first-class way
- npm-style lifecycle scripts are too broad and too risky for this purpose

Vite+ already supports `preX` / `postX` behavior for `vp run` via `run.enablePrePostScripts`. The missing piece is equivalent extensibility for built-in commands themselves.

Placing hooks on the command's own config block is also more discoverable than a centralized map:

- `fmt` concerns stay in `fmt`
- `check` concerns stay in `check`
- `dev` concerns stay in `dev`
- `build` concerns stay in `build`

That locality fits how Vite+ already treats `lint`, `fmt`, `test`, `run`, and `staged` config. In particular, `staged` is already a precedent for a Vite+-owned workflow block in `vite.config.ts`, and it points toward a possible future `git.*` namespace for Git-triggered workflows.

## Goals

1. Let projects extend selected built-in `vp` commands without replacing them
2. Keep hook configuration close to the command it affects
3. Reuse existing `vp run` task and script resolution instead of inventing a second command runner
4. Keep hook configuration declarative and easy to audit
5. Make hook execution opt-in and transparent
6. Avoid recreating npm's install-time attack surface

## Non-goals

1. Supporting `preinstall`, `install`, `postinstall`, or any other dependency-install lifecycle hook
2. Running arbitrary inline shell commands directly from the hook config
3. Hooking package-manager commands such as `vp install`, `vp add`, `vp remove`, or `vp update`
4. Hooking `vp config`, `vp staged`, `vp create`, or `vp migrate`
5. Replacing a built-in command entirely in v1

## Proposed Design

### Command-Local Hook Config

Hooks are configured on the command-specific config block, not under `run.commandHooks`.

Initial shape:

```ts
type HookTarget = string;

type CommandHooks = {
  pre?: HookTarget[];
  post?: HookTarget[];
  finally?: HookTarget[];
};

type CheckConfig = {
  hooks?: CommandHooks;
};

type DevConfig = {
  hooks?: CommandHooks;
};

type ReleaseConfig = {
  hooks?: CommandHooks;
};

type BuildConfig = ViteBuildOptions & {
  hooks?: CommandHooks;
};

type FmtConfig = OxfmtConfig & {
  hooks?: CommandHooks;
};

type LintConfig = OxlintConfig & {
  hooks?: CommandHooks;
};
```

Example:

```ts
import { defineConfig } from 'vite-plus';

export default defineConfig({
  check: {
    hooks: {
      pre: ['codegen'],
    },
  },

  fmt: {
    ignorePatterns: ['dist/**'],
    singleQuote: true,
    hooks: {
      pre: ['generated:verify'],
      post: ['generated:write-manifest'],
    },
  },

  dev: {
    hooks: {
      pre: ['dev:setup'],
      finally: ['dev:teardown'],
    },
  },

  build: {
    sourcemap: true,
    hooks: {
      pre: ['generated:verify'],
    },
  },
});
```

Each hook entry contains names that are resolved exactly like `vp run <name>`:

- a Vite Task entry from `run.tasks`
- a `package.json` script

This is the central security decision of the RFC: the hook config does not contain raw shell commands. It only references named local workflows that the project already exposes through `vp run`.

### Supported Commands

Initial allowlist:

- `check`
- `fmt`
- `lint`
- `dev`
- `build`
- `release` once that command exists

Commands intentionally excluded in v1:

- `install`, `add`, `remove`, `update`, `dedupe`, `pm`
- `config`, `staged`
- `create`, `migrate`
- `run`, because it already has `preX` / `postX`
- `exec` and `dlx`
- `preview`

`preview` is excluded in v1 because it is less central than `build`, and the initial demand is much clearer for build pipelines.

`build` is included even though the top-level `build` key is owned by Vite core. To keep that boundary explicit, Vite+ must strip `build.hooks` out before forwarding the remaining `build` options to Vite itself.

### Why This Shape

This structure matches how users already think about Vite+ config:

- formatter behavior lives under `fmt`
- linter behavior lives under `lint`
- build behavior lives under `build`
- staged-file behavior lives under `staged`

Adding `hooks` next to each command's existing options keeps the config easier to read than a separate global registry.

It also makes reviews clearer:

- changing `fmt.hooks` clearly means "the formatter workflow changed"
- changing `check.hooks` clearly means "the check workflow changed"

### Relationship to `staged` and a Future `git` Namespace

`staged` is useful precedent here.

Vite+ already asks users to configure staged-file workflows inside `vite.config.ts`:

```ts
export default defineConfig({
  staged: {
    '*.{js,ts,tsx}': 'vp check --fix',
  },
});
```

That means Vite+ already has the idea of "workflow config belongs in a named top-level block" rather than only in `package.json` scripts or third-party config files.

This RFC follows the same direction:

- `staged` configures the staged-file workflow
- `check.hooks` configures the `vp check` workflow
- `fmt.hooks` configures the `vp fmt` workflow
- `dev.hooks` configures the `vp dev` workflow

Longer term, Git-triggered workflows may want a clearer namespace such as:

```ts
export default defineConfig({
  git: {
    staging: {
      rules: {
        '*.{js,ts,tsx}': 'vp check --fix',
      },
    },
    commit: {
      // future git-commit workflow config
    },
    push: {
      // future git-push workflow config
    },
  },
});
```

That would create a consistent family:

- `git.staging`
- `git.commit`
- `git.push`

This RFC does not propose that rename directly. `staged` can remain the current public shape for now. But the design should avoid blocking that future normalization if the team decides the Git workflow surface should be unified.

## Execution Model

When a hookable built-in command runs:

1. Resolve the workspace root and the command's `hooks` config
2. Check whether hooks are enabled for this project
3. Run `pre` hooks sequentially through `vp run`
4. Run the built-in command
5. If the built-in command succeeds, run `post` hooks sequentially through `vp run`
6. Always run `finally` hooks sequentially through `vp run`

Behavior:

- if any `pre` hook fails, the built-in command is not executed
- if the built-in command fails, `post` is skipped
- `finally` always runs after `pre` or the main command has started
- any failing hook makes the overall command exit non-zero
- hooks inherit the current working directory and workspace context

## Hook Context

Hooks receive a small set of environment variables:

- `VP_HOOK_COMMAND` - canonical built-in command name such as `check`
- `VP_HOOK_STAGE` - `pre`, `post`, or `finally`
- `VP_HOOK_ARGS_JSON` - JSON array of the original CLI args
- `VP_HOOK_ROOT` - workspace root path
- `VP_HOOK_MAIN_EXIT_CODE` - available for `post` and `finally`

This lets a shared task behave differently when called from `vp check` versus `vp dev`, without making the hook config itself imperative.

## Re-entrancy Guard

Hooks must not recursively trigger more hooks forever.

When Vite+ invokes a hook target, it sets an internal guard environment variable. Any built-in `vp` command launched from that hook target runs with command hooks disabled by default for that nested invocation.

This keeps these cases safe and predictable:

- a hook target that internally calls `vp check`
- a shared `package.json` script reused in multiple places
- cleanup hooks that call other Vite+ commands

## `vp hooks` Command

Add a new management command:

```bash
vp hooks list
vp hooks trust
vp hooks untrust
```

### `vp hooks list`

Prints the effective hook config found across the supported command blocks and whether it is trusted on this machine.

Example:

```text
Configured command hooks for /path/to/repo

check
  pre: codegen

fmt
  pre: generated:verify

dev
  pre: dev:setup
  finally: dev:teardown

build
  pre: generated:verify

Trust: not granted
```

### `vp hooks trust`

Marks the current project as trusted for automatic command-hook execution.

Trust is stored per workspace root plus a hash of the normalized effective hook config. If any command's hook config changes later, trust is invalidated and the user is asked again.

### `vp hooks untrust`

Removes the local trust record for the current project.

## Trust and UX

Command hooks are more powerful than today's built-in commands because they can indirectly run arbitrary project tasks. They should therefore be opt-in.

### Interactive Terminals

When a trusted record does not exist and the user runs a hookable command with configured hooks:

1. Vite+ prints a short summary of the configured hooks for that command
2. Vite+ asks whether command hooks should be trusted for this project
3. If accepted, trust is recorded and hooks run
4. If declined, the built-in command runs without hooks

### Non-interactive / CI

In non-interactive environments, untrusted command hooks are skipped by default.

Users can opt in per invocation or per environment with:

```bash
VP_COMMAND_HOOKS=1 vp check
```

Users can disable hooks explicitly with:

```bash
VP_COMMAND_HOOKS=0 vp check
```

This makes automation safe by default and explicit when teams want hooked behavior in CI.

## Example Workflows

### 1. Codegen before `vp check`

```ts
export default defineConfig({
  check: {
    hooks: {
      pre: ['codegen'],
    },
  },
  run: {
    tasks: {
      codegen: {
        command: 'node scripts/codegen.mjs',
      },
    },
  },
});
```

Now `vp check` means:

```text
vp run codegen
vp check
```

without forcing users to remember a different command.

### 2. Verification around `vp fmt`

```ts
export default defineConfig({
  fmt: {
    singleQuote: true,
    hooks: {
      pre: ['generated:verify'],
    },
  },
  run: {
    tasks: {
      'generated:verify': {
        command: 'node scripts/verify-generated.mjs',
      },
    },
  },
});
```

This keeps all formatter concerns in the `fmt` block instead of splitting them between tool config and a separate hook registry.

### 3. Service bootstrap around `vp dev`

```ts
export default defineConfig({
  dev: {
    hooks: {
      pre: ['dev:setup'],
      finally: ['dev:teardown'],
    },
  },
  run: {
    tasks: {
      'dev:setup': {
        command: 'docker compose up -d postgres',
        cache: false,
      },
      'dev:teardown': {
        command: 'docker compose stop postgres',
        cache: false,
      },
    },
  },
});
```

This preserves `vp dev` as the entrypoint while still allowing project-specific local setup and cleanup.

### 4. Release preparation for a future `vp release`

```ts
export default defineConfig({
  release: {
    hooks: {
      pre: ['release:prepare'],
      post: ['release:announce'],
    },
  },
  run: {
    tasks: {
      'release:prepare': {
        command: 'node scripts/prepare-release.mjs',
        cache: false,
      },
      'release:announce': {
        command: 'node scripts/post-release.mjs',
        cache: false,
      },
    },
  },
});
```

### 5. Verification before `vp build`

```ts
export default defineConfig({
  build: {
    sourcemap: true,
    hooks: {
      pre: ['assets:verify'],
      post: ['artifacts:manifest'],
    },
  },
  run: {
    tasks: {
      'assets:verify': {
        command: 'node scripts/verify-assets.mjs',
      },
      'artifacts:manifest': {
        command: 'node scripts/write-artifact-manifest.mjs',
        cache: false,
      },
    },
  },
});
```

This lets teams keep production-build concerns attached to the `build` block instead of inventing a wrapper script around `vp build`.

## Relationship to Existing Features

### `run.enablePrePostScripts`

`run.enablePrePostScripts` continues to apply to `vp run`.

Command hooks are different:

- `pretest` / `posttest` affect `vp run test`
- `check.hooks` affects built-in `vp check`
- `fmt.hooks` affects built-in `vp fmt`
- `build.hooks` affects built-in `vp build`

The two features intentionally compose rather than replace each other.

### Git Hooks

This RFC is not about Git hooks.

- Git hooks remain `vp config` + `vp staged`
- command hooks are runtime extensions for built-in `vp` commands

Using command-local `hooks` instead of a top-level `hooks` key avoids conflating the two concepts.

`staged` is also a good mental model for why these hooks belong in `vite.config.ts`: both features are project workflow extensions owned by Vite+, not ad-hoc shell wrappers.

If Vite+ later introduces a `git` namespace, that would still be complementary to this RFC:

- command-invoked workflows stay local to the command, such as `check.hooks`
- Git-invoked workflows could live under `git.staging`, `git.commit`, and `git.push`

### Built-in Command Ownership

Built-in commands still own their default behavior.

For example:

- `vp check` is still Vite+'s built-in fast check command
- a project cannot silently replace it with an unrelated script
- customization happens around the built-in command, not instead of it

This keeps docs, help output, and team expectations consistent.

## Security Considerations

This RFC is explicitly designed around the current supply-chain climate.

### 1. No install-time hooks

There is no equivalent of npm `preinstall` / `postinstall`.

Nothing in this system runs during:

- `vp install`
- dependency resolution
- package extraction
- `vp add` / `vp update`

That removes the most common and least auditable automatic-execution path.

### 2. No inline shell in config

Hook config only references names that are already valid `vp run` targets.

This improves auditability:

- reviewers see a named workflow such as `codegen`
- the actual command still lives in one place
- Vite+ does not need a new mini-shell language in config

### 3. Explicit allowlist

Only a small set of developer workflow commands are hookable. Package-management and lifecycle commands are not.

### 4. Trust-on-first-use

Automatic hook execution requires explicit trust. This reduces surprise when entering an unfamiliar repository.

### 5. Config-hash invalidation

Trust is tied to the normalized effective hook config, not just the repo path. If a pull request adds or changes hooks, the user is asked again.

### 6. Visible execution

When hooks run, Vite+ prints that they are running and which named target is being invoked. Hooks are not silent magic.

### 7. Escape hatch

`VP_COMMAND_HOOKS=0` disables command hooks immediately for debugging, incident response, or cautious inspection.

## Implementation Sketch

### Config Types

Extend Vite+'s `defineConfig()` typing with:

- `check?: CheckConfig`
- `dev?: DevConfig`
- `release?: ReleaseConfig`
- `build?: ViteBuildOptions & { hooks?: CommandHooks }`
- `fmt?: OxfmtConfig & { hooks?: CommandHooks }`
- `lint?: OxlintConfig & { hooks?: CommandHooks }`

For `build`, Vite+ strips `hooks` before passing the remaining object to Vite core. This keeps the hook surface local to the command without leaking Vite+-specific metadata into Vite's runtime config.

### Rust Global CLI

Add `Hook` to the command enum in `crates/vite_global_cli/src/cli.rs` and delegate it to the JavaScript side, similar to `config` or `create`.

### JavaScript Side

Add a small hook runtime in `packages/cli/src/hook/`:

- `config.ts` - load and normalize hook config from `check`, `fmt`, `lint`, `dev`, and `release`
- `config.ts` - load and normalize hook config from `check`, `fmt`, `lint`, `dev`, `build`, and `release`
- `trust.ts` - read and write local trust records
- `runner.ts` - execute hook stages around a built-in command
- `bin.ts` - implement `vp hooks list|trust|untrust`

`packages/cli/src/bin.ts` becomes the main integration point:

1. parse the requested built-in command
2. check whether that command supports hooks
3. load the command-local `hooks` config
4. wrap the normal command execution with the hook runner

No NAPI changes are required for the hook runtime itself. Hook orchestration can happen entirely in the TypeScript entrypoint before delegating to existing resolvers or bindings.

## Alternatives Considered

### 1. Centralized `run.commandHooks`

This was the first draft of the idea.

Rejected in favor of command-local hooks because:

- it separates a command from its own customization
- it is harder to scan during code review
- it creates a second place to look when debugging `vp fmt` or `vp check`

The command-local shape better matches the rest of `vite.config.ts`.

### 2. Wrapper scripts only

Users can already write:

```json
{
  "scripts": {
    "check": "node scripts/codegen.mjs && vp check"
  }
}
```

But this loses the canonical `vp check` entrypoint and recreates the exact confusion Vite+ tries to avoid.

### 3. Allow arbitrary shell commands in hook config

Example:

```ts
check: {
  hooks: {
    pre: ['node scripts/codegen.mjs'],
  },
}
```

Rejected because it is harder to review, harder to validate, and much closer to reintroducing npm-style lifecycle risk.

### 4. Hook every command, including install

Rejected for security reasons. The RFC is intentionally useful but narrow.

### 5. Full command replacement

Allowing a project to replace `vp check` with something unrelated makes built-ins impossible to reason about. Pre / post / finally hooks are a better first step.

## Open Questions

1. Should `post` remain success-only, or should there also be an explicit `postFailure` stage?
2. Should `release` be accepted in config before the built-in command exists, or only once that RFC lands?
3. Should `vp hooks trust` store trust under the git root, the workspace root, or both for nested workspaces?
4. Should `preview` eventually gain hook support too, and if so, should it live in `preview.hooks` or a separate Vite+-owned namespace?
5. Should there be a `--no-hooks` global override flag in addition to `VP_COMMAND_HOOKS=0`?
6. Should Git-triggered workflows eventually be unified under `git: { staging, commit, push }`, with today's `staged` shape preserved as a compatibility alias or migration path?

## Conclusion

`vp hooks` gives Vite+ a controlled way to extend built-in commands for real-world projects, while keeping the configuration exactly where users expect it.

The proposal is intentionally conservative:

- hooks live on the command they affect
- hooks reuse `vp run` instead of inventing a new execution model
- hooks never run during install
- automatic execution requires explicit trust

That balance makes the feature powerful enough for `check`, `fmt`, `lint`, `dev`, `build`, and future `release` workflows, while still respecting the security lessons that npm lifecycle scripts have taught the ecosystem.

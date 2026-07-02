# RFC: App Commands at the Monorepo Root (`vp dev` / `vp build` / `vp preview` / `vp pack`)

## Summary

Make the built-in app commands useful and predictable in monorepos with three changes:

1. **Path equivalence**: `vp dev <path>` behaves exactly like `cd <path> && vp dev`, by spawning the underlying tool with its working directory set to `<path>` instead of only forwarding the positional as Vite's `root`. This also fixes `vp pack <path>`, which today misinterprets a directory path as an entry glob.
2. **Interactive package picker at the workspace root**: running `vp dev` / `vp build` / `vp preview` / `vp pack` at a monorepo root in an interactive terminal opens a fuzzy-searchable package selector (reusing the `vite_select` component that powers the `vp run` task picker). Selecting a package runs the command there and prints a hint teaching the direct form (`vp dev apps/web`).
3. **`defaultPackage` config**: a root `vite.config.ts` can set a default target directory so these commands skip the picker and run in a known package. This also covers framework monorepos that are not JS workspaces (a Laravel, Rails, or Go repo with a `frontend/` directory), where the picker has no package list to enumerate.

The commands stay singular: `vp dev` still starts exactly one Vite dev server. The picker only elicits the one argument the command needs, in the one place where omitting it is ambiguous. Fan-out and task orchestration remain the job of `vp run`.

## Motivation

### Current Pain Points

**1. At a monorepo root, the app commands are silently wrong.**

The workspace root usually has no app, but `vp dev` happily starts a server pointed at it:

```
$ vp dev

  VITE v7.1.4  ready in 312 ms

  ➜  Local:   http://localhost:5173/        # opens to a 404, no index.html here
```

Nothing errors, nothing guides the user toward `vp dev apps/web` or `vp run`. The command is opinionated but broken instead of opinionated and helpful.

**2. `vp dev <path>` and `cd <path> && vp dev` are not equivalent.**

The positional is forwarded verbatim to Vite's `[root]`. Vite re-bases config lookup and `.env` loading on `root`, so those already come from `<path>` in both invocations. What differs is `process.cwd()` of the spawned Vite process:

- `process.cwd()` reads inside `vite.config.ts` and plugins resolve against the invocation directory instead of the project directory.
- Relative CLI arguments resolve against the invocation directory.
- The local `vite-plus` install is resolved from the invocation directory.

Users hit these differences as one-off bugs, and the only reliable workaround is `cd <path> && vp dev`, which undercuts `vp dev <path>` as the documented mechanism (`docs/guide/monorepo.md`, "App Commands").

**3. `vp pack <path>` does not work at all for directories.**

Pack's positional means "entry files/globs" (`packages/cli/src/pack-bin.ts`), and its config is always resolved from `process.cwd()`. So `vp pack packages/ui` bundles a directory glob against the root's config:

```
$ vp pack packages/ui
✗ Error: cannot resolve entry: packages/ui
```

Directory targeting exists only via `--root` / `-W` / `-F`, which is inconsistent with `vp dev <path>` / `vp build <path>`.

**4. Neither Vite's `root` option nor in-process `chdir` can close the gap.**

Upstream Vite resolves `root` without touching `process.cwd()`, by design, so forwarding the positional as `root` can never make the two forms equivalent. Calling `process.chdir()` inside the CLI process would close the gap but is a global mutation that leaks into everything sharing the process. The observation this RFC builds on: `vp` never runs Vite or tsdown in-process. The NAPI binding always spawns a fresh child process (`packages/cli/binding/src/cli/execution.rs`), so setting the child's spawn cwd achieves full equivalence with zero `process.chdir()` and no upstream Vite change.

## Proposed UX

Example workspace used throughout:

```
acme/
├── pnpm-workspace.yaml
├── vite.config.ts
├── apps/web          (Vite app)
├── apps/admin        (Vite app)
├── packages/ui       (library)
└── packages/utils    (library)
```

### 1. `vp dev` at the workspace root (interactive terminal)

Same look and keybindings as the `vp run` task selector, listing packages instead of tasks:

```
$ vp dev
Select a package to dev (↑/↓, Enter to run, type to search):

  › web         apps/web
    admin       apps/admin
    ui          packages/ui
    utils       packages/utils
```

Typing filters fuzzily, with the query shown inline:

```
Select a package to dev (↑/↓, Enter to run, type to search): adm

  › admin       apps/admin
```

Enter confirms, prints the teaching hint once, then hands off to Vite exactly as if the user had run it in that directory:

```
Selected package: admin (apps/admin)
Tip: run this directly with `vp dev apps/admin`

  VITE v7.1.4  ready in 324 ms

  ➜  Local:   http://localhost:5173/
  ➜  Network: use --host to expose
```

Escape clears the search, Ctrl+C cancels with exit code 130 and runs nothing (matching the task picker). `vp build`, `vp preview`, and `vp pack` at the root look the same, with `Select a package to build` / `preview` / `pack`.

### 2. The direct forms, now equivalent

These do the same thing after this RFC, byte for byte:

```bash
vp dev apps/admin
cd apps/admin && vp dev
# (picker selection of admin)
```

Flags pass through unchanged:

```
$ vp dev apps/admin --port 4000

  VITE v7.1.4  ready in 298 ms

  ➜  Local:   http://localhost:4000/
```

`vp pack packages/ui` now means "pack the ui package": config, entries, and `outDir` all resolve from `packages/ui`:

```
$ vp pack packages/ui

  dist/index.mjs      1.24 kB
  dist/index.d.mts    0.31 kB
  ✓ built in 187 ms
```

### 3. Non-interactive at the root (CI, piped output, scripts)

No picker can appear, so the command fails fast with the same information the picker would have shown:

```
$ vp build
✗ `vp build` at the workspace root needs a target package.

  Packages in this workspace:
    web         apps/web
    admin       apps/admin
    ui          packages/ui
    utils       packages/utils

  Pass a path:  vp build apps/web
  Or run every package's build script:  vp run -r build

$ echo $?
1
```

### 4. With `defaultPackage` configured

The motivating repo shape is a framework monorepo where the Vite app lives in a subdirectory of a repo that is not a JS workspace at all, for example a Laravel, Rails, or Go server with a `frontend/` directory:

```
shop/
├── app/               (PHP / Ruby / Go)
├── routes/
├── composer.json
├── vite.config.ts     (root config below)
└── frontend/          (the Vite app)
```

There is no `pnpm-workspace.yaml` or `workspaces` field here, so the picker has no package list to enumerate. `defaultPackage` is what makes `vp dev` at the root work in this shape:

```ts
// vite-plus is not installed at this root, and that is fine: vp reads this
// key via static extraction and never executes this file, so no import is
// needed. Vite never loads this config either (the app's real config lives
// in frontend/vite.config.ts); at this root it is purely a pointer for vp.
export default {
  defaultPackage: './frontend',
}
```

The app commands at the root skip the picker and go straight to the configured directory, with one line of output so it never feels magical:

```
$ vp dev
vp dev: using ./frontend (defaultPackage)

  VITE v7.1.4  ready in 301 ms

  ➜  Local:   http://localhost:5173/
```

An explicit path still wins over the config: `vp dev apps/admin` ignores `defaultPackage`.

The same key works in a JS workspace root, where it skips the picker for monorepos with one blessed app among many packages. There `vite-plus` is installed, so the usual `import { defineConfig } from 'vite-plus'` form applies and the key is type-checked.

### 5. Inside a sub-package: nothing changes

```
$ cd apps/web
$ vp dev

  VITE v7.1.4  ready in 289 ms

  ➜  Local:   http://localhost:5173/
```

No picker ever appears below the root. Interactive mode is root-only, unless explicitly requested with `--pick`.

## Command Syntax

```
vp dev     [dir] [vite options...]
vp build   [dir] [vite options...]
vp preview [dir] [vite options...]
vp pack    [dir | ...entries] [pack options...]
```

### The `[dir]` positional

- Only the first positional is considered. When it names an existing directory, `vp` resolves it against the invocation cwd, strips it from the forwarded args, and spawns the tool with it as the working directory. A first positional that looks like a path but does not exist errors with `directory not found`.
- For `dev`/`build`/`preview` this refines Vite's existing `[root]` positional: same position, same user-facing meaning, now with full `cd` semantics.
- For `pack`, an existing directory selects the project directory; file and glob positionals keep their current meaning as entries. A directory-shaped entry glob is still expressible as `dir/**`.

### Forwarded options

Every other argument is forwarded verbatim to the underlying tool. Relative option values (`--config`, `--outDir`, ...) resolve in the target directory, matching the `cd` form. Pack's existing directory-related flags (`--root`, `-W`/`--workspace`, `-F`/`--filter`) keep their tsdown semantics and are evaluated after the cwd change: `vp pack apps/web -F ui` equals `cd apps/web && vp pack -F ui`.

### `--pick`

The only flag this RFC adds, available on all four commands. `--pick` forces the interactive package picker even when `defaultPackage` is set or when running inside a sub-package, and errors in a non-interactive terminal. Combining `--pick` with a `[dir]` positional is an error.

It is long-form only, deliberately: these commands forward unrecognized flags to Vite and tsdown, so a short alias like `-i` (pnpm's `-i, --interactive` convention) risks colliding with the bundled tools' current or future option namespaces.

### No `--filter` for dev/build/preview

Package targeting stays path-based. `-F`/`--filter` is not added because `-F` already means tsdown workspace filtering on `vp pack` and pnpm-style multi-package selection on `vp run`/`vp exec`; a third meaning (pick one package by name) would overload it. Name-based targeting can be added later without a new flag by letting the `[dir]` positional accept package names.

## Behavior

### Target directory resolution

For `vp dev`, `vp build`, `vp preview`, and `vp pack`, the target directory is resolved in this order:

1. **`--pick` flag**: open the interactive picker regardless of location or config (TTY required; error otherwise).
2. **Explicit path positional** (an existing directory): run there.
3. **`defaultPackage`** from the root `vite.config.ts`, when invoked in the directory containing that config (a workspace root, or the root of a non-workspace repo): run there, print a one-line note.
4. **Interactive picker**, when invoked at the workspace root in an interactive TTY (and not CI): pick, print hint, run there.
5. **Non-interactive at the workspace root**: print the package list and the direct-form hint, exit 1.
6. **Anywhere else** (sub-package or non-workspace project): current behavior, run in the current directory.

"Workspace root" means the current directory's package is the workspace root package, as determined by `vite_workspace::find_workspace_root` (already called on every invocation in `packages/cli/binding/src/cli/mod.rs`).

### Path positional semantics

- For `dev` / `build` / `preview`: the first positional is already Vite's `[root]` directory, so there is no ambiguity. When it names an existing directory, `vp` resolves it, strips it from the forwarded args, and spawns the child with that directory as its working directory. When it names a missing path, `vp` errors with `directory not found` (Vite would fail anyway, just less clearly).
- For `pack`: an existing **directory** positional selects the project directory; **file or glob** positionals keep their current meaning as entries. Users who genuinely want a directory-shaped entry glob can pass `dir/**` or use `--root` with explicit entries.
- Only the first positional is treated as the project directory; remaining args are forwarded verbatim.

### Equivalence invariant

After this RFC the following holds and is documented:

```
vp <cmd> <path> [args...]  ===  cd <path> && vp <cmd> [args...]
```

for `cmd` in `dev`, `build`, `preview`, `pack`. Concretely: the child process spawn cwd is `<path>`, so config lookup, `.env` loading, `process.cwd()` reads in configs and plugins, and relative CLI args all behave as if the user had `cd`'d. The parent `vp` process never calls `process.chdir()`.

The local `vite-plus` CLI itself is still resolved from the invocation directory, before the target directory applies. The invariant therefore assumes a workspace uses a single Vite+ version, which is already the supported monorepo model; with one version installed, both forms resolve the same CLI and the equivalence holds end to end.

### Entry points and version assumption

- The feature lives in the local CLI's NAPI binding (`execute_direct_subcommand`), which every entry point executes: the global `vp` binary delegates to the nearest local `vite-plus` install from the invocation directory (or to its bundled copy when none exists), and the `vite-plus` package's own `vp` bin (`pnpm exec vp dev`, `package.json` scripts) reaches the binding directly. No global-CLI routing changes are needed.
- Typing `vp dev` at an arbitrary root is primarily a global-CLI experience; local-only setups usually reach these commands through per-package scripts that already pin the directory. A root-level script such as `"dev": "vp dev"` in the workspace root `package.json` flows through the same logic and gets the same picker and config behavior.
- Both the global CLI and any local install are assumed to be versions that ship this feature. Vite+ is pre-1.0, so no compatibility behavior or version negotiation with older CLIs is specified. In the non-workspace shape the root has no local install, so the global binary's bundled CLI executes end to end; under this assumption that is equivalent, and re-resolving a local CLI from the target directory is out of scope for v1.

### Picker contents

- One row per workspace package: package name plus relative path.
- All workspace packages are listed; nothing is filtered out. Ranking favors likely targets: packages that look runnable for the command (a `vite.config.*` or `index.html` for `dev`/`build`/`preview`, a pack config or library entry for `pack`) sort above the rest, then by path. In a large monorepo with many libraries, `vp dev` shows the apps at the top while every package stays reachable by search, so a wrong relevance guess costs ranking, not reachability.
- Fuzzy search over name and path via `vite_select::fuzzy_match`, paging identical to the task picker.
- If the workspace root itself looks runnable (it has a `vite.config.*` or `index.html`), it appears as a `(workspace root)` entry, mirroring the task picker's labeling. This keeps today's "run at root anyway" behavior one keystroke away.
- If exactly one likely-runnable package exists, the picker auto-selects it, printing only the `Selected package:` line and the tip.

### `defaultPackage` config

```ts
export default defineConfig({
  // Relative to the config file's directory. Used by vp dev/build/preview/pack
  // when invoked next to this config without an explicit path.
  defaultPackage: './frontend',
})
```

- Type: `string` (a single directory). A per-command map can be added later if real demand appears; v1 stays simple.
- Consulted when `vp` is invoked in the directory containing the root config: a workspace root, or the root of a non-workspace repo. It is deliberately not limited to JS workspaces, because the framework-monorepo shape (Laravel, Rails, a Go server with a `frontend/` directory) has no workspace metadata to enumerate, so neither the picker nor auto-select can serve it; `defaultPackage` is the only mechanism of the three that covers it. An explicit positional always wins.
- If the directory does not exist, error: `defaultPackage points to a missing directory: ./frontend`.
- Read via the existing static extraction path (`vite_static_config` + the NAPI config loader in `packages/cli/binding/src/cli/handler.rs`), same as `run` config. For non-workspace roots, static extraction is load-bearing rather than an optimization: `vite-plus` is typically not installed at that root, so the config file cannot be imported or executed there. The root config must therefore work without executing (a plain default-export object, no `vite-plus` import), and the value must be a static string literal. If the key cannot be statically extracted and no local install exists to execute the config, vp errors and names the construct that defeated extraction.

## Decisions

### Spawn cwd instead of `process.chdir()` or Vite `root`

In-process `chdir` is a global mutation: it leaks into everything sharing the process and makes behavior depend on when the mutation happens, so it is ruled out. `vp` is a launcher: the final Vite/tsdown process is always a fresh child (`vite_command::build_command`), so its spawn cwd is free to set and perfectly scoped. Forwarding only Vite's `root` (today's behavior for dev/build) is the source of the non-equivalence; `root` intentionally does not change cwd upstream, and upstream is right not to change that.

### Commands stay singular

`vp dev` never becomes an orchestrator. It starts one dev server in one directory. The picker is argument elicitation, not fan-out: every path through it teaches the direct non-interactive form, and multi-package work stays on `vp run` (`-r`, `--filter`).

### Root-only interactivity

Below the root, the current directory unambiguously identifies the project, so prompting would be noise; `--pick` remains the explicit opt-in from anywhere. At the root the command is ambiguous today and silently wrong; that is exactly where a prompt earns its keep. This mirrors how bare `vp run` behaves (interactive when a TTY, informative listing when not), with one difference: the app commands exit 1 in the non-interactive case, because starting a server or build against the wrong directory is worse than failing loudly.

### Command scope

Only single-target app commands get target elicitation, because only they are ambiguous at the root. Tree-scoped commands (`test`, `lint`, `fmt`, `check`) mean "the whole repo" at the root, which is their desired behavior. Workspace-state commands (`install`, `add`, `outdated`, ...) have the root as their natural home. Orchestrators (`run`, `exec`) already own their selection models and remain the fan-out tools. A future command joins this RFC's treatment exactly when its subject is one package directory.

## Implementation Architecture

All changes live in the Rust layers; no upstream Vite or tsdown changes are required.

### NAPI binding (local CLI)

- `packages/cli/binding/src/cli/resolver.rs`: for `Dev` / `Build` / `Preview` / `Pack`, detect a leading existing-directory positional and intercept `--pick`, strip both from the forwarded args, and carry the resolved target directory. All other args continue to pass through verbatim.
- `packages/cli/binding/src/cli/execution.rs`: spawn the child with cwd set to the target directory when one was resolved.
- `packages/cli/binding/src/cli/mod.rs` (`execute_direct_subcommand`): workspace-root detection is already available here; add the resolution order (positional, `defaultPackage`, picker, non-TTY error).
- Picker: reuse `vite_select` (fuzzy search, groups, paging) and `vite_workspace` package enumeration, both already dependencies of this path via the `vite_task` crates.
- `defaultPackage`: extend the `VitePlusConfigLoader` static extraction the same way `run` config is loaded.

### TypeScript side

- `packages/cli/src/define-config.ts`: add `defaultPackage?: string` to the top-level config type.
- `packages/cli/src/pack-bin.ts`: no change needed if the binding strips the directory positional before delegation; pack continues to resolve config from its (now correct) cwd.

### Global CLI

No routing changes. `crates/vite_global_cli` already delegates dev/build/preview/pack to the local CLI with the invocation cwd; the new logic runs in the binding.

### Docs

- `docs/guide/monorepo.md` "App Commands": document the equivalence invariant, the root picker, and `defaultPackage`.
- `docs/config/` page for the new top-level key.

## Compatibility

- `vp dev <path>` / `vp build <path>` change behavior in the cwd-dependent edge cases (cwd reads in configs and plugins, relative CLI args). The new behavior matches what `cd <path> && vp <cmd>` already does, which is the semantics users report expecting; the delta is exactly the set of cases currently reported as bugs. Local CLI resolution is unchanged (still from the invocation directory; see the equivalence invariant). Ship in a minor with a changelog note.
- `vp pack <path>` with a directory changes from an error (or nonsense entry glob) to packing that directory. File and glob entries are unaffected.
- Running the app commands at a workspace root changes from "silently serve or build the root" to picker / config / clear error. Monorepos that intentionally run an app at the root keep working when the root has a `vite.config.*` or `index.html` (it appears as a picker entry, and setting `defaultPackage: '.'` restores the old behavior unconditionally).
- Non-workspace projects and sub-package invocations are unchanged.

## Snap Tests

Non-interactive branches are covered by snap tests:

- `vp build <dir>` / `vp pack <dir>` directory positionals (none exist today).
- App commands at a workspace root without a TTY: package listing and exit code.
- `defaultPackage`: happy path and missing-directory error.
- `--pick` without a TTY, and `--pick` combined with a `[dir]` positional: both error with exit 1.
- Equivalence checks: `vp build <dir>` and `cd <dir> && vp build` produce the same output in a fixture whose config reads `process.cwd()`.

The interactive picker gets pty snapshot coverage in the `vite_task` repo style (`task_select` fixtures) if the picker lands near `vite_select`, or manual verification via tmux-driven interactive runs otherwise.

## Open Questions

1. Picker relevance ranking is in v1 (likely-runnable packages sort first, nothing hidden). Is outright filtering of clearly non-runnable packages ever wanted, or does ranking plus search suffice?
2. Should a `VP_DEFAULT_PACKAGE` environment variable override the config value? Env companions are an established pattern (`NX_DEFAULT_PROJECT`, `AMPLIFY_MONOREPO_APP_ROOT`, `VITE_RUBY_SOURCE_CODE_DIR`) and would cover per-environment CI needs; deferred from v1.
3. Should `vp test` join? Probably not: Vitest already has first-class `projects` semantics at the root.
4. Exact CI/non-interactive gate: same TTY check as the `vp run` picker, plus the `CI` environment check used by the global command picker?

## Appendix: Naming Survey for `defaultPackage`

How comparable tools name "the member a root-level command targets when none is specified":

| Tool | Field | Notes |
| --- | --- | --- |
| Ionic CLI | `defaultProject` | active; root config with a `projects` map |
| Nx | `defaultProject` | deprecated in favor of `NX_DEFAULT_PROJECT` env var |
| Angular CLI | `defaultProject` | deprecated in favor of cwd inference |
| Cargo | `workspace.default-members` | plural, fan-out semantics |
| Salesforce DX | `default: true` on the member | marker pattern; needs member enumeration |
| Vercel / Netlify / Amplify | `rootDirectory` / `base` / `appRoot` | per-app deploy config, not a default among many |
| GitHub Actions | `defaults.run.working-directory` | names the mechanism (cwd) |

The pattern across these tools is `default` plus the tool's own noun for the unit: Angular, Nx, and Ionic call members "projects" everywhere, Cargo says "members", Salesforce says "package directories". vp's noun is "package" (the picker, `vp run` docs, `vite_workspace`, pnpm vocabulary), hence `defaultPackage`.

Rejected: `defaultProject` (collides with Vitest `test.projects`, a different kind of unit, and breaks noun consistency with the picker), `defaultWorkspace` (in vp/pnpm vocabulary "workspace" means the whole monorepo; the flags `-w`/`--workspace-root` would contradict it), `defaultMembers` (fan-out plural; "member" is meaningless in the non-workspace Laravel shape), `appRoot`/`rootDirectory`/`base` (collide with Vite's `root`/`base` options), member markers a la Salesforce (require enumeration, impossible without workspace metadata). The Angular and Nx deprecations do not transfer: Angular's reason (prefer cwd inference) is built into the resolution order, and Nx's reason (per-environment flexibility) is answered by the env override in open question 2.

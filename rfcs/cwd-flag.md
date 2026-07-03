# RFC: Global `-C` Flag for Working-Directory Switching

## Summary

Add a global `-C <dir>` flag to vp, then use it to fix the app-command experience in monorepos. Everything is additive and backward compatible:

1. **`-C <dir>` global flag** (the feature): for every vp command, `vp -C <dir> <cmd>` behaves exactly like `cd <dir> && vp <cmd>`, following the `git -C` / `make -C` convention: the first-class "run there" form vp currently lacks. Positional semantics stay untouched: `vp dev <path>` keeps upstream Vite semantics (`root` only), and `vp pack` positionals stay tsdown entries.
2. **App-command UX built on `-C`**: running `vp dev` / `vp build` / `vp preview` / `vp pack` bare at a workspace root elicits the missing target instead of silently running against the root: an interactive fuzzy package picker in a TTY, a `defaultPackage` config for repos with one blessed target, and a clear listing plus exit 1 when non-interactive. All three are defined as an implicit `-C <dir>`.

In the common flows users never type `-C`: bare `vp dev` at the root goes through the picker or `defaultPackage`, and inside a package it runs there as today. `-C` is the explicit, teachable form underneath. The commands stay singular: `vp dev` still starts exactly one Vite dev server in one directory. Running a task across many packages at once remains the job of `vp run` (`-r`, `--filter`).

## Motivation

### Current Pain Points

**1. vp has no first-class "run this command in that directory" form.**

For `dev`/`build`/`preview`, the positional is forwarded verbatim to Vite's `[root]`, which re-bases config lookup and `.env` loading, but `process.cwd()` of the Vite process stays at the invocation directory. Any cwd-relative read in a config or plugin diverges even though `root` points at the right app:

```ts
// apps/admin/vite.config.ts
const cert = fs.readFileSync(path.resolve('certs/dev.pem')) // cwd-relative
```

```
$ cd apps/admin && vp dev          # cwd = apps/admin, cert found

  VITE+ v0.2.2

  ➜  Local:   http://localhost:5173/

$ vp dev apps/admin                # root is right, cwd is still the repo root
failed to load config from /acme/apps/admin/vite.config.ts
error when starting dev server:
Error: ENOENT: no such file or directory, open '/acme/certs/dev.pem'
```

For `pack`, directories do not work at all: the positional means entry files/globs (`packages/cli/src/pack-bin.ts`) and config always resolves from `process.cwd()`:

```
$ vp pack packages/ui
ℹ entry: packages/ui
ℹ Build start
error: Build failed with 1 error:

[UNRESOLVED_ENTRY] Cannot resolve entry module packages/ui.
```

So the only form that works reliably, uniformly, for every command, is `cd <path> && vp <cmd>`, and vp offers no flag equivalent of it.

**2. At a monorepo root, the app commands are silently wrong.**

The workspace root usually has no app, but `vp dev` happily starts a server pointed at it:

```
$ vp dev

  VITE+ v0.2.2

  ➜  Local:   http://localhost:5173/        # opens to a 404, no index.html here
```

Nothing errors, nothing guides the user toward the right invocation, and the server exposes the whole repository tree. Fixing this requires eliciting a target, and eliciting a target requires a well-defined primitive to expand to. `-C` is that primitive.

All of the failures above are reproducible with `vite-plus@0.2.2`: https://github.com/why-reproductions-are-required/vite-plus-monorepo-app-commands-repro

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

### 1. `-C`: run any vp command in another directory

These do the same thing, byte for byte:

```bash
vp -C apps/admin dev
cd apps/admin && vp dev
```

It is not limited to the app commands:

```bash
vp -C apps/web test
vp -C apps/web run build
```

Args after the subcommand pass through unchanged:

```
$ vp -C apps/admin dev --port 4000

  VITE+ v0.2.2

  ➜  Local:   http://localhost:4000/
```

And `-C` gives pack its missing directory form:

```
$ vp -C packages/ui pack
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  0.10 kB │ gzip: 0.11 kB
✔ Build complete in 9ms
```

`vp dev apps/admin` (the positional) is untouched: `root` is set, cwd is not, so the pain point 1 pitfall remains on that form (see Decisions).

### 2. `vp dev` at the workspace root (interactive terminal)

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

Enter confirms, prints the teaching hint once, then runs as an implicit `-C`:

```
Selected package: admin (apps/admin)
Tip: run this directly with `vp -C apps/admin dev`

  VITE+ v0.2.2

  ➜  Local:   http://localhost:5173/
  ➜  Network: use --host to expose
```

Escape clears the search, Ctrl+C cancels with exit code 130 and runs nothing (matching the task picker). `vp build`, `vp preview`, and `vp pack` at the root look the same, with `Select a package to build` / `preview` / `pack`.

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

  Pass a directory:  vp -C apps/web build
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

There is no `pnpm-workspace.yaml` or `workspaces` field to enumerate, so the picker cannot serve this shape. `defaultPackage` can:

```ts
// vp reads this key via static extraction and never executes this file, so
// the missing vite-plus install at this root is fine. Vite never loads this
// config either; at this root it is purely a pointer for vp.
export default {
  defaultPackage: './frontend',
}
```

Bare app commands at the root now behave as `vp -C ./frontend <cmd>`, with one line of output so it never feels magical:

```
$ vp dev
vp dev: using ./frontend (defaultPackage)

  VITE+ v0.2.2

  ➜  Local:   http://localhost:5173/
```

An explicit `-C` still wins: `vp -C apps/admin dev` ignores `defaultPackage`.

### 5. Inside a sub-package: nothing changes

```
$ cd apps/web
$ vp dev

  VITE+ v0.2.2

  ➜  Local:   http://localhost:5173/
```

No picker ever appears below the root.

## Command Syntax

```
vp [-C <dir>] <command> [args...]
```

### The `-C <dir>` global flag

- A vp-global flag, parsed before the subcommand like `git -C` / `make -C`, never forwarded to the underlying tool. It works with every vp command.
- Semantics: run the command exactly as if invoked in `<dir>`. The directory is resolved against the invocation cwd; a missing directory errors with `directory not found`.
- The name follows pnpm: its global `-C <path>` is documented as "Run as if pnpm was started in `<path>` instead of the current working directory", which is this flag's semantics verbatim. Short form only in v1 (git style); pnpm's `--dir` long alias can be added compatibly later if wanted.
- Because it sits before the subcommand, it cannot collide with Vite or tsdown flags, present or future. The subcommands themselves gain zero flags.

### Positionals and forwarded args: unchanged

Everything after the subcommand is forwarded verbatim, exactly as today. `vp dev <path>` keeps upstream Vite semantics (positional = `root` option), `vp pack` positionals stay tsdown entries, and relative option values keep resolving against the process cwd (the invocation directory, or `<dir>` under `-C`). There is no directory-vs-entry disambiguation anywhere.

Rejected alternatives: repurposing the app-command positional to mean "run there" (breaks Vite CLI parity; see Decisions), a per-command picker-forcing flag, and `-F`-style name filters (`-F` already has other meanings on `pack` and `run`/`exec`).

## Behavior

### Target directory resolution

An app command invocation is **bare** when it has no `-C` and no positional target (no Vite `[root]`, no pack entries); flags alone keep it bare. For `vp dev` / `build` / `preview` / `pack`, the target directory is resolved in this order:

1. **`-C <dir>`**: run there. Never triggers the picker.
2. **Positional target present**: forward as today, upstream semantics, vp does not interfere.
3. **`defaultPackage`**, when bare in the directory containing the root config (a workspace root, or the root of a non-workspace repo): implicit `-C`, print a one-line note.
4. **Interactive picker**, when bare at the workspace root in an interactive TTY (and not CI): pick, print hint, run as implicit `-C`.
5. **Non-interactive and bare at the workspace root**: print the package list and the `-C` hint, exit 1.
6. **Anywhere else**: current behavior, run in the current directory.

"Workspace root" means the current directory's package is the workspace root package, as determined by `vite_workspace::find_workspace_root` (already called on every invocation in `packages/cli/binding/src/cli/mod.rs`).

### Equivalence invariant

For every vp command:

```
vp -C <dir> <cmd> [args...]  ===  cd <dir> && vp <cmd> [args...]
```

The child's spawn cwd is `<dir>`, so config lookup, `.env` loading, `process.cwd()` reads in configs and plugins, and relative CLI args all behave as if the user had `cd`'d. The Rust layers never mutate their own cwd; the one exception is the local `vp` bin, which applies `-C` by changing its process cwd at startup before any dispatch, indistinguishable from having been started in `<dir>`.

The global binary also resolves the local `vite-plus` install from `<dir>`, matching `cd` exactly; through the package's own `vp` bin the executing CLI is already chosen, so there the invariant assumes a single Vite+ version per workspace (the supported monorepo model).

### Entry points and version assumption

- `-C` is parsed by both the global binary and the local bin; the picker and `defaultPackage` live in the local CLI's NAPI binding (`execute_direct_subcommand`), which every entry point executes. Bare `vp dev` at a root is primarily a global-CLI experience, but a root-level `"dev": "vp dev"` script flows through the same logic.
- Pre-1.0, both the global CLI and any local install are assumed to ship this feature; no version negotiation with older CLIs is specified. In the non-workspace shape the root has no local install, so the bundled CLI executes end to end, which is equivalent under this assumption.

### Picker contents

- One row per workspace package: name plus relative path. Nothing is filtered out; packages that look runnable for the command (`vite.config.*` or `index.html` for `dev`/`build`/`preview`, a pack config or library entry for `pack`) rank first, then by path, so apps surface at the top while everything stays searchable.
- Fuzzy search over name and path via `vite_select::fuzzy_match`, paging identical to the task picker.
- A runnable workspace root appears as a `(workspace root)` entry, keeping today's "run at root" behavior one keystroke away.
- With exactly one likely-runnable package, the picker auto-selects it, printing only the `Selected package:` line and the tip.

### `defaultPackage` config

```ts
export default defineConfig({
  // Relative to the config file's directory. Used by vp dev/build/preview/pack
  // when invoked bare next to this config: an implicit -C.
  defaultPackage: './frontend',
})
```

- Type: `string`, a single directory. A per-command map can come later if real demand appears.
- Consulted when a bare app command runs in the directory containing the root config: a workspace root, or a non-workspace repo root. The non-workspace shape has no package list, so `defaultPackage` is the only mechanism that covers it. An explicit `-C` always wins.
- A missing directory errors: `defaultPackage points to a missing directory: ./frontend`.
- Read via static extraction (`vite_static_config` + the loader in `packages/cli/binding/src/cli/handler.rs`), like `run` config. At a non-workspace root there is no install to execute the config, so the file must work unexecuted: a plain default-export object with a static string value. If extraction fails and no local install can execute the config, vp errors and names the offending construct.

## Decisions

### Vite CLI parity preserved; `-C` carries the `cd` semantics

The core tension: parity with `vite <path>` (positional sets `root` only, cwd untouched) and parity with `cd <path> && vp dev` cannot both hold on the same positional. An earlier draft repurposed the positional and accepted a permanent divergence from the upstream CLI. This RFC keeps the positional fully Vite-compatible and puts the `cd` semantics on a new, explicitly named channel instead. Nothing existing changes meaning, pack needs no directory-vs-entry heuristic, and the primitive generalizes to every vp command instead of four.

The accepted cost: two ways to pass a directory with different semantics. Mitigation: users rarely type either (bare `vp dev` plus picker or `defaultPackage` covers the common flows), and every hint, error, and doc teaches only `-C`.

Mechanism: `process.chdir()` in the CLI process was rejected as a global mutation that leaks into everything sharing the process. vp is a launcher: the NAPI binding always spawns the tool as a fresh child (`packages/cli/binding/src/cli/execution.rs`), so the child's spawn cwd is free to set, with no upstream change.

### Root-only interactivity

Below the root the cwd already identifies the project, so prompting would be noise. At the root the command is ambiguous and silently wrong today; that is where a prompt earns its keep. Unlike bare `vp run`'s informational listing, the app commands exit 1 when non-interactive, because building or serving the wrong directory is worse than failing loudly.

### Elicitation scope

`-C` is global and works with every command. The elicitation behaviors (picker, `defaultPackage`, root error) apply only to the single-target app commands, because only they are ambiguous at the root. Tree-scoped commands (`test`, `lint`, `fmt`, `check`) mean "the whole repo" there, which is their desired behavior. Workspace-state commands (`install`, `add`, `outdated`, ...) have the root as their natural home. Orchestrators (`run`, `exec`) own their selection models and remain the way to run one task across many packages. A future command joins the elicitation set exactly when its subject is one package directory.

## Implementation Architecture

All changes live in the Rust layers; no upstream Vite or tsdown changes are required.

- `crates/vite_global_cli/src/cli.rs`: parse the global `-C <dir>`; resolve the local install from `<dir>` and delegate with `<dir>` as the effective cwd.
- `packages/cli/binding/src/cli/types.rs` / `mod.rs`: parse `-C` on the local bin path; in `execute_direct_subcommand`, add the bare-invocation resolution order (workspace-root detection already happens here).
- `packages/cli/binding/src/cli/execution.rs`: spawn the child with cwd set to the target directory.
- Picker: reuse `vite_select` and `vite_workspace`, both already dependencies via the `vite_task` crates.
- `defaultPackage`: extend the `VitePlusConfigLoader` static extraction the same way `run` config is loaded, and add `defaultPackage?: string` to `packages/cli/src/define-config.ts`.
- `packages/cli/src/pack-bin.ts` needs no change: positional handling is untouched and `-C` never reaches it.
- Docs: a `-C` entry in the global CLI docs, `docs/guide/monorepo.md` "App Commands", and a `docs/config/` page for the new key.

## Compatibility

Every existing invocation is unchanged. The only behavior change is the bare app command at a workspace root, which goes from "silently serve or build the root" to picker / config / clear error. A runnable root stays available as a picker entry, and `defaultPackage: '.'` restores the old behavior unconditionally.

## Snap Tests

Non-interactive branches are covered by snap tests:

- `vp -C <dir> build` / `vp -C <dir> pack` / `vp -C <dir> run <task>`, plus `-C` with a missing directory.
- Parity regression: `vp dev <dir>` still forwards the positional as Vite `root` with cwd untouched.
- Bare app commands at a workspace root without a TTY: package listing and exit code.
- `defaultPackage`: happy path and missing-directory error.
- Equivalence checks: `vp -C <dir> build` and `cd <dir> && vp build` produce the same output in a fixture whose config reads `process.cwd()`.

The interactive picker gets pty snapshot coverage in the `vite_task` repo style (`task_select` fixtures) if the picker lands near `vite_select`, or manual verification via tmux-driven interactive runs otherwise.

## Open Questions

1. Does ranking plus search suffice, or is outright filtering of non-runnable packages ever wanted?
2. Add a `VP_DEFAULT_PACKAGE` env override later? Env companions are an established pattern (`NX_DEFAULT_PROJECT`); deferred from v1.
3. Should `vp test` join the elicitation set? Probably not: Vitest already has first-class `projects` semantics at the root (`-C` works with it regardless).
4. Exact non-interactive gate: the `vp run` picker's TTY check plus the `CI` check used by the global command picker?
5. Should `vp dev <dir>` print a one-line tip pointing at `vp -C <dir> dev`, or would that be noise on a fully supported upstream form?

## Appendix: Naming Survey for `defaultPackage`

How comparable tools name "the member a root-level command targets when none is specified":

| Tool | Field | Notes |
| --- | --- | --- |
| Ionic CLI | `defaultProject` | active; root config with a `projects` map |
| Nx | `defaultProject` | deprecated in favor of `NX_DEFAULT_PROJECT` env var |
| Angular CLI | `defaultProject` | deprecated in favor of cwd inference |
| Cargo | `workspace.default-members` | plural: root `cargo build` builds all listed members |
| Salesforce DX | `default: true` on the member | marker pattern; needs member enumeration |
| Vercel / Netlify / Amplify | `rootDirectory` / `base` / `appRoot` | per-app deploy config, not a default among many |
| GitHub Actions | `defaults.run.working-directory` | names the mechanism (cwd) |

The pattern is `default` plus the tool's own noun for the unit: Angular, Nx, and Ionic say "project", Cargo says "members", Salesforce says "package directories". vp's noun is "package" (the picker, `vp run` docs, `vite_workspace`, pnpm vocabulary), hence `defaultPackage`.

Rejected: `defaultProject` (collides with Vitest `test.projects`, and the picker says "package"), `defaultWorkspace` ("workspace" means the whole monorepo in vp/pnpm vocabulary), `defaultMembers` (plural, implies running in many packages; meaningless without a workspace), `appRoot`/`rootDirectory`/`base` (collide with Vite's `root`/`base` options), member markers (need enumeration, impossible without workspace metadata). The Angular and Nx deprecations do not transfer: cwd inference is built into the resolution order, and per-environment flexibility is open question 2.

The `-C` scheme does not change this conclusion. Tools with `-C`-style flags (git, make, tar, ninja, terraform, pnpm, yarn, bun) ship the flag with no config-file default at all, and tools that do have a directory config name it after the mechanism precisely because it applies to everything they run (just's `set working-directory`, GitHub Actions' `defaults.run.working-directory`, per-task `cwd` in vp's own `run.tasks`). `defaultPackage` is neither: it selects a member, only for the app commands, only when bare at the root. A mechanism name like `defaultCwd` or `defaultDir` would promise vp-wide effect it does not have; the member-selection name matches its member-selection scope.

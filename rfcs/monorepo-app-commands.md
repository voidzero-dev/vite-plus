# RFC: App Commands at the Monorepo Root (`vp dev` / `vp build` / `vp preview` / `vp pack`)

## Summary

Three changes make the built-in app commands useful and predictable in monorepos:

1. **Path equivalence**: `vp dev <path>` behaves exactly like `cd <path> && vp dev`, by spawning the underlying tool with its working directory set to `<path>`. This also fixes `vp pack <path>`, which today misinterprets a directory path as an entry glob.
2. **Interactive package picker at the workspace root**: running an app command at a monorepo root in an interactive terminal opens a fuzzy-searchable package selector (the `vite_select` component behind the `vp run` task picker). Selecting a package runs the command there and prints a hint teaching the direct form (`vp dev apps/web`).
3. **`defaultPackage` config**: a root `vite.config.ts` can set the default target directory, skipping the picker. This also covers framework monorepos that are not JS workspaces (a Laravel, Rails, or Go repo with a `frontend/` directory), where there is no package list to enumerate.

The commands stay singular: `vp dev` still starts exactly one Vite dev server, and the picker only elicits the one argument the command needs. Fan-out stays with `vp run`.

## Motivation

### Current Pain Points

**1. At a monorepo root, the app commands are silently wrong.**

The workspace root usually has no app, but `vp dev` happily starts a server pointed at it:

```
$ vp dev

  VITE v7.1.4  ready in 312 ms

  ➜  Local:   http://localhost:5173/        # opens to a 404, no index.html here
```

Nothing errors, nothing guides the user toward `vp dev apps/web` or `vp run`.

**2. `vp dev <path>` and `cd <path> && vp dev` are not equivalent.**

The positional is forwarded verbatim to Vite's `[root]`, which re-bases config lookup and `.env` loading, but `process.cwd()` of the Vite process stays at the invocation directory. So `process.cwd()` reads in configs and plugins, relative CLI arguments, and local `vite-plus` resolution all differ between the two forms. The only reliable form is `cd <path> && vp dev`, which undercuts `vp dev <path>` as the documented mechanism (`docs/guide/monorepo.md`, "App Commands").

**3. `vp pack <path>` does not work for directories.**

Pack's positional means entry files/globs (`packages/cli/src/pack-bin.ts`) and its config always resolves from `process.cwd()`, so `vp pack packages/ui` bundles a directory glob against the root's config:

```
$ vp pack packages/ui
✗ Error: cannot resolve entry: packages/ui
```

Directory targeting exists only via `--root` / `-W` / `-F`, inconsistent with `vp dev <path>`.

**4. Neither Vite's `root` option nor in-process `chdir` can close the gap.**

Vite resolves `root` without touching `process.cwd()`, by design. So a cwd-relative read in a config or plugin diverges even when `root` points at the right app:

```ts
// apps/admin/vite.config.ts
const cert = fs.readFileSync(path.resolve('certs/dev.pem')) // cwd-relative
```

```
$ cd apps/admin && vp dev          # cwd = apps/admin, cert found

  VITE v7.1.4  ready in 298 ms

$ vp dev apps/admin                # root is right, cwd is still the repo root
failed to load config from /acme/apps/admin/vite.config.ts
error when starting dev server:
Error: ENOENT: no such file or directory, open '/acme/certs/dev.pem'
```

Calling `process.chdir()` in the CLI process would close the gap but is a global mutation that leaks into everything sharing the process. The way out: `vp` never runs Vite or tsdown in-process; the NAPI binding always spawns a fresh child (`packages/cli/binding/src/cli/execution.rs`), so setting the child's spawn cwd achieves full equivalence with no `process.chdir()` and no upstream change.

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

There is no `pnpm-workspace.yaml` or `workspaces` field to enumerate, so the picker cannot serve this shape. `defaultPackage` can:

```ts
// vp reads this key via static extraction and never executes this file, so
// the missing vite-plus install at this root is fine. Vite never loads this
// config either; at this root it is purely a pointer for vp.
export default {
  defaultPackage: './frontend',
}
```

The app commands at the root now go straight to the configured directory, with one line of output so it never feels magical:

```
$ vp dev
vp dev: using ./frontend (defaultPackage)

  VITE v7.1.4  ready in 301 ms

  ➜  Local:   http://localhost:5173/
```

An explicit path still wins: `vp dev apps/admin` ignores `defaultPackage`. The same key works at a JS workspace root to skip the picker for one blessed app; there `vite-plus` is installed and the usual typed `import { defineConfig } from 'vite-plus'` form applies.

### 5. Inside a sub-package: nothing changes

```
$ cd apps/web
$ vp dev

  VITE v7.1.4  ready in 289 ms

  ➜  Local:   http://localhost:5173/
```

No picker ever appears below the root.

## Command Syntax

```
vp dev     [dir] [vite options...]
vp build   [dir] [vite options...]
vp preview [dir] [vite options...]
vp pack    [dir | ...entries] [pack options...]
```

### The `[dir]` positional

- Only the first positional is considered. When it names an existing directory, `vp` resolves it against the invocation cwd, strips it from the forwarded args, and spawns the tool with it as the working directory. A path-like first positional that does not exist errors with `directory not found`.
- For `dev`/`build`/`preview` this refines Vite's `[root]` positional: same position, same meaning, now with full `cd` semantics.
- For `pack`, an existing directory selects the project directory; file and glob positionals keep their current meaning as entries (`dir/**` still expresses a directory-shaped entry glob).

### Forwarded options

Everything else is forwarded verbatim to the underlying tool. Relative option values (`--config`, `--outDir`, ...) resolve in the target directory, matching the `cd` form. Pack's `--root`, `-W`/`--workspace`, and `-F`/`--filter` keep their tsdown semantics, evaluated after the cwd change: `vp pack apps/web -F ui` equals `cd apps/web && vp pack -F ui`.

### No new flags

The `[dir]` positional is the only vp-level input; every flag stays owned by Vite/tsdown, so vp never collides with the bundled tools' option namespaces. Two flags were considered and dropped: a picker-forcing `--pick` (an explicit path already overrides `defaultPackage`, and below the root the cwd already identifies the target) and `-F`/`--filter` for name-based targeting (`-F` already means tsdown workspace filtering on `pack` and multi-package selection on `run`/`exec`). Both can be added compatibly later, and name-based targeting can also come from letting `[dir]` accept package names.

## Behavior

### Target directory resolution

For `vp dev`, `vp build`, `vp preview`, and `vp pack`, the target directory is resolved in this order:

1. **Explicit path positional** (an existing directory): run there.
2. **`defaultPackage`**, when invoked in the directory containing the root config (a workspace root, or the root of a non-workspace repo): run there, print a one-line note.
3. **Interactive picker**, when invoked at the workspace root in an interactive TTY (and not CI): pick, print hint, run there.
4. **Non-interactive at the workspace root**: print the package list and the direct-form hint, exit 1.
5. **Anywhere else** (sub-package or non-workspace project): current behavior, run in the current directory.

"Workspace root" means the current directory's package is the workspace root package, as determined by `vite_workspace::find_workspace_root` (already called on every invocation in `packages/cli/binding/src/cli/mod.rs`).

### Equivalence invariant

After this RFC the following holds and is documented:

```
vp <cmd> <path> [args...]  ===  cd <path> && vp <cmd> [args...]
```

for `cmd` in `dev`, `build`, `preview`, `pack`. The child's spawn cwd is `<path>`, so config lookup, `.env` loading, `process.cwd()` reads in configs and plugins, and relative CLI args all behave as if the user had `cd`'d. The parent `vp` process never calls `process.chdir()`.

The local `vite-plus` CLI itself is still resolved from the invocation directory. The invariant therefore assumes a workspace uses a single Vite+ version, which is already the supported monorepo model; with one version installed, both forms resolve the same CLI.

### Entry points and version assumption

- The feature lives in the local CLI's NAPI binding (`execute_direct_subcommand`), which every entry point executes: the global `vp` binary delegates to the nearest local `vite-plus` install (or to its bundled copy when none exists), and the package's own `vp` bin (`pnpm exec vp dev`, `package.json` scripts) calls it directly. No global-CLI changes are needed.
- Bare `vp dev` at an arbitrary root is primarily a global-CLI experience; local-only setups usually go through per-package scripts. A root-level `"dev": "vp dev"` script flows through the same logic and gets the same behavior.
- Pre-1.0, both the global CLI and any local install are assumed to ship this feature; no version negotiation with older CLIs is specified. In the non-workspace shape the root has no local install, so the bundled CLI executes end to end, which is equivalent under this assumption; re-resolving a local CLI from the target directory is out of scope for v1.

### Picker contents

- One row per workspace package: name plus relative path. Nothing is filtered out; packages that look runnable for the command (`vite.config.*` or `index.html` for `dev`/`build`/`preview`, a pack config or library entry for `pack`) rank first, then by path, so apps surface at the top while everything stays searchable.
- Fuzzy search over name and path via `vite_select::fuzzy_match`, paging identical to the task picker.
- A runnable workspace root appears as a `(workspace root)` entry, keeping today's "run at root" behavior one keystroke away.
- With exactly one likely-runnable package, the picker auto-selects it, printing only the `Selected package:` line and the tip.

### `defaultPackage` config

```ts
export default defineConfig({
  // Relative to the config file's directory. Used by vp dev/build/preview/pack
  // when invoked next to this config without an explicit path.
  defaultPackage: './frontend',
})
```

- Type: `string`, a single directory. A per-command map can come later if real demand appears.
- Consulted when `vp` runs in the directory containing the root config: a workspace root, or a non-workspace repo root. The non-workspace shape has no package list, so `defaultPackage` is the only mechanism that covers it. An explicit positional always wins.
- A missing directory errors: `defaultPackage points to a missing directory: ./frontend`.
- Read via static extraction (`vite_static_config` + the loader in `packages/cli/binding/src/cli/handler.rs`), like `run` config. At a non-workspace root there is no install to execute the config, so the file must work unexecuted: a plain default-export object with a static string value. If extraction fails and no local install can execute the config, vp errors and names the offending construct.

## Decisions

### Root-only interactivity

Below the root the cwd already identifies the project, so prompting would be noise. At the root the command is ambiguous and silently wrong today; that is where a prompt earns its keep. Unlike bare `vp run`'s informational listing, the app commands exit 1 when non-interactive, because building or serving the wrong directory is worse than failing loudly.

### Command scope

Only single-target app commands get target elicitation, because only they are ambiguous at the root. Tree-scoped commands (`test`, `lint`, `fmt`, `check`) mean "the whole repo" there, which is their desired behavior. Workspace-state commands (`install`, `add`, `outdated`, ...) have the root as their natural home. Orchestrators (`run`, `exec`) own their selection models and remain the fan-out tools. A future command joins exactly when its subject is one package directory.

## Implementation Architecture

All changes live in the Rust layers; no upstream Vite or tsdown changes are required.

- `packages/cli/binding/src/cli/resolver.rs`: detect a leading existing-directory positional, strip it, and carry it as the target directory.
- `packages/cli/binding/src/cli/execution.rs`: spawn the child with cwd set to the target directory.
- `packages/cli/binding/src/cli/mod.rs` (`execute_direct_subcommand`): workspace-root detection already happens here; add the resolution order.
- Picker: reuse `vite_select` and `vite_workspace`, both already dependencies via the `vite_task` crates.
- `defaultPackage`: extend the `VitePlusConfigLoader` static extraction the same way `run` config is loaded, and add `defaultPackage?: string` to `packages/cli/src/define-config.ts`.
- `packages/cli/src/pack-bin.ts` needs no change once the binding strips the positional; the global CLI needs no change since it already delegates with the invocation cwd.
- Docs: `docs/guide/monorepo.md` "App Commands" and a `docs/config/` page for the new key.

## Compatibility

- `vp dev <path>` / `vp build <path>`: the cwd-dependent edge cases (cwd reads in configs and plugins, relative CLI args) now match the `cd` form; the delta is exactly the set of cases currently reported as bugs. Local CLI resolution is unchanged. Ship in a minor with a changelog note.
- `vp pack <path>` with a directory changes from an error to packing that directory; file and glob entries are unaffected.
- At a workspace root, picker / config / clear error replaces "silently serve the root". A root that is itself runnable stays available as a picker entry, and `defaultPackage: '.'` restores the old behavior unconditionally.
- Sub-package and non-workspace invocations are unchanged.

## Snap Tests

Non-interactive branches are covered by snap tests:

- `vp build <dir>` / `vp pack <dir>` directory positionals (none exist today).
- App commands at a workspace root without a TTY: package listing and exit code.
- `defaultPackage`: happy path and missing-directory error.
- Equivalence checks: `vp build <dir>` and `cd <dir> && vp build` produce the same output in a fixture whose config reads `process.cwd()`.

The interactive picker gets pty snapshot coverage in the `vite_task` repo style (`task_select` fixtures) if the picker lands near `vite_select`, or manual verification via tmux-driven interactive runs otherwise.

## Open Questions

1. Does ranking plus search suffice, or is outright filtering of non-runnable packages ever wanted?
2. Add a `VP_DEFAULT_PACKAGE` env override later? Env companions are an established pattern (`NX_DEFAULT_PROJECT`); deferred from v1.
3. Should `vp test` join? Probably not: Vitest already has first-class `projects` semantics at the root.
4. Exact non-interactive gate: the `vp run` picker's TTY check plus the `CI` check used by the global command picker?

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

The pattern is `default` plus the tool's own noun for the unit: Angular, Nx, and Ionic say "project", Cargo says "members", Salesforce says "package directories". vp's noun is "package" (the picker, `vp run` docs, `vite_workspace`, pnpm vocabulary), hence `defaultPackage`.

Rejected: `defaultProject` (collides with Vitest `test.projects`, and the picker says "package"), `defaultWorkspace` ("workspace" means the whole monorepo in vp/pnpm vocabulary), `defaultMembers` (fan-out plural, meaningless without a workspace), `appRoot`/`rootDirectory`/`base` (collide with Vite's `root`/`base` options), member markers (need enumeration, impossible without workspace metadata). The Angular and Nx deprecations do not transfer: cwd inference is built into the resolution order, and per-environment flexibility is open question 2.

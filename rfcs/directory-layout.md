# RFC: Split Directory Layout via `VpDirs`

## Status

**Partially implemented** — fresh-install split layout + centralized resolution ship in [#2346](https://github.com/voidzero-dev/vite-plus/pull/2346) (closes [#827](https://github.com/voidzero-dev/vite-plus/issues/827)). Automatic on-disk migration and full `VP_HOME` cleanup are follow-ups ([#2371](https://github.com/voidzero-dev/vite-plus/issues/2371), [#2372](https://github.com/voidzero-dev/vite-plus/issues/2372)).

## Background

Vite+ historically stores the entire global install under a single monolithic root:

```text
~/.vite-plus/
├── bin/                 # shims (vp, node, npm, …)
├── current → <version>/
├── <version>/           # CLI payload + node_modules
├── js_runtime/
├── package_manager/
├── packages/
├── bins/
├── cache/
├── env, env.fish, …     # shell env scripts
├── config.json
└── …
```

That layout is simple to install and document, but it conflicts with platform conventions:

1. **XDG / platform split** — binaries, data, cache, config, and state belong in different roots (`~/.local/bin`, `~/.local/share`, `~/.cache`, `~/.config`, `~/.local/state` on Unix; analogous Local/Roaming app dirs on Windows).
2. **PATH hygiene** — a dedicated `~/.local/bin` (or `%LOCALAPPDATA%\vite-plus\bin`) is the usual place for user tools; burying shims under a private tree forces a custom PATH entry forever.
3. **Scattered path construction** — call sites historically joined `~/.vite-plus/...` or read `VP_HOME` ad hoc, making layout changes error-prone.
4. **Testing friction** — snapshot and CI setups pin `VP_HOME` to force a single tree, which couples fixtures to the legacy shape.

## Goals

1. **Centralize** all on-disk category roots and first-level data subdirectories in `vp_shared::VpDirs` so no call site invents `~/.vite-plus/...` or reads `XDG_*` itself.
2. **Default fresh installs** to the split XDG / platform layout.
3. **Grandfather** existing default installs that still live at `~/.vite-plus` (or `./.vite-plus`) without moving files in this phase.
4. Keep **`VP_HOME` as a deprecated full-root pin** for custom roots and older scripts; prefer `VP_BIN_DIR` / `VP_DATA_DIR` / `VP_CACHE_DIR`.
5. Align **installers** (`install.sh`, `install.ps1`, `vp-setup`, `install-global-cli`) with the same resolution strategy as the CLI.
6. Support **implode, env setup, trampoline, upgrade check**, and related flows on both layouts.

## Non-Goals (this phase)

1. **Automatic migration** of an existing `~/.vite-plus` tree into split roots (tracked in [#2372](https://github.com/voidzero-dev/vite-plus/issues/2372); see [Follow-up: layout migrate](#follow-up-layout-migrate-on-vp-upgrade)).
2. Removing the deprecated **read** of `VP_HOME` from the resolution chain (cleanup of _setters_ is [#2371](https://github.com/voidzero-dev/vite-plus/issues/2371)).
3. Introducing a new distribution channel or package format.
4. Changing the on-disk _payload_ shape under a version directory (`current`, version dirs, `node_modules`).

## Design

### Ownership: `VpDirs`

`crates/vp_shared/src/dirs.rs` owns only:

| Layer                      | Examples                                                                           |
| -------------------------- | ---------------------------------------------------------------------------------- |
| **Category roots**         | `bin_dir`, `data_dir`, `cache_dir`, `config_dir`, `state_dir`                      |
| **First-level under data** | `current_dir`, `js_runtime_dir`, `package_manager_dir`, `packages_dir`, `bins_dir` |

Files and deeper trees (`config.json`, `js_runtime/node/<ver>`, `resolve_cache.json`, …) are joined by the owning feature, not by `VpDirs`.

Resolution is **recomputed on every call** (cheap joins + at most a few existence checks) so process env changes and test `temp_env` overrides are observed without a process-wide path cache.

`VpDirs::is_legacy_layout()` is true when `data_dir` is named `.vite-plus` and `bin_dir` is that root’s `bin` child (the legacy on-disk mapping).

### Resolution chain

Each category walks the following ordered sources. A source either proposes a path or is skipped. Acceptance is gated by a fallthrough strategy:

| Strategy  | Meaning                                                                                                                                                                                                 |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Set**   | Use the proposed path as soon as it is configured (env overrides, platform defaults, first install).                                                                                                    |
| **Exist** | Use the path only when the install root already exists on disk (so grandfathering does not claim empty paths; bin/cache under an existing root are accepted even if those subdirs are not created yet). |

**Unix:**

```text
VP_HOME
  → existing ~/.vite-plus
  → existing ./.vite-plus
  → VP_BIN_DIR / VP_DATA_DIR / VP_CACHE_DIR
  → XDG_BIN_HOME / XDG_DATA_HOME / XDG_CACHE_HOME / XDG_CONFIG_HOME / XDG_STATE_HOME
  → platform defaults
```

**Windows:** same head; no XDG step — after `VP_*_DIR`, fall through to Windows platform defaults (`%LOCALAPPDATA%` / `%APPDATA%`).

| Source                                            | Fallthrough | Behavior                                                                                                                                                                                                                           |
| ------------------------------------------------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`VP_HOME`** (deprecated)                        | Set         | When set, pins the **legacy monolithic mapping** under that root for all categories.                                                                                                                                               |
| **`~/.vite-plus`**                                | Exist       | When that directory exists, use the legacy mapping under it.                                                                                                                                                                       |
| **`./.vite-plus`**                                | Exist       | When present in the process cwd, same legacy mapping (project-local / tests).                                                                                                                                                      |
| **`VP_BIN_DIR` / `VP_DATA_DIR` / `VP_CACHE_DIR`** | Set         | Absolute per-category overrides (relative values ignored). Only the categories with a corresponding variable are proposed here.                                                                                                    |
| **`XDG_*`** (Unix)                                | Set         | Absolute `XDG_BIN_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME`, with app name `vite-plus` on data/cache/config/state. Bin may follow uv-style `$XDG_DATA_HOME/../bin` when only data home is set. |
| **Platform defaults**                             | Set         | See [category mapping](#category-mapping) (Unix XDG-style homes under `$HOME`, Windows Local/Roaming app dirs).                                                                                                                    |

Relative `VP_*` / `XDG_*` values are treated as unset (per the XDG Base Directory Spec for the spec-defined variables). `XDG_BIN_HOME` is not part of the XDG spec; it is a uv-style convention, as is the `$XDG_DATA_HOME/../bin` bin fallback above.

### Category mapping

| Category   | Split default (Unix)       | Split default (Windows)          | Legacy (`VP_HOME` / existing `~/.vite-plus`) |
| ---------- | -------------------------- | -------------------------------- | -------------------------------------------- |
| **bin**    | `~/.local/bin`             | `%LOCALAPPDATA%\vite-plus\bin`   | `<root>/bin`                                 |
| **data**   | `~/.local/share/vite-plus` | `%LOCALAPPDATA%\vite-plus\data`  | `<root>`                                     |
| **cache**  | `~/.cache/vite-plus`       | `%LOCALAPPDATA%\vite-plus\cache` | `<root>/cache`                               |
| **config** | `~/.config/vite-plus`      | `%APPDATA%\vite-plus`            | `<root>`                                     |
| **state**  | `~/.local/state/vite-plus` | `%LOCALAPPDATA%\vite-plus\state` | `<root>`                                     |

Under **data** (both layouts): version directories, `current`, `js_runtime`, `package_manager`, `packages`, `bins`.

### Installers

`install.sh` / `install.ps1` (and local `install-global-cli`) mirror the CLI chain:

1. If `VP_HOME` is set → install into that root as **legacy**.
2. Else if default `~/.vite-plus` (or Windows equivalent) **exists** → **grandfather** legacy root.
3. Else → **split** data/bin/config (and related) using `VP_*_DIR` / `XDG_*` / platform defaults.

Installers deliberately omit the `./.vite-plus` (cwd-local) step from the CLI chain; that step exists only for project-local/test resolution and must not influence where an installer puts files.

There is a **single** install script per platform (no separate `legacy_install.*`). Local bootstrap does **not** force `VP_HOME`; it resolves the install data dir the same way.

Env scripts are written under **config** (split: `~/.config/vite-plus/env*`; legacy: still under the monolithic root). PATH entries point at the resolved **bin** directory.

### Global CLI → JS children

Under the split layout, the global CLI injects `VP_BIN_DIR` / `VP_DATA_DIR` / `VP_CACHE_DIR` into JS child processes when those vars are unset, so the NAPI / local CLI and JS tools see the same category roots without re-implementing XDG logic.

### User impact (this phase)

| Install state           | Behavior                                                                              |
| ----------------------- | ------------------------------------------------------------------------------------- |
| Existing `~/.vite-plus` | Paths unchanged (grandfathered until migrate follow-up).                              |
| Custom `VP_HOME`        | Still works as deprecated full-root pin.                                              |
| Fresh install           | Split layout; typically only `~/.local/bin` (or Windows bin dir) needs to be on PATH. |

### Verified scenarios (manual)

1. **Fresh split** — empty home, no `VP_HOME`: install lands on `~/.local/share/vite-plus`, shims in `~/.local/bin`, env under `~/.config/vite-plus`; `vp --version` works.
2. **Legacy reuse** — pre-seeded `~/.vite-plus` with markers: `install-global-cli` upgrades `current` in place, keeps prior version dirs and markers, does not create split roots; runtime writes `resolve_cache.json` under `~/.vite-plus/cache`; `vp env doctor` reports home `~/.vite-plus`.

## Follow-up: `VP_HOME` cleanup

**Issue:** [#2371](https://github.com/voidzero-dev/vite-plus/issues/2371)

Much of the repo still **sets** or **assumes** `VP_HOME` as the primary install root (especially PTY snapshot tests). That fights the split layout.

**Direction:**

- Prefer `VP_*_DIR` / XDG / platform defaults in tests, CI, and docs.
- Keep **reading** `VP_HOME` in `VpDirs` as a deprecated custom-root pin until a later deprecation cut.
- Snapshot suite should not require a permanent `VP_HOME=~/.vite-plus` baseline for the happy path.

## Follow-up: layout migrate on `vp upgrade`

**Issue:** [#2372](https://github.com/voidzero-dev/vite-plus/issues/2372)

After the split layout ships, stop grandfathering forever: on `vp upgrade` (and installer reinstall where appropriate), migrate a **default** legacy install into split roots and remove `~/.vite-plus`.

### Mapping (Unix defaults; Windows analogous)

| From under `~/.vite-plus`                                                    | To                                                                              |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Version dirs, `current`, runtimes, package managers, packages, bins metadata | data dir (`~/.local/share/vite-plus`, …)                                        |
| `config.json` (and durable user config)                                      | config dir                                                                      |
| Session / upgrade-check state                                                | state dir                                                                       |
| Shims / env scripts                                                          | **regenerate** into bin + config (do not copy relative links or stale env text) |
| Resolve cache                                                                | **drop** (rebuild on next use)                                                  |

Custom `VP_HOME` roots are **out of auto-migrate** (remain a deprecated pin only).

### Locked design constraints

1. **Copy-first**, then delete the legacy root (no long-lived tombstone unless Windows file locks force a deferred cleanup).
2. **Never delete** the legacy root before split `data/current` (and critical shims) are verified.
3. **Conflict** if the split data root already holds a healthy unrelated install — abort with a clear message.
4. Shell profiles that source `~/.vite-plus/env*` must be **rewritten or cleaned** to the new config env path.
5. **N-1 path**: users on a pre-migrate CLI may re-exec after upgrade and/or re-run the install script as the guaranteed fallback.
6. **Immediate** removal of the default legacy root after a successful migrate (product choice: do not leave an empty grandfather forever).

### Acceptance (migrate)

- Machine with only default `~/.vite-plus` runs `vp upgrade` once → split roots populated, `~/.vite-plus` gone, shims/env work after shell restart.
- Fresh install never creates `~/.vite-plus`.
- CI covers legacy → split upgrade (in addition to released-CLI and fresh-split install paths).

> Experimental migrate work was sketched on a side branch and **withdrawn** from the dirs PR because moving a live global install is high risk (Windows locks, PATH/profile cutover, concurrent shims). Re-land only behind careful staging and tests.

## Testing strategy

| Layer      | What                                                                                                                                                    |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Unit       | `vp_shared` resolution / `is_legacy_layout` / fallthrough cases                                                                                         |
| Install CI | `test-standalone-install`: released CLI (often `VP_HOME`-pinned for pre-split packages) + local-build fresh split + grandfather / upgrade-adjacent jobs |
| Snapshots  | Layout isolation without assuming a permanent monolithic home (improve further in #2371)                                                                |
| Manual     | Fresh split install; existing legacy reuse with new CLI                                                                                                 |

## Alternatives considered

1. **Always split; never grandfather** — breaks existing installs until migrate is perfect. Rejected for the first ship.
2. **Always migrate on first run of any command** — surprising and dangerous mid-script. Prefer explicit `vp upgrade` / installer.
3. **Keep monolithic forever; only document XDG as optional** — fails PATH and platform conventions for new users.
4. **Separate install scripts for legacy vs split** — duplicated drift; replaced by one script with resolution branching.

## Open questions (post-migrate)

1. Deprecation timeline for **reading** `VP_HOME` after most users are on split roots.
2. Whether cwd-local `./.vite-plus` remains useful for tests after snapshot fixtures stop relying on it.
3. Windows deferred delete / reboot policy when locked files block legacy root removal.

## References

- Issue: [#827](https://github.com/voidzero-dev/vite-plus/issues/827)
- Implementation PR: [#2346](https://github.com/voidzero-dev/vite-plus/pull/2346)
- Follow-ups: [#2371](https://github.com/voidzero-dev/vite-plus/issues/2371), [#2372](https://github.com/voidzero-dev/vite-plus/issues/2372)
- Code: `crates/vp_shared/src/dirs.rs`, `crates/vp_shared/src/dirs/resolution.rs`
- Installers: `packages/cli/install.sh`, `packages/cli/install.ps1`, `packages/tools/src/install-global-cli.ts`
- Related RFCs: [upgrade-command](./upgrade-command.md), [implode-command](./implode-command.md), [env-command](./env-command.md), [js-runtime](./js-runtime.md)

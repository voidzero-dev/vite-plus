# RFC: Split Directory Layout via `VpDirs`

## Status

**Partially implemented** — fresh-install split layout + centralized resolution ship in [#2346](https://github.com/voidzero-dev/vite-plus/pull/2346) (closes [#827](https://github.com/voidzero-dev/vite-plus/issues/827)). Automatic on-disk migration and full `VP_HOME` cleanup are follow-ups ([#2371](https://github.com/voidzero-dev/vite-plus/issues/2371), [#2372](https://github.com/voidzero-dev/vite-plus/issues/2372)).

## Background

Vite+ historically stores the entire global install under a single monolithic root:

```text
~/.vite-plus/
├── bin/                            # shims (vp, node, npm, …)
├── current → <version>/            # active CLI version symlink
├── <version>/                      # CLI payload (bin/, node_modules/, package.json, pnpm-lock.yaml)
├── js_runtime/                     # managed runtimes (node/<ver>/, *.lock, index_cache.json)
├── package_manager/                # managed package managers (npm/, pnpm/, yarn/, bun/)
├── packages/                       # globally installed packages (@scope/<name>#<id>/, *.lock)
├── bins/                           # bin metadata for installed packages (<name>.json)
├── cache/                          # resolve_cache.json
├── tmp/                            # staging (package installs, create-org downloads)
├── env, env.fish, env.nu, env.ps1  # shell env scripts
├── config.json                     # user config (created on first write)
├── .session-node-version           # session Node version (`vp env use`)
├── .previous-version               # CLI version before the last upgrade
└── .upgrade-check.json             # upgrade-check cache
```

That layout is simple to install and document, but it conflicts with platform conventions:

1. **XDG / platform split** — binaries, data, cache, config, and state belong in different roots (`~/.local/bin`, `~/.local/share`, `~/.cache`, `~/.config`, `~/.local/state` on Unix; analogous Local/Roaming app dirs on Windows).
2. **PATH hygiene** — a dedicated `~/.local/bin` (or `%LOCALAPPDATA%\vite-plus\bin`) is the usual place for user tools; burying shims under a private tree forces a custom PATH entry forever.
3. **Scattered path construction** — call sites historically joined `~/.vite-plus/...` or read `VP_HOME` ad hoc, making layout changes error-prone.
4. **Testing friction** — snapshot and CI setups pin `VP_HOME` to force a single tree, which couples fixtures to the monolithic shape.

## Goals

1. **Centralize** all on-disk category roots and first-level data subdirectories in `vp_shared::VpDirs` so no call site invents `~/.vite-plus/...` or reads `XDG_*` itself.
2. **Default fresh installs** to the split XDG / platform layout.
3. **Grandfather** existing default installs that still live at `~/.vite-plus` without moving files in this phase.
4. Keep **`VP_HOME` as a full-root pin** for custom roots and older scripts; prefer `VP_BIN_DIR` / `VP_DATA_DIR` / `VP_CACHE_DIR` for new configuration.
5. Align **installers** (`install.sh`, `install.ps1`, `vp-setup`, `install-global-cli`) with the same resolution strategy as the CLI.
6. Support **implode, env setup, trampoline, upgrade check**, and related flows on both layouts.

## Non-Goals (this phase)

1. **Automatic migration** of an existing `~/.vite-plus` tree into split roots (tracked in [#2372](https://github.com/voidzero-dev/vite-plus/issues/2372); see [Follow-up: layout migrate](#follow-up-layout-migrate-on-vp-upgrade)).
2. Removing the **read** of `VP_HOME` from the resolution chain (cleanup of _setters_ is [#2371](https://github.com/voidzero-dev/vite-plus/issues/2371)).
3. Introducing a new distribution channel or package format.
4. Changing the on-disk _payload_ shape under a version directory (`current`, version dirs, `node_modules`).

## Design

### Ownership: `VpDirs`

`crates/vp_shared/src/dirs.rs` owns only the **category roots**:

| Root     | Contents                                                                                                                                             |
| -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bin`    | Executables and shims (`<BIN>/vp`, `<BIN>/node`, …)                                                                                                  |
| `data`   | CLI versions, managed runtimes, package managers (`<DATA>/current`, `<DATA>/js_runtime`, `<DATA>/package_manager`, `<DATA>/packages`, `<DATA>/bins`) |
| `cache`  | Disposable caches (`resolve_cache.json`, `.upgrade-check.json`, create-org tarballs)                                                                 |
| `config` | User configuration (`<CONFIG>/env*`, `<CONFIG>/config.json`)                                                                                         |
| `state`  | State files (session version)                                                                                                                        |

`VpDirs` is a **stateful value**: the strategy chain runs once at
construction (`VpDirs::resolve()`), the five roots are stored as public
fields, and process env changes afterwards are not observed (child processes
resolve their own roots from their own environment). The struct carries **no
notion of layout** — every resolution source maps onto the same five roots,
and features must not branch on how the roots were produced.

First-level directories under `data` (`current`, `js_runtime`, …) and all
deeper trees (`config.json`, `js_runtime/node/<ver>`, `resolve_cache.json`,
…) are joined by the owning feature, not by `VpDirs`.

`EnvConfig` **owns** the resolved `VpDirs` (`EnvConfig::get().dirs`),
constructed in `EnvConfig::from_env()`; the dependency is one-way —
`from_env()` resolves the user home once (`HOME`/`USERPROFILE`,
platform-ordered like the installers, with a system base-dirs fallback),
stores it as `EnvConfig.user_home` (`AbsolutePathBuf`), and passes it into
`VpDirs::resolve(home)`, so `user_home` and `dirs` never disagree. Directory
resolution reads only the override env vars (`VP_HOME`, `VP_*_DIR`, `XDG_*`)
— never `HOME`/`USERPROFILE` — and carries no test-only branches: tests
exercise the same resolution chain through the process environment (see
[Test configuration](#test-configuration)).

### Test configuration

`EnvConfig::get()` has two behaviors, selected at compile time:

- **Release builds** read the process env once, lazily, and cache the config
  process-wide (`OnceLock`).
- **Test builds** — `cfg(test)`, or any downstream crate enabling the
  `test-utils` feature through `[dev-dependencies]` — re-resolve on **every**
  `get()`, so env-scoped tests observe pinned values immediately. The
  feature pulls in `temp-env` and `tempfile` as optional dependencies, so
  the helpers stay out of release binaries.

Tests pin the **environment**, never paths: the same env → dirs resolution
chain production uses derives the roots, so fixtures cannot drift from
production resolution. Four `EnvConfig` associated functions (all gated on
`test-utils`) cover the matrix:

| Helper                          | Environment                                                           | Root                                                                     | Use                                                                                  |
| ------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------ |
| `with_vars(vars, \|config\| …)` | declared vars pinned (`None` values unset); everything else inherited | caller-chosen (e.g. `VP_HOME` → own tempdir, or a per-crate shared root) | Tests asserting on pinned values or concrete paths                                   |
| `with_vars_async`               | same, held across `.await`                                            | same                                                                     | Async tests (requires a current-thread runtime — `temp_env`'s lock guard is `!Send`) |
| `scoped(\|config\| …)`          | `VP_HOME` → fresh `tempfile` root                                     | hidden, deleted on scope exit                                            | Dir read/write tests that don't care where the root lives                            |
| `scoped_async`                  | same, across `.await`                                                 | same                                                                     | Async equivalent                                                                     |

Semantics:

- The callback **receives the resolved `Arc<EnvConfig>`** — no manual
  `EnvConfig::get()` needed. For the async helpers the config is resolved
  inside the scoped future, after the variables are pinned.
- **Any** process variable may be pinned; there is no allowlist. Undeclared
  variables are inherited from the process environment as-is.
- Values implement the `EnvValue` trait: plain string/path values set the
  variable, and an `Option` sets it when `Some` / **unsets** it when `None`
  — the only "off" state for presence-checked variables (`CI` → `is_ci`,
  `VP_ENV_USE_EVAL_ENABLE`, …), since assigning a value still counts as set.
  `EnvValue` is implemented for the concrete string/path types rather than
  via `ToString`: paths may be non-UTF-8 and a lossy conversion would
  silently corrupt them.
- The helpers delegate to `temp_env`, which holds a process-wide lock for
  the whole scope — no `#[serial]` needed between scope-based tests, and
  nested scopes shadow outer ones until they return.
- Download-heavy suites (package managers, Node runtimes) pin a **shared**
  `VP_HOME` root per test binary (under `std::env::temp_dir()`) so download
  caches stay warm across tests and runs; concurrent installs under one root
  are lock-protected.

One discipline follows from the shared lock: `temp_env`'s lock is independent
of `serial_test`'s. Within a test binary that uses these scopes, **every**
test that mutates the process environment must go through `temp_env` (or
these helpers) — a raw `set_var`/`remove_var` can otherwise rewrite a
variable mid-scope and corrupt another test's pinned state. Tests mutating
only variables that no scope pins (e.g. `VP_SHIM_TOOL`) may stay on
`#[serial]`.

### Comment convention

Code comments and docs refer to on-disk locations with **category
placeholders** — `<BIN>/xxx`, `<DATA>/xxx`, `<CACHE>/xxx`, `<CONFIG>/xxx`,
`<STATE>/xxx` — never with dual-layout annotations. Do not write
`monolithic: ~/.vite-plus/xxx, split: ~/.local/share/vite-plus/xxx`; the mapping
from placeholder to concrete path is defined once, in
[category mapping](#category-mapping), and must not be restated per call
site.

### Resolution chain

Each category walks the following ordered sources. A source either proposes
a path or is skipped; the first proposal wins. The only stateful source is
`~/.vite-plus`: it proposes only when that directory already exists on disk
**as a directory** (checked once at resolution, matching the installers'
`[ -d ]` / `-PathType Container` gates), so grandfathering does not claim
empty paths; bin/cache under an existing root are accepted even if those
subdirs are not created yet.

**Unix:**

```text
VP_HOME
  → existing ~/.vite-plus
  → VP_BIN_DIR / VP_DATA_DIR / VP_CACHE_DIR
  → XDG_BIN_HOME / XDG_DATA_HOME / XDG_CACHE_HOME / XDG_CONFIG_HOME / XDG_STATE_HOME
  → platform defaults
```

**Windows:** same head; no XDG step — after `VP_*_DIR`, fall through to Windows platform defaults (`%LOCALAPPDATA%` / `%APPDATA%`). When the known-folder query is unavailable (restricted service or CI contexts), the platform step falls back to the conventional `AppData\Local` / `AppData\Roaming` locations under the resolved user home, so a known home always yields a complete layout.

| Source                                            | Behavior                                                                                                                                                                                                                           |
| ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`VP_HOME`**                                     | When set, pins the **monolithic mapping** under that root for all categories.                                                                                                                                                      |
| **`~/.vite-plus`**                                | When that directory exists, use the monolithic mapping under it.                                                                                                                                                                   |
| **`VP_BIN_DIR` / `VP_DATA_DIR` / `VP_CACHE_DIR`** | Absolute per-category overrides (relative values ignored). Only the categories with a corresponding variable are proposed here.                                                                                                    |
| **`XDG_*`** (Unix)                                | Absolute `XDG_BIN_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME`, with app name `vite-plus` on data/cache/config/state. Bin may follow uv-style `$XDG_DATA_HOME/../bin` when only data home is set. |
| **Platform defaults**                             | See [category mapping](#category-mapping) (Unix XDG-style homes under `$HOME`, Windows Local/Roaming app dirs).                                                                                                                    |

Relative `VP_*` / `XDG_*` values are treated as unset (per the XDG Base Directory Spec for the spec-defined variables). `XDG_BIN_HOME` is not part of the XDG spec; it is a uv-style convention, as is the `$XDG_DATA_HOME/../bin` bin fallback above.

### Category mapping

| Category   | Split default (Unix)       | Split default (Windows)          | Monolithic (`VP_HOME` / existing `~/.vite-plus`) |
| ---------- | -------------------------- | -------------------------------- | ------------------------------------------------ |
| **bin**    | `~/.local/bin`             | `%LOCALAPPDATA%\vite-plus\bin`   | `<root>/bin`                                     |
| **data**   | `~/.local/share/vite-plus` | `%LOCALAPPDATA%\vite-plus\data`  | `<root>`                                         |
| **cache**  | `~/.cache/vite-plus`       | `%LOCALAPPDATA%\vite-plus\cache` | `<root>/cache`                                   |
| **config** | `~/.config/vite-plus`      | `%APPDATA%\vite-plus`            | `<root>`                                         |
| **state**  | `~/.local/state/vite-plus` | `%LOCALAPPDATA%\vite-plus\state` | `<root>`                                         |

Under **data** (both layouts): version directories, `current`, `js_runtime`, `package_manager`, `packages`, `bins`.

### Installers

`install.sh` / `install.ps1` (and local `install-global-cli`) mirror the CLI chain:

1. If `VP_HOME` is set → install into that root as **monolithic**.
2. Else if default `~/.vite-plus` (or Windows equivalent) **exists** → **grandfather** the monolithic root.
3. Else → **split** data/bin/config (and related) using `VP_*_DIR` / `XDG_*` / platform defaults.

There is a **single** install script per platform (no separate per-layout install script). Local bootstrap does **not** force `VP_HOME`; it resolves the install data dir the same way.

Env scripts are written under **config** (split: `~/.config/vite-plus/env*`; monolithic: the install root). PATH entries point at the resolved **bin** directory.

### Compatibility with pre-split releases

The installers accept any published `VP_VERSION`, and until the first split-aware release (planned 0.3.0) ships, `latest` also resolves to a pre-split version. A pre-split binary resolves every path from `VP_HOME` (default `~/.vite-plus`): its env setup, shims, trampoline, and `vp upgrade` all assume that monolithic root. Installing such a binary into split roots produces a broken install that still exits 0: the PATH trampoline points at `<bin>/../current` which does not exist, env scripts are sourced from a config dir the binary never writes, and the binary's own env setup builds a second, half-built `~/.vite-plus` tree.

**Detection: capability probe, not a version gate.** After downloading the platform payload and before committing to a layout, every installer (`install.sh`, `install.ps1`, `vp-setup`) runs the payload binary once with `VP_DUMP_DIRS=1`:

- A split-aware binary prints one tab-separated line per category root (`data\t<path>`, `bin\t<path>`, `config\t<path>`) and exits. The installer **adopts these paths verbatim**, so the layout the installer writes and the layout the binary resolves cannot drift.
- A pre-split binary does not know the variable, prints its help, and exits 0 without those lines. The installer then installs into the **monolithic root** (`VP_HOME` if set, else `~/.vite-plus`) and prints a notice (`vite-plus <version> predates the split directory layout; installing to ~/.vite-plus`). Everything the binary later does agrees with that root, so the installed `vp`, `vpr`, `vpx`, `node`, and `npm` commands work.

**Failure direction.** Probe failure also covers a payload that cannot run at all (wrong platform, missing VC++ runtime). The installer then picks the monolithic root and the dependency-install step fails with the real error, as before. The monolithic root works for every release, old and new (a split-aware binary grandfathers an existing `~/.vite-plus`), so a false "pre-split" answer degrades gracefully; a false "split-aware" answer is impossible because only a binary that implements `VP_DUMP_DIRS` can print the roots.

**`vp-setup` specifics.** `vp-setup` resolves its `EnvConfig` at process start, so the fallback happens mid-install: `do_install` swaps to the monolithic mapping after the probe and returns the effective directories for the success summary. Managed Node.js/pnpm used for the wrapper install still resolve from the process-wide `EnvConfig` pinned before the fallback; the abandoned split data root they land in is removed when this run created it. Known limit: the interactive menu shows the split directories before the download, so a pinned pre-split version confirms one location and installs to the legacy root with the notice.

**Coverage.** The `test-install-sh-old-version` (Linux, macOS) and `test-install-ps1-old-version` (Windows) CI jobs install a pinned pre-split release with no `VP_HOME` and assert the legacy layout, the absence of split roots, and working PATH-resolved commands. This mechanism also keeps fresh default installs of `latest` working during the window between merging this RFC and shipping 0.3.0.

### Global CLI → JS children

Under the split layout, the global CLI injects `VP_BIN_DIR` / `VP_DATA_DIR` / `VP_CACHE_DIR` into JS child processes when those vars are unset, so the NAPI / local CLI and JS tools see the same category roots without re-implementing XDG logic.

### User impact (this phase)

| Install state                                       | Behavior                                                                                                                             |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Existing `~/.vite-plus`                             | Paths unchanged (grandfathered until migrate follow-up).                                                                             |
| Custom `VP_HOME`                                    | Still works as a full-root pin.                                                                                                      |
| Fresh install                                       | Split layout; typically only `~/.local/bin` (or Windows bin dir) needs to be on PATH.                                                |
| Pre-split version (pinned, or `latest` until 0.3.0) | Monolithic `~/.vite-plus`, detected via probe (see [Compatibility with pre-split releases](#compatibility-with-pre-split-releases)). |

### Verified scenarios (manual)

1. **Fresh split** — empty home, no `VP_HOME`: install lands on `~/.local/share/vite-plus`, shims in `~/.local/bin`, env under `~/.config/vite-plus`; `vp --version` works.
2. **Monolithic reuse** — pre-seeded `~/.vite-plus` with markers: `install-global-cli` upgrades `current` in place, keeps prior version dirs and markers, does not create split roots; runtime writes `resolve_cache.json` under `~/.vite-plus/cache`; `vp env doctor` reports home `~/.vite-plus`.

## Follow-up: `VP_HOME` cleanup

**Issue:** [#2371](https://github.com/voidzero-dev/vite-plus/issues/2371)

Much of the repo still **sets** or **assumes** `VP_HOME` as the primary install root (especially PTY snapshot tests). That fights the split layout.

**Direction:**

- Prefer `VP_*_DIR` / XDG / platform defaults in tests, CI, and docs.
- Keep **reading** `VP_HOME` in `VpDirs` as a custom-root pin until a later cleanup.
- Snapshot suite should not require a permanent `VP_HOME=~/.vite-plus` baseline for the happy path.

## Follow-up: layout migrate on `vp upgrade`

**Issue:** [#2372](https://github.com/voidzero-dev/vite-plus/issues/2372)

After the split layout ships, stop grandfathering forever: on `vp upgrade` (and installer reinstall where appropriate), migrate a **default** monolithic install into split roots and remove `~/.vite-plus`.

### Mapping (Unix defaults; Windows analogous)

| From under `~/.vite-plus`                                                    | To                                                                              |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Version dirs, `current`, runtimes, package managers, packages, bins metadata | data dir (`~/.local/share/vite-plus`, …)                                        |
| `config.json` (and durable user config)                                      | config dir                                                                      |
| Session state                                                                | state dir                                                                       |
| Shims / env scripts                                                          | **regenerate** into bin + config (do not copy relative links or stale env text) |
| Resolve cache, upgrade-check cache, create-org tarballs                      | cache dir                                                                       |

Custom `VP_HOME` roots are **out of auto-migrate** (they stay a manual full-root pin).

### Locked design constraints

1. **Copy-first**, then delete the monolithic root (no long-lived tombstone unless Windows file locks force a deferred cleanup).
2. **Never delete** the monolithic root before split `data/current` (and critical shims) are verified.
3. **Conflict** if the split data root already holds a healthy unrelated install — abort with a clear message.
4. Shell profiles that source `~/.vite-plus/env*` must be **rewritten or cleaned** to the new config env path.
5. **N-1 path**: users on a pre-migrate CLI may re-exec after upgrade and/or re-run the install script as the guaranteed fallback.
6. **Immediate** removal of the default monolithic root after a successful migrate (product choice: do not leave an empty grandfather forever).

### Acceptance (migrate)

- Machine with only default `~/.vite-plus` runs `vp upgrade` once → split roots populated, `~/.vite-plus` gone, shims/env work after shell restart.
- Fresh install never creates `~/.vite-plus`.
- CI covers monolithic → split upgrade (in addition to released-CLI and fresh-split install paths).

> Experimental migrate work was sketched on a side branch and **withdrawn** from the dirs PR because moving a live global install is high risk (Windows locks, PATH/profile cutover, concurrent shims). Re-land only behind careful staging and tests.

## Testing strategy

| Layer      | What                                                                                                                                                                                    |
| ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Unit       | `vp_shared` resolution / fallthrough / home-ordering cases pin env via `EnvConfig::with_vars`; feature tests use `with_vars` / `scoped` (see [Test configuration](#test-configuration)) |
| Install CI | `test-standalone-install`: released CLI (often `VP_HOME`-pinned for pre-split packages) + local-build fresh split + grandfather / upgrade-adjacent jobs                                 |
| Snapshots  | Layout isolation without assuming a permanent monolithic home (improve further in #2371)                                                                                                |
| Manual     | Fresh split install; existing monolithic reuse with new CLI                                                                                                                             |

## Alternatives considered

1. **Always split; never grandfather** — breaks existing installs until migrate is perfect. Rejected for the first ship.
2. **Always migrate on first run of any command** — surprising and dangerous mid-script. Prefer explicit `vp upgrade` / installer.
3. **Keep monolithic forever; only document XDG as optional** — fails PATH and platform conventions for new users.
4. **Separate install scripts for monolithic vs split** — duplicated drift; replaced by one script with resolution branching.

## Open questions (post-migrate)

1. Timeline for dropping the **read** of `VP_HOME` after most users are on split roots.
2. Windows deferred delete / reboot policy when locked files block monolithic root removal.

## References

- Issue: [#827](https://github.com/voidzero-dev/vite-plus/issues/827)
- Implementation PR: [#2346](https://github.com/voidzero-dev/vite-plus/pull/2346)
- Follow-ups: [#2371](https://github.com/voidzero-dev/vite-plus/issues/2371), [#2372](https://github.com/voidzero-dev/vite-plus/issues/2372)
- Code: `crates/vp_shared/src/dirs.rs`, `crates/vp_shared/src/dirs/resolution.rs`
- Installers: `packages/cli/install.sh`, `packages/cli/install.ps1`, `packages/tools/src/install-global-cli.ts`
- Related RFCs: [upgrade-command](./upgrade-command.md), [implode-command](./implode-command.md), [env-command](./env-command.md), [js-runtime](./js-runtime.md)

# RFC: Split Directory Layout via `VpDirs`

## Status

**Partially implemented.** [PR #2346](https://github.com/voidzero-dev/vite-plus/pull/2346)
adds the split layout for fresh installs and centralizes directory resolution.
It closes [issue #827](https://github.com/voidzero-dev/vite-plus/issues/827).
[Issue #2371](https://github.com/voidzero-dev/vite-plus/issues/2371) tracks
full `VP_HOME` cleanup. [Issue #2372](https://github.com/voidzero-dev/vite-plus/issues/2372)
tracks automatic migration of existing files.

## Background

Vite+ previously stored the complete global install under one monolithic root:

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

This layout is easy to install and document. However, it conflicts with
platform conventions:

1. **XDG and platform roots.** Data, cache, config, and state belong in their
   platform roots. Unix uses `~/.local/share`, `~/.cache`, `~/.config`, and
   `~/.local/state`. Windows uses the applicable Local and Roaming directories.
2. **Executable ownership.** Vite+ manages `vp`, runtime dispatchers, and
   global-package shims. An application-owned bin directory prevents collisions
   with unrelated tools in a shared directory.
3. **Path construction.** Call sites previously joined paths under
   `~/.vite-plus` or read `VP_HOME` directly. This made layout changes difficult
   and increased the risk of errors.
4. **Tests.** Snapshot and CI tests set `VP_HOME` to create one directory tree.
   This makes the fixtures depend on the monolithic layout.

## Goals

1. Store all category roots and first-level data directories in
   `vp_shared::VpDirs`. Call sites must not construct `~/.vite-plus` paths or
   read `XDG_*` directly.
2. Use the split XDG or platform layout for fresh installs.
3. Keep existing default installs in `~/.vite-plus` during this phase. Do not
   move their files.
4. Keep `VP_HOME` as a full-root pin for custom roots and old scripts. Internal
   integrations can set `VP_BIN_DIR`, `VP_DATA_DIR`, and `VP_CACHE_DIR` as one
   complete group when they must pin a split layout.
5. Use the CLI resolution strategy in `install.sh`, `install.ps1`, `vp-setup`,
   and `install-global-cli`.
6. Support both layouts in implode, env setup, trampoline, upgrade check, and
   related operations.

## Non-Goals (this phase)

1. This phase does not move an existing `~/.vite-plus` tree to the split roots.
   [Issue #2372](https://github.com/voidzero-dev/vite-plus/issues/2372) tracks
   this work. See [Follow-up: layout migrate](#follow-up-layout-migrate-on-vp-upgrade).
2. This phase does not remove `VP_HOME` from the resolution chain.
   [Issue #2371](https://github.com/voidzero-dev/vite-plus/issues/2371) tracks
   cleanup of code that sets this variable.
3. This phase does not add a distribution channel or package format.
4. This phase does not change the payload structure in a version directory.
   This structure includes `current`, version directories, and `node_modules`.

## Design

### Ownership: `VpDirs`

`crates/vp_shared/src/dirs.rs` owns only the **category roots**:

| Root     | Contents                                                                                                                                             |
| -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bin`    | Executables and shims; ownership follows the resolved mapping (`<BIN>/vp`, `<BIN>/node`, …)                                                          |
| `data`   | CLI versions, managed runtimes, package managers (`<DATA>/current`, `<DATA>/js_runtime`, `<DATA>/package_manager`, `<DATA>/packages`, `<DATA>/bins`) |
| `cache`  | Disposable caches (`resolve_cache.json`, `.upgrade-check.json`, create-org tarballs)                                                                 |
| `config` | User configuration (`<CONFIG>/env*`, `<CONFIG>/config.json`)                                                                                         |
| `state`  | State files (session version)                                                                                                                        |

#### `<BIN>` ownership invariant

Vite+ owns the default `<BIN>` and may manage all entries in it. Treat a bin
from an explicit `VP_*_DIR` group as potentially shared. Check the ownership
of each entry in a potentially shared directory.

| Resolution source                               | Ownership                                                                        |
| ----------------------------------------------- | -------------------------------------------------------------------------------- |
| Unix platform default or `XDG_DATA_HOME`        | Application-owned `<DATA>/bin`.                                                  |
| Windows platform default                        | Application-owned `%LOCALAPPDATA%\vite-plus\bin`.                                |
| `VP_HOME` or a grandfathered monolithic install | Application-owned `<root>/bin` inside the Vite+ install root.                    |
| Complete user-supplied `VP_*_DIR` group         | Potentially shared, regardless of whether its path resembles a platform default. |

`VpDirs` and `VP_DUMP_DIRS` preserve the layout mode, but not the precise source
of each root. A path string does not prove that a separately resolved `<BIN>`
is application-owned.
A consumer may remove `<BIN>` as part of a Vite+-owned parent root. For example,
removing the Unix default `<DATA>` also removes `<BIN>`. A consumer may also
remove `<BIN>` when it has trusted source information for an application-owned
mapping. Otherwise, it must use the policy for a potentially shared directory.

For a potentially shared `<BIN>`, installers, `vp env setup`, global package shim management, and `vp implode` must follow these rules:

- Never recursively remove `<BIN>`.
- Before replacement or removal, verify a symlink target, trampoline marker, or
  equivalent ownership record. A known filename or global-package metadata does
  not prove ownership.
- Preserve an entry if Vite+ cannot verify its ownership. Treat the entry as a
  conflict unless the user permits replacement. Record ownership after Vite+
  creates the replacement.

`VpDirs` is a **stateful value**. `VpDirs::resolve()` runs the strategy chain
once during construction. It stores the five roots in public fields. Later
changes to the process environment do not change these fields. Child processes
resolve their own roots from their own environment.

The struct also stores whether resolution selected the single-root or split
mode. This value preserves provenance for Windows trampoline sidecars. Feature
code must use the five resolved roots. It must not construct different paths
for each mode. The ownership rule above controls destructive operations on
`<BIN>`.

Each feature constructs the paths that it owns. These paths include first-level
directories under `data`, such as `current` and `js_runtime`. They also include
deeper paths such as `config.json`, `js_runtime/node/<ver>`, and
`resolve_cache.json`. `VpDirs` does not construct these paths.

`EnvConfig` owns the resolved `VpDirs` at `EnvConfig::get().dirs`.
`EnvConfig::from_env()` constructs this value. It resolves the user home once
from `HOME` or `USERPROFILE`. It uses the same platform order as the installers
and falls back to the system base directories.

`EnvConfig` stores the result as an `AbsolutePathBuf` in
`EnvConfig.user_home`. It passes the same value to `VpDirs::resolve(home)`.
Thus, `user_home` and `dirs` cannot use different home directories. Directory
resolution reads only `VP_HOME`, `VP_*_DIR`, and `XDG_*`. It does not read
`HOME` or `USERPROFILE`.

`DirEnvOverrides` reads `VP_HOME`, `VP_BIN_DIR`, `VP_DATA_DIR`, and
`VP_CACHE_DIR`. It records one of these states for each variable:

- unset
- absolute path
- relative path

Runtime resolution uses only absolute paths. `validate_vp_dir_env` uses the
same states to apply the installer policy. The function returns an error for
these conditions:

- `VP_HOME` contains a relative path.
- The split group is incomplete.
- A variable in the complete split group contains a relative path.

Directory resolution has no test-only branches. Tests use the process
environment to run the production resolution chain. See
[Test configuration](#test-configuration).

### Test configuration

The build configuration selects one of two `EnvConfig::get()` behaviors:

- **Release builds** read the process environment on the first call. They cache
  the configuration for the process in a `OnceLock`.
- **Test builds** resolve the configuration on **every** call. This behavior
  applies to `cfg(test)` and to downstream tests that enable `test-utils` in
  `[dev-dependencies]`. Thus, environment scopes apply their values immediately.
  The feature adds `temp-env` and `tempfile` as optional dependencies. Release
  binaries do not include these helpers.

Tests pin the **environment**, not resolved paths. The production resolution
chain then derives the roots. This prevents fixtures from using different
resolution rules. The `test-utils` feature provides four `EnvConfig` functions:

| Helper                          | Environment                                                           | Root                                                                     | Use                                                                                  |
| ------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------ |
| `with_vars(vars, \|config\| …)` | declared vars pinned (`None` values unset); everything else inherited | caller-chosen (e.g. `VP_HOME` → own tempdir, or a per-crate shared root) | Tests asserting on pinned values or concrete paths                                   |
| `with_vars_async`               | same, held across `.await`                                            | same                                                                     | Async tests (requires a current-thread runtime — `temp_env`'s lock guard is `!Send`) |
| `scoped(\|config\| …)`          | `VP_HOME` → fresh `tempfile` root                                     | hidden, deleted on scope exit                                            | Dir read/write tests that don't care where the root lives                            |
| `scoped_async`                  | same, across `.await`                                                 | same                                                                     | Async equivalent                                                                     |

Semantics:

- The callback receives the resolved `Arc<EnvConfig>`. The caller does not need
  to call `EnvConfig::get()`. An async helper resolves the configuration inside
  the scoped future after it sets the variables.
- A test can pin any process variable. There is no allowlist. The helper
  inherits undeclared variables without changes.
- Values implement the `EnvValue` trait. A string or path sets the variable.
  `Some` sets an optional value, and `None` unsets it. Unsetting is the only
  inactive state for presence-checked variables such as `CI` and
  `VP_ENV_USE_EVAL_ENABLE`.
- Concrete string and path types implement `EnvValue`. The implementation does
  not use `ToString`. A path can contain non-UTF-8 data, and a lossy conversion
  can damage it.
- The helpers call `temp_env`, which holds a process-wide lock for the complete
  scope. Tests that use these scopes do not need `#[serial]`. A nested scope
  replaces values from the outer scope until the nested scope ends.
- Download tests use one shared `VP_HOME` for each test binary. This root is
  under `std::env::temp_dir()`. It keeps download caches available between tests
  and test runs. Locks protect concurrent installs in the root.

The `temp_env` lock is independent of the `serial_test` lock. In a test binary
that uses these scopes, every environment change must use `temp_env` or these
helpers. A direct `set_var` or `remove_var` call can change a variable during
another scope. This can damage the pinned state of another test. A test can use
only `#[serial]` if no scope pins the variable, such as `VP_SHIM_TOOL`.

### Comment convention

Code comments and documentation use **category placeholders** for on-disk
locations. Use `<BIN>/xxx`, `<DATA>/xxx`, `<CACHE>/xxx`, `<CONFIG>/xxx`, or
`<STATE>/xxx`. Do not add a separate path for each layout at a call site. For
example, do not write `monolithic: ~/.vite-plus/xxx, split:
~/.local/share/vite-plus/xxx`. [Category mapping](#category-mapping) defines the
concrete paths in one place.

### Resolution chain

Each category checks the following sources in order. A source can provide a path
or provide no value. The first path wins.

`~/.vite-plus` is the only source that checks file-system state. It provides a
path only when the directory contains a `current` link. Each global install
creates this link. The check runs once during resolution and does not follow the
link. The installers use the same check.

Directory existence alone is not sufficient. A local pre-split Vite+ dependency
can create `~/.vite-plus` for caches, config, and managed runtimes. This source
has a higher priority than `VP_*_DIR`. Without the `current` check, this stray
directory could select the monolithic layout for an existing split install. A
later upgrade or reinstall could then change roots without a notice. The split
`PATH` entries would continue to run the old binary.

**Unix:**

```text
VP_HOME
  → existing ~/.vite-plus
  → complete VP_BIN_DIR / VP_DATA_DIR / VP_CACHE_DIR group
  → XDG_DATA_HOME / XDG_CACHE_HOME / XDG_CONFIG_HOME / XDG_STATE_HOME
  → platform defaults
```

**Windows:** Windows uses the same first three sources but does not use XDG
variables. After `VP_*_DIR`, resolution uses `%LOCALAPPDATA%` and `%APPDATA%`.
A restricted service or CI environment can prevent the known-folder query.
When this occurs, Vite+ uses `AppData\Local` and `AppData\Roaming` under the
resolved user home. Thus, a known home always produces a complete layout.

| Source                                            | Runtime behavior                                                                                    |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| **`VP_HOME`**                                     | Vite+ puts the **monolithic mapping** for all categories under this root.                           |
| **`~/.vite-plus`**                                | Vite+ uses the monolithic mapping when this directory contains a `current` link.                    |
| **`VP_BIN_DIR` / `VP_DATA_DIR` / `VP_CACHE_DIR`** | All three values must be set to absolute paths. An incomplete or invalid group has no effect.       |
| **`XDG_*`** (Unix)                                | Vite+ uses absolute XDG category roots with the app name `vite-plus`. Bin resolves to `<DATA>/bin`. |
| **Platform defaults**                             | [Category mapping](#category-mapping) defines the Unix and Windows defaults.                        |

`VpDirs` ignores relative `VP_*` and `XDG_*` values during runtime resolution.
This behavior follows the XDG Base Directory Specification. Installers apply
stricter rules before they create installation roots.

### Category mapping

| Category   | Split default (Unix)           | Split default (Windows)          | Monolithic (`VP_HOME` / existing `~/.vite-plus`) |
| ---------- | ------------------------------ | -------------------------------- | ------------------------------------------------ |
| **bin**    | `~/.local/share/vite-plus/bin` | `%LOCALAPPDATA%\vite-plus\bin`   | `<root>/bin`                                     |
| **data**   | `~/.local/share/vite-plus`     | `%LOCALAPPDATA%\vite-plus\data`  | `<root>`                                         |
| **cache**  | `~/.cache/vite-plus`           | `%LOCALAPPDATA%\vite-plus\cache` | `<root>/cache`                                   |
| **config** | `~/.config/vite-plus`          | `%APPDATA%\vite-plus`            | `<root>`                                         |
| **state**  | `~/.local/state/vite-plus`     | `%LOCALAPPDATA%\vite-plus\state` | `<root>`                                         |

In both layouts, **data** contains version directories, `current`, `js_runtime`,
`package_manager`, `packages`, and `bins`. The Unix split default also puts the
executable category in `<DATA>/bin`.

#### XDG category semantics

On Unix, Vite+ follows the XDG Base Directory Specification for user data,
configuration, cache, and state. It resolves these categories from
`XDG_DATA_HOME`, `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, and `XDG_STATE_HOME`.

The specification identifies
[`$HOME/.local/bin`](https://specifications.freedesktop.org/basedir-spec/latest/)
as the directory for user-specific executables. Vite+ does not use this shared
directory for its default `<BIN>`. XDG does not require an executable directory
under the data root. Vite+ derives its application-owned Unix bin from the
resolved data root:

```text
<BIN> = <DATA>/bin
```

The installers and `vp env setup` add this directory to `PATH`. It contains
`vp`, runtime dispatchers, and package dispatchers that Vite+ manages. Examples
include `node`, `npm`, `npx`, and global-package commands. This directory lets
Vite+ create, update, and remove these entries. Vite+ does not own entries in
the shared `~/.local/bin` directory.

The relationship to `<DATA>` is the required rule. A custom `XDG_DATA_HOME`
moves both categories and keeps the same layout.

#### Windows `<CONFIG>` portability

Windows maps `<CONFIG>` to the roaming application-data directory at
[`%APPDATA%\vite-plus`](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid#folderid_roamingappdata).
It maps `<DATA>`, `<CACHE>`, and `<STATE>` under
[`%LOCALAPPDATA%`](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid#folderid_localappdata).
Files in `<CONFIG>`, including `config.json`, must contain portable user
preferences. Store machine-specific paths, downloaded payloads, caches, and
session state in the local categories.

The generated `<CONFIG>/env*` files add the resolved `<BIN>` to `PATH`. They do
not export the internal `VP_BIN_DIR`, `VP_DATA_DIR`, and `VP_CACHE_DIR` group.
Each process resolves the split layout from its current environment. The files
keep an explicit `VP_HOME` only when they must preserve a custom monolithic
root. Features must not store machine identity or durable state in these files.

### Installers

`install.sh`, `install.ps1`, and the local `install-global-cli` use the CLI
resolution chain:

1. If `VP_HOME` contains an absolute path, use that root for the
   **monolithic** layout.
2. Otherwise, check the default `~/.vite-plus` directory or its Windows
   equivalent. If it contains a `current` link, keep the monolithic root.
3. Otherwise, use a complete `VP_*_DIR` group, `XDG_*`, or platform defaults
   for the **split** layout.

The script installers reject an incomplete or relative `VP_*_DIR` group.
`vp-setup` rejects the same groups. It also rejects a relative `VP_HOME`.
`vp-setup` calls `vp_shared::validate_vp_dir_env` before `EnvConfig` resolves
paths. If validation fails, `vp-setup` returns status 1. It does not create a
requested or default installation root.

Each platform has one install script. There is no separate script for each
layout. Local bootstrap does not set `VP_HOME`. It resolves the install data
directory through the same chain.

The installers write environment scripts under **config**. The split layout
uses `~/.config/vite-plus/env*`, and the monolithic layout uses the install root.
`PATH` entries point to the resolved **bin** directory.

External installers and integrations must get resolved paths from the Vite+
binary through `VP_DUMP_DIRS`. They must not construct `<BIN>` from `$HOME`, XDG
variables, or platform rules.

#### Node-manager shim ownership

Node-manager shims follow the
[`<BIN>` ownership invariant](#bin-ownership-invariant). Vite+ owns the default
bin. A bin from an explicit `VP_*_DIR` group can be shared, so replacement and
cleanup code must check each entry. On Unix, `<BIN>/node` must be a symlink to the active `vp`
binary. On Windows, an applicable `node.shim` file must point to the resolved
data root. `install.ps1`, `vp-setup`, and Unix-like shells use this check for
`<BIN>/node.exe`.

A foreign Node entry prevents automatic enablement. This rule applies in CI,
development containers, and the fallback for a missing system Node.js. The installer may
replace conflicting `node`, `npm`, `npx`, and `corepack` entries after explicit
permission. Set `VP_NODE_MANAGER=yes` or accept the interactive prompt to give
permission. Without permission, the installer keeps the foreign entry.
`VP_NODE_MANAGER=no` prevents replacement. During a reinstall, Vite+ updates a
shim only after the ownership check identifies it as a Vite+ shim.

Windows sidecar files record ownership and tell the trampoline which layout to
preserve. The versioned sidecar records the layout mode, data root, and cache
root. A split trampoline sets `VP_DATA_DIR`, `VP_BIN_DIR`, and `VP_CACHE_DIR`
for its child. A single-root trampoline sets `VP_HOME`. It does not infer the
mode from path equality because an explicit split layout can also set `<BIN>`
to `<DATA>/bin`.

Vite+ writes a sidecar immediately after it copies a trampoline executable.
`vp env setup --env-only` does not write sidecars. Setup without `--refresh`
does not add a sidecar to a skipped executable. `vp env setup --refresh`
replaces the executable and then records ownership of the new trampoline.
The trampoline and ownership readers require the `vite-plus-shim-v1` header.
They reject unversioned sidecars.

On Windows, `vp implode` cannot delete the trampoline that started the current
command. For a separate `<BIN>`, it renames the owned executable and its
sidecar to unique paths. A detached process deletes only these renamed paths
after the trampoline exits. The original names are free for an immediate
reinstall, and the cleanup process cannot delete the new files. Vite+ does not
remove unrelated entries or the complete `<BIN>` directory.

### Compatibility with pre-split releases

The installers accept each published `VP_VERSION`. Before version 0.3.0 is
available, `latest` can select a pre-split release. A pre-split binary resolves
all paths from `VP_HOME`, which defaults to `~/.vite-plus`. Its environment
setup, shims, trampoline, and `vp upgrade` expect this monolithic root. If the
installer puts this binary in split roots, the install is not functional.
However, the installer still exits with status 0:

- The PATH trampoline points at `<bin>/../current`, which does not exist.
- Shell startup sources an env script from a config dir the binary never writes.
- The binary's own env setup builds a second, incomplete `~/.vite-plus` tree.

**Detection.** `install.sh` and `install.ps1` download the platform payload
before they select the final layout. Each script then runs the payload binary
once with `VP_DUMP_DIRS=1`. The scripts validate the split override group, but
they do not resolve `VP_*_DIR`, XDG variables, platform defaults, or legacy
installs:

- A current split-aware binary prints the layout mode and one tab-separated line
  for each category root. The categories are `bin`, `data`, `cache`, `config`,
  and `state`. The installer uses these values without changes. Thus, the
  installer and the binary cannot resolve different layouts.
- An earlier split-aware preview can omit the layout mode. For this output, the
  installer selects single-root mode only if all five roots match the
  single-root mapping. Otherwise, it selects split mode.
- A pre-split binary does not recognize the variable. It prints help and exits
  with status 0 without the category lines. The installer then uses the
  **monolithic root**. It uses `VP_HOME` when set and `~/.vite-plus` otherwise.
  The installer prints a notice. The binary then uses the same root, so the
  installed `vp`, `vpr`, `vpx`, `node`, and `npm` commands work.

**Stray legacy trees.** A project can contain a local pre-split Vite+ dependency.
This dependency can create `~/.vite-plus` on a machine with a split global
install. It writes caches, config, and managed runtimes there. The legacy check
requires a `current` link, not only a directory. Thus, a stray tree does not
change an existing split install. `vp upgrade` and reinstalls keep the split
roots.

The `test-install-sh-layout` CI job tests this case. It creates a split install
and a stray `~/.vite-plus` tree. It then checks that resolution and a reinstall
remain split.

**Probe failure.** A payload can fail on the wrong platform. It can also fail
if Windows does not have the required VC++ runtime. The script installer then
selects the monolithic root. The dependency-install step reports the applicable
error as before.

The monolithic root works for old and new releases. A split-aware binary keeps
an existing `~/.vite-plus` install. Thus, an incorrect pre-split result still
produces a compatible layout. An incorrect split-aware result is not possible.
Only a binary that implements `VP_DUMP_DIRS` can print the roots.

**`vp-setup` behavior.** `vp-setup` supports Vite+ 0.3.0 and later. This
includes 0.3.0 prereleases. It also supports internal preview versions that use
the `0.0.0-commit.<sha>` format. The installer resolves the target version
first. It rejects an older version before it downloads the platform payload or
creates an installation root. It uses the directories from `EnvConfig`. It does
not probe the payload or use a monolithic fallback.

**`vp upgrade`.** On a split install, `vp upgrade` rejects a target earlier than
0.3.0 before the download. It tells the user to run `vp upgrade` to install the
latest version. Preview builds with the `0.0.0-commit.<sha>` format are allowed
because they track the current branch. Monolithic installs accept every release.
Thus, CI upgrade tests can keep their old targets.

The upgrade path checks the version instead of probing the payload. The running
binary already supports the split layout. Therefore, a fixed release number can
define the boundary.

**Coverage.** The `test-install-sh-old-version` jobs run on Linux and macOS. The
`test-install-ps1-old-version` job runs on Windows. These jobs install a pinned
pre-split release without `VP_HOME`. They check the monolithic layout, the
absence of split roots, and commands that run through `PATH`.

The `test-vp-setup-exe` job checks these invalid configurations:

- Each incomplete split-variable combination.
- A relative `VP_HOME`.
- A complete split group that contains relative paths.

The job checks that validation creates no requested or default roots. It also
checks that `vp-setup` rejects version `0.2.9` without creating an installation
root. The successful test installs a local `0.0.0-commit.<sha>` preview package.

`install.sh` and `install.ps1` use their fallback to install `latest` before
Vite+ 0.3.0 is available.

### Global CLI → JS children

JavaScript child processes inherit the same `VP_HOME`, complete `VP_*_DIR`
group, XDG variables, and platform environment as the global CLI. The NAPI CLI
resolves these inputs through `VpDirs`; it does not implement the directory
rules again. Generated env files do not create a `VP_*_DIR` override group.
Windows trampolines are the exception: a split sidecar passes the complete
group to its child because the trampoline must preserve its recorded layout.

### User impact (this phase)

| Install state                                       | Behavior                                                                                                |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Existing `~/.vite-plus`                             | Vite+ keeps the paths until the migration follow-up.                                                    |
| Custom `VP_HOME`                                    | Vite+ continues to use this value as a full-root pin.                                                   |
| Fresh install                                       | Vite+ uses the split layout and adds its bin directory to `PATH`.                                       |
| Pre-split version (pinned, or `latest` until 0.3.0) | Vite+ uses monolithic `~/.vite-plus`. The installer detects this requirement through the payload probe. |

### Verified scenarios (manual)

1. **Fresh split.** Start with an empty home and no `VP_HOME`. The installer
   uses `~/.local/share/vite-plus`. It puts shims in the `bin` subdirectory and
   environment files under `~/.config/vite-plus`. The `vp --version` command
   works.
2. **Monolithic reuse.** Start with a marked `~/.vite-plus` install.
   `install-global-cli` updates `current` in place. It keeps old version
   directories and markers, and it does not create split roots. The runtime
   writes `resolve_cache.json` under `~/.vite-plus/cache`. `vp env doctor`
   reports `~/.vite-plus` as the home.

## Follow-up: `VP_HOME` cleanup

**Issue:** [#2371](https://github.com/voidzero-dev/vite-plus/issues/2371)

Many files still set or expect `VP_HOME` as the primary install root. PTY
snapshot tests use it frequently. This behavior conflicts with the split layout.

**Direction:**

- Prefer `VP_*_DIR` / XDG / platform defaults in tests, CI, and docs.
- Continue to read `VP_HOME` in `VpDirs` as a custom-root pin until a later
  cleanup.
- Do not require a permanent `VP_HOME=~/.vite-plus` value for normal snapshot
  tests.

## Follow-up: layout migrate on `vp upgrade`

**Issue:** [#2372](https://github.com/voidzero-dev/vite-plus/issues/2372)

After the split layout ships, stop keeping the default monolithic layout. During
`vp upgrade`, move a default monolithic install to the split roots. Do the same
during a reinstall when applicable. Then remove `~/.vite-plus`.

### Mapping (Unix defaults; Windows analogous)

| From under `~/.vite-plus`                                                    | To                                                                              |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Version dirs, `current`, runtimes, package managers, packages, bins metadata | data dir (`~/.local/share/vite-plus`, …)                                        |
| `config.json` (and durable user config)                                      | config dir                                                                      |
| Session state                                                                | state dir                                                                       |
| Shims / env scripts                                                          | **regenerate** into bin + config (do not copy relative links or stale env text) |
| Resolve cache, upgrade-check cache, create-org tarballs                      | cache dir                                                                       |

Do not migrate custom `VP_HOME` roots automatically. They remain a manual
full-root pin.

### Locked design constraints

1. Copy the files before you delete the monolithic root. Keep a temporary
   marker only if Windows file locks delay cleanup.
2. Do not delete the monolithic root before you verify split `data/current` and
   the critical shims.
3. Stop with a clear conflict message if the split data root contains a healthy,
   unrelated install.
4. Update or remove shell-profile entries that source `~/.vite-plus/env*`. Use
   the new config environment path.
5. Support users who run the release before the migration release. After the
   upgrade, the CLI can start itself again or the user can run the installer.
   The installer is the required fallback.
6. Remove the default monolithic root immediately after a successful migration.
   Do not keep an empty legacy root.

### Acceptance (migrate)

- A machine with only the default `~/.vite-plus` runs `vp upgrade` once. The
  command populates the split roots and removes `~/.vite-plus`. Shims and
  environment setup work after a shell restart.
- Fresh install never creates `~/.vite-plus`.
- CI tests the monolithic-to-split upgrade. It also tests the released CLI and
  fresh split installs.

> A side branch contained an experimental migration. This PR does not include
> that work because moving a live global install has high risk. The risks include
> Windows locks, changes to `PATH` and profiles, and concurrent shims. Add the
> migration only with controlled stages and tests.

## Testing strategy

| Layer      | What                                                                                                                                                                                    |
| ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Unit       | `vp_shared` resolution / fallthrough / home-ordering cases pin env via `EnvConfig::with_vars`; feature tests use `with_vars` / `scoped` (see [Test configuration](#test-configuration)) |
| Install CI | `test-standalone-install`: released CLI (often `VP_HOME`-pinned for pre-split packages) + local-build fresh split + grandfather / upgrade-adjacent jobs                                 |
| Snapshots  | Layout isolation without assuming a permanent monolithic home (improve further in #2371)                                                                                                |
| Manual     | Fresh split install; existing monolithic reuse with new CLI                                                                                                                             |

## Alternatives considered

1. **Always use the split layout.** This breaks existing installs until the
   migration is reliable. Therefore, the first release does not use this option.
2. **Migrate during the first run of any command.** This can change files during
   a script without warning. Use an explicit `vp upgrade` or installer instead.
3. **Keep the monolithic layout.** Optional XDG documentation does not meet
   `PATH` and platform conventions for new users.
4. **Use separate install scripts for each layout.** Separate scripts can become
   inconsistent. One script with resolution branches prevents this problem.
5. **Use the shared `~/.local/bin` on Unix.** This can avoid one `PATH` change.
   However, Vite+ must then check ownership and collisions for each runtime and
   package shim. Vite+ already manages `PATH`, so an application-owned data bin
   is simpler and safer.

## Open questions (post-migrate)

1. When can Vite+ stop reading `VP_HOME` after most users use split roots?
2. Which delayed deletion or restart policy must Windows use when locked files prevent root removal?

## References

- Issue: [#827](https://github.com/voidzero-dev/vite-plus/issues/827)
- Implementation PR: [#2346](https://github.com/voidzero-dev/vite-plus/pull/2346)
- Follow-ups: [#2371](https://github.com/voidzero-dev/vite-plus/issues/2371), [#2372](https://github.com/voidzero-dev/vite-plus/issues/2372)
- Code: `crates/vp_shared/src/dirs.rs`, `crates/vp_shared/src/dirs/resolution.rs`
- Installers: `packages/cli/install.sh`, `packages/cli/install.ps1`, `packages/tools/src/install-global-cli.ts`
- Related RFCs: [upgrade-command](./upgrade-command.md), [implode-command](./implode-command.md), [env-command](./env-command.md), [js-runtime](./js-runtime.md)

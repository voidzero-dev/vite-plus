# RFC: `devEngines` Support for Runtime and Package Manager Selection

## Summary

Make `package.json#devEngines` a first-class source for both Node.js runtime selection and package manager selection in Vite+, following the [OpenJS devEngines field proposal](https://github.com/openjs-foundation/package-metadata-interoperability-working-group/blob/main/devengines-field-proposal.md), with a **compatibility-first** rule:

- **Existing projects**: the current source of truth keeps winning for writes. An existing `.node-version` keeps being updated by `vp env pin`; an existing top-level `packageManager` keeps being updated by package-manager pinning (Corepack compatibility).
- **New projects**: `devEngines.runtime` and `devEngines.packageManager` become the recommended and default manifest.
- **Conflicts**: conflicting declarations are surfaced by `vp env doctor` (semver-aware), never silently resolved.

This RFC implements the plan agreed in [#864](https://github.com/voidzero-dev/vite-plus/issues/864), incorporating review notes from that thread (semver-aware conflict detection, range preservation for `devEngines.packageManager`, handling of all field shapes defined by the spec).

## Motivation

`devEngines` is the cross-tool standard for declaring development environment requirements, already supported by npm (v10.9+), pnpm (`devEngines.runtime` for Node management), and Corepack. Vite+ currently:

- Reads `devEngines.runtime` for Node resolution, but at the lowest project-file priority and without honoring `onFail`.
- Ignores `devEngines.packageManager` entirely (TODO at `crates/vite_install/src/package_manager.rs:288`). Worse, in a project that intentionally uses `devEngines.packageManager` plus a lockfile, today's auto-pin writes a redundant top-level `packageManager` field into `package.json`, fighting the user's chosen manifest.
- Only ever writes `.node-version` (`vp env pin`) and `packageManager` (auto-pin, `vp create`, `vp migrate`), so users standardizing on `devEngines` get no write-path support.

Community feedback in #864 asks Vite+ to treat `devEngines` as the standard going forward while not breaking existing `.node-version` / `packageManager` workflows.

## Background

### The devEngines specification

Per the [OpenJS proposal](https://github.com/openjs-foundation/package-metadata-interoperability-working-group/blob/main/devengines-field-proposal.md):

```typescript
interface DevEngines {
  os?: DevEngineDependency | DevEngineDependency[];
  cpu?: DevEngineDependency | DevEngineDependency[];
  libc?: DevEngineDependency | DevEngineDependency[];
  runtime?: DevEngineDependency | DevEngineDependency[];
  packageManager?: DevEngineDependency | DevEngineDependency[];
}

interface DevEngineDependency {
  name: string; // required
  version?: string; // semver range, same syntax as engines.node; absent = any
  onFail?: 'ignore' | 'warn' | 'error' | 'download'; // default: error
}
```

Spec semantics that matter for this RFC:

- `version` is **optional**; absent means any version satisfies.
- `version` uses **semver range syntax** (like `engines.node`). LTS aliases (`lts/*`, `lts/iron`) are not valid values.
- `onFail` defaults to `error`. In **array form**, the first acceptable option is used; prior elements default to `ignore` and only the final element defaults to `error`.
- Each sub-field accepts a single object or an array of objects.

### Current Vite+ behavior

**Node.js resolution chain** (`crates/vite_global_cli/src/commands/env/config.rs`, `crates/vite_js_runtime/src/runtime.rs`):

1. `VITE_PLUS_NODE_VERSION` env var (session)
2. `~/.vite-plus/.session-node-version` (session)
3. `.node-version` (walk up)
4. `package.json#engines.node` (walk up)
5. `package.json#devEngines.runtime[name="node"]` (walk up)
6. User default (`~/.vite-plus/config.json`)
7. Latest LTS

**Package manager detection chain** (`crates/vite_install/src/package_manager.rs`; [rfcs/package-manager-detection.md](./package-manager-detection.md) has been updated alongside this RFC and now documents the new chain):

1. `packageManager` field (exact version, optional hash)
2. Lockfiles (`pnpm-workspace.yaml`, `pnpm-lock.yaml`, `yarn.lock`, ...) at version `latest`
3. Config files (`.pnpmfile.cjs`, `bunfig.toml`, ...) at version `latest`
4. Explicit default / interactive selection

**Write paths today**:

- `vp env pin` writes `.node-version` only (`crates/vite_global_cli/src/commands/env/pin.rs`).
- After downloading a package manager resolved from `latest`, `PackageManagerBuilder::build()` auto-writes the exact version into the `packageManager` field (`set_package_manager_field()`).
- `vp create` / `vp migrate` write `packageManager` when absent (`packages/cli/src/migration/migrator.ts#setPackageManager`).
- `vp migrate` converts `.nvmrc` / Volta pins into `.node-version`.

**Existing parsing** (`crates/vite_shared/src/package_json.rs`): `DevEngines` has only `runtime`; `RuntimeEngine { name, version, on_fail }` where all fields default to empty strings; `on_fail` is parsed but unused.

## Guiding Principle: Compatibility First

> Existing `.node-version` wins for Node.js. Existing `packageManager` wins for package manager. New projects use `devEngines.runtime` and `devEngines.packageManager`. Conflicting declarations are surfaced by `vp env doctor`, not silently resolved.

Every design decision below derives from this rule.

## Detailed Design

### 1. Shared parsing: spec-compliant `DevEngines`

Generalize `crates/vite_shared/src/package_json.rs`:

```rust
/// One devEngines dependency entry (spec: DevEngineDependency).
pub struct DevEngineDependency {
    pub name: Str,                  // required by spec
    pub version: Option<Str>,       // optional; None = any version satisfies
    pub on_fail: Option<OnFail>,    // optional; effective default depends on position
}

pub enum OnFail { Ignore, Warn, Error, Download }

/// Single object or array (spec allows both for every sub-field).
pub enum DevEngineField {
    Single(DevEngineDependency),
    Multiple(Vec<DevEngineDependency>),
}

pub struct DevEngines {
    pub runtime: Option<DevEngineField>,
    pub package_manager: Option<DevEngineField>,
    // os / cpu / libc: not parsed; out of scope (see Non-Goals)
}
```

Parsing rules:

- **Lenient on read**: a malformed entry (missing `name`, unknown `onFail` value, invalid JSON shape for one entry) is skipped with a warning; it never aborts resolution or breaks unrelated commands. This matches the existing `normalize_version` "warn and ignore" behavior.
- **Effective `onFail`**: single object defaults to `error`; in arrays, every element except the last defaults to `ignore`, the last defaults to `error` (spec).
- **`version` absent or empty**: treated as "any version satisfies". For resolution purposes the entry imposes no version constraint.
- **Unknown `onFail` strings**: treated as the positional default, with a warning.

`RuntimeEngineConfig` / `RuntimeEngine` are replaced by (or aliased to) the new generic types so runtime and packageManager share one implementation, as suggested in #864 (handle "the different shapes of each field" once).

### 2. Node.js runtime

#### 2.1 Read priority

Proposed chain (change marked):

1. `VITE_PLUS_NODE_VERSION` env var (session)
2. `.session-node-version` (session)
3. `.node-version` (walk up)
4. **`package.json#devEngines.runtime[name="node"]` (walk up)** (moved above `engines.node`)
5. `package.json#engines.node` (walk up)
6. User default
7. Latest LTS

Rationale for swapping 4 and 5: `engines.node` is a consumer-facing support range (often broad, e.g. `>=18`), while `devEngines.runtime` is by definition the development-environment requirement and is the field npm/pnpm act on for dev tooling. When both exist, the dev-specific field should drive the dev runtime. This was raised in #864 and matches pnpm behavior.

Decision: the reorder lands together with this RFC. Compatibility impact: only projects declaring **both** fields with **disagreeing** resolutions change behavior, and `vp env doctor` flags exactly those projects.

The walk-up algorithm is unchanged: at each directory, sources are checked in the order above before moving to the parent.

#### 2.2 Array form and non-node runtimes

- Entries are evaluated in array order; the first entry with `name == "node"` is used (matches spec "first acceptable option" for the runtimes Vite+ manages).
- Entries for runtimes Vite+ does not manage (`deno`, `bun`, ...) are skipped for resolution. `vp env doctor` lists them as informational notes ("declared runtime `deno` is not managed by Vite+").
- If no `node` entry exists, the chain falls through to `engines.node`.

#### 2.3 `onFail` semantics

> **Status:** As of this PR, `runtime.onFail` is **parsed and preserved but not yet acted on**. Managed mode always resolves and downloads the requested version (equivalent to `onFail: "download"`), and a resolution/download failure surfaces as an error regardless of the declared `onFail`. The matrix below is the intended future behavior, tracked under [Deferred / Future Work](#deferred--future-work).

The intended semantics, once implemented: managed mode already implements the strongest remediation (`download`), so `onFail` mainly matters when remediation is impossible or when system-first mode (`vp env off`) is active:

| `onFail`          | Managed mode (`vp env on`, default)                                       | System-first mode (`vp env off`)                                             |
| ----------------- | ------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| `ignore`          | Entry skipped for resolution; chain continues                             | Entry skipped; system Node used                                              |
| `warn`            | Resolve and download as usual; if that fails, warn and continue the chain | If system Node does not satisfy the range: warn, then use system Node anyway |
| `error` (default) | Resolve and download as usual; if that fails, exit with error             | If system Node does not satisfy the range: exit with error                   |
| `download`        | Resolve and download (Vite+ default behavior)                             | If system Node does not satisfy the range: fall back to managed download     |

Notes:

- "If that fails" covers: the version/range does not exist upstream, or the download fails (network, platform).
- The current default managed-mode UX is unchanged: a plain `{ "name": "node", "version": "^24.0.0" }` behaves exactly like today (resolve and download).

#### 2.4 `vp env pin` write target

`vp env pin <version>` selects its write target in the current directory:

| State of cwd                                                             | Write target                                                          |
| ------------------------------------------------------------------------ | --------------------------------------------------------------------- |
| `.node-version` exists                                                   | Update `.node-version` (unchanged behavior)                           |
| No `.node-version`; `package.json` has a `devEngines.runtime` node entry | Update that entry's `version` (preserve `onFail`, sibling entries)    |
| No `.node-version`; `package.json` exists without a node runtime entry   | Add `devEngines.runtime` node entry with `onFail: "download"`         |
| No `package.json` in cwd                                                 | Create `.node-version` (unchanged behavior; nothing else to write to) |

- `engines.node` is **never** a pin target: it is a consumer-facing constraint, and rewriting it would change the published package contract. More broadly, no Vite+ write path (pin, unpin, auto-pin, create, migrate) ever deletes or modifies an existing `engines.node`; it is always kept unchanged.
- When updating an existing node entry in array form, only that entry's `version` changes; other runtimes and `onFail` values are preserved.
- An explicit `--target` flag overrides the selection: `vp env pin 24 --target node-version` or `--target dev-engines`. The flag always wins: `--target dev-engines` writes `devEngines.runtime` even when `.node-version` exists, with a note that `.node-version` still takes resolution precedence until removed.

Value semantics (matching the implemented `vp env pin` behavior, which resolves
every input to an exact version at pin time; identical for both targets):

| Input             | Written to the target (either `.node-version` or `devEngines.runtime.version`) |
| ----------------- | ------------------------------------------------------------------------------ |
| `24.11.1` (exact) | `24.11.1` (validated against the registry)                                     |
| `24` (partial)    | resolved exact at pin time (e.g. `24.11.1`)                                    |
| `^24.0.0` (range) | resolved exact at pin time                                                     |
| `lts` / `latest`  | resolved exact at pin time                                                     |

Pinning always writes exact versions ("pin" means lock down; this is today's
`.node-version` behavior, kept for both targets). Teams that prefer a range in
`devEngines.runtime` edit `package.json` directly; Vite+ reads and preserves
ranges, it just does not author them through `vp env pin`. The alias-to-semver
conversion table in section 5.2 applies to `vp migrate` (which preserves the
floating intent of `.nvmrc` values), not to pin.

Rule: Vite+ is lenient when **reading** (`devEngines.runtime.version` containing an alias still resolves, with a doctor warning about spec non-compliance) but strict when **writing** (only valid semver ranges or exact versions are ever written to `devEngines`).

When `.node-version` is the write target and a `devEngines.runtime` node entry also exists:

- If the new pinned version **satisfies** the declared range, nothing else changes (the range is still honest).
- If it does **not** satisfy: in an interactive terminal, `vp env pin` prompts to sync (`devEngines.runtime ("^24.0.0") is no longer satisfied. Update it to match? [Y/n]`), consistent with pin's existing overwrite confirmation. In non-interactive environments it warns and points at `vp env doctor`. It never rewrites the other source without confirmation.

#### 2.5 `vp env pin` (show) and `vp env unpin`

- `vp env pin` with no argument reports the active pin and its source, now including `devEngines.runtime` as a possible source (the `VersionSource::DevEnginesRuntime` display string already exists). Inherited pins from parent directories are reported for both sources, checking `.node-version` first and then the `devEngines.runtime` node entry per directory (matching the resolution order).
- `vp env unpin` / `vp env pin --unpin` removes the pin from the same target that `vp env pin` would write: delete `.node-version` if present, otherwise remove the node entry from `devEngines.runtime` (removing the `devEngines.runtime` key entirely if it becomes empty, and `devEngines` if it becomes empty).

### 3. Package manager

#### 3.1 Detection priority

New chain (insertion marked):

1. `packageManager` field (exact version, optional hash; unchanged)
2. **`devEngines.packageManager` (new)**
3. Lockfiles (unchanged)
4. Config files (unchanged)
5. Explicit default / interactive selection (unchanged)

When **both** `packageManager` and `devEngines.packageManager` exist:

- The `packageManager` field drives selection (it is exact and hash-verifiable, and this matches Corepack precedence).
- If the field's name or version does not satisfy the `devEngines.packageManager` constraint, the command prints a one-line warning and `vp env doctor` reports details. The warning notes that this becomes a hard error in a future release (warn-now, error-later transition). npm already errors in this situation, so npm-driven projects get hard enforcement today.

#### 3.2 Resolving a `devEngines.packageManager` entry

- `name` must be one of `pnpm`, `yarn`, `npm`, `bun`. For other names (the spec leaves the namespace open): in array form, skip to the next entry; if no usable entry remains, apply the effective `onFail` of the last entry (`error` → fail with a clear message; `ignore`/`warn` → continue down the detection chain). `download` for an unsupported manager is an error.
- `version` may be exact, a range, or absent:
  - Exact (`11.5.1`): used directly (same as the `packageManager` field path, minus hash).
  - Range (`^11.0.0`) or absent (= any):
    1. If an already-downloaded version under `$VP_HOME/package_manager/<name>/` satisfies the range, use the highest satisfying one (offline-friendly, no network).
    2. Otherwise resolve the latest satisfying version from the npm registry, fetching the abbreviated metadata document (`Accept: application/vnd.npm.install-v1+json`, KBs instead of the multi-MB full packument) and download it.
  - Once a satisfying version is downloaded, step 1 short-circuits every later resolution, so the registry is only consulted while no satisfying version is cached (no separate TTL cache needed).
  - Prereleases are excluded from range resolution, except when the requirement itself contains a prerelease marker (e.g. `^12.0.0-0`) and no stable version satisfies it.
- `onFail` (current PR): acted on **only when no array entry names a supported package manager** (the bullet above) - `ignore`/`warn` continue down the detection chain, `error`/`download` fail. Once a supported entry is selected, its `onFail` is **not yet** consulted: a later unresolved range or download/install failure surfaces as an error rather than falling back to the next entry. Per-entry fallback (try each supported entry in order, applying its effective `onFail` on failure) is tracked under [Deferred / Future Work](#deferred--future-work).

#### 3.3 Auto-pin behavior changes

Today: whenever the detected version was `latest` (lockfile/config/interactive detection), Vite+ writes the exact downloaded version into the `packageManager` field.

Proposed:

| Detection source                  | Auto-write behavior                                                                                                                                          |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `packageManager` field            | No write needed (already exact); unchanged                                                                                                                   |
| `devEngines.packageManager` exact | No write needed                                                                                                                                              |
| `devEngines.packageManager` range | **No write.** The range is the user's chosen source of truth; freezing it into `packageManager` would create a second, conflicting source (#864 review note) |
| Lockfile / config / interactive   | Write the exact resolved version to **`devEngines.packageManager`** with `onFail: "download"` (new default target), instead of the `packageManager` field    |

The last row is the "new projects default to devEngines" rule applied to the auto-pin path. Projects that already have a `packageManager` field never hit this row, so Corepack-pinned repos keep their current behavior. Decision: the auto-pin value is **exact** (preserving today's determinism guarantee); teams that prefer a range can edit the field afterwards, and Vite+ preserves it (range sources are never frozen).

Auto-written shape:

```json
{
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "11.5.1",
      "onFail": "download"
    }
  }
}
```

Auto-pin never replaces entries it did not write: when `devEngines.packageManager` already declares entries Vite+ does not act on (e.g. another package manager with `onFail: "ignore"` whose detection fell through to a lockfile), the resolved entry is appended to an existing array, and an existing single entry is converted to array form with the original entry kept first. A single entry is only written when the field is absent or malformed.

#### 3.4 Surfacing the source

- `vp env --current --json` gains `"source": "devEngines.packageManager"` as a possible value in the `package_manager` block (alongside the existing `"packageManager"`).
- `vp env which pnpm` and friends report the resolution source the same way they do for Node.

### 4. `vp env doctor` conflict detection

All checks are **semver-aware**: an exact version satisfying a declared range is not a conflict (`.node-version: 24.11.1` is compatible with `devEngines.runtime.version: ^24.0.0`; #864 review note).

Which package.json each check examines mirrors the consumer it diagnoses:

- **Runtime checks** use nearest-first walk-up semantics, like Node.js resolution. The `.node-version` vs `devEngines.runtime` conflict check follows the resolution walk on both sides: it fires only when a `.node-version` actually wins resolution, and the `devEngines.runtime` declaration is found in ancestor manifests too (a parent `.node-version` shadowed by a nearer winning `devEngines.runtime` is not a conflict).
- **Package-manager checks** examine the **workspace root** package.json: that is the file `vp install` reads for `packageManager` / `devEngines.packageManager`, which in a monorepo can be a different (higher) file than the nearest package.json.

New checks:

| Check                                                                       | Severity | Example message                                                                        |
| --------------------------------------------------------------------------- | -------- | -------------------------------------------------------------------------------------- |
| `.node-version` does not satisfy `devEngines.runtime[node].version`         | warn     | `.node-version (22.13.0) does not satisfy devEngines.runtime "^24.0.0"`                |
| Resolved Node version does not satisfy `engines.node`                       | warn     | (extends the existing `check_version_compatibility` warning)                           |
| `packageManager` name differs from `devEngines.packageManager` name         | warn     | `packageManager is "npm@11.4.0" but devEngines.packageManager requires "pnpm"`         |
| `packageManager` version does not satisfy `devEngines.packageManager` range | warn     | `packageManager pnpm@10.9.0 does not satisfy devEngines.packageManager "^11.0.0"`      |
| `devEngines.runtime.version` is not a valid semver range (e.g. `lts/*`)     | warn     | `devEngines.runtime.version "lts/*" is not a valid semver range (see devEngines spec)` |
| Malformed `devEngines` entry (missing `name`, unknown `onFail`)             | warn     | `devEngines.packageManager entry is missing "name" and was ignored`                    |
| Runtime entries Vite+ does not manage (`deno`, ...)                         | info     | `devEngines.runtime declares "deno", which is not managed by Vite+`                    |
| Unsupported `devEngines.packageManager` name                                | warn     | `devEngines.packageManager "vlt" is not supported (supported: pnpm, yarn, npm, bun)`   |

Doctor never auto-fixes; it explains which source wins under the precedence rules and what to change.

### 5. `vp create` and `vp migrate`

#### 5.1 `vp create` (new projects)

- Templates (`packages/cli/templates/*/package.json`) gain a `devEngines` block; `setPackageManager()` writes `devEngines.packageManager` (instead of the `packageManager` field) when the project declares neither:

```json
{
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "^24.0.0",
      "onFail": "download"
    },
    "packageManager": {
      "name": "pnpm",
      "version": "11.5.1",
      "onFail": "download"
    }
  }
}
```

- Decision (revised in review): templates keep their existing `engines.node` entry (`>=22.12.0`) **unchanged** and add the `devEngines` block alongside it. `engines.node` stays the broadly-understood consumer-facing floor (CI images, Renovate, Netlify, pnpm enforcement); `devEngines.runtime` carries the dev requirement (e.g. the current LTS major, `^24.0.0`, as shown above). Doctor guards against drift between the two.
- Creating inside an existing workspace keeps honoring the workspace's existing source of truth (unchanged precedence from [rfcs/package-manager-detection.md](./package-manager-detection.md)).

#### 5.2 `vp migrate` (existing projects)

Same precedence rule as `vp env pin`:

- Project already has `.node-version`: keep it (today's behavior).
- Project has `packageManager`: keep updating that field (today's behavior).
- `.nvmrc` / Volta migration targets `devEngines.runtime`; the prompt names the destination ("Migrate .nvmrc to devEngines.runtime?"). Valid semver values transfer verbatim; alias values are **converted to semver at migration time** (decision from review):

| Source value (`.nvmrc`)     | Written to `devEngines.runtime.version`     | Note                                                                 |
| --------------------------- | ------------------------------------------- | -------------------------------------------------------------------- |
| `20.18.0` / `v20.18.0`      | `20.18.0`                                   | Exact, verbatim (`v` prefix stripped)                                |
| `20` / `20.18` / `^20.0.0`  | verbatim                                    | Already valid semver ranges                                          |
| `lts/iron` (codename)       | `^20.0.0` (the codename's major line)       | Same release line, faithful conversion                               |
| `lts/*`                     | `^<current LTS major>.0.0` (e.g. `^24.0.0`) | Loses float to future LTS lines; migration output notes this         |
| `lts/-1`, `lts/-2` (offset) | `^<resolved major>.0.0`                     | Offset resolved at migration time                                    |
| `latest` / `node`           | exact version resolved at migration time    | No semver equivalent of "always newest"; migration output notes this |

- Volta's `volta.node` is always exact, so it migrates verbatim.

### 6. JSON editing fidelity

Writes to `package.json` must be surgical:

- Preserve key order (serde_json `preserve_order` is already enabled for `set_package_manager_field`).
- Detect and preserve the file's existing indentation (2 spaces, 4 spaces, tabs) and trailing-newline style instead of unconditionally `to_string_pretty`.
- When adding `devEngines`, place it adjacent to `engines` when present, otherwise append at the end.
- The TypeScript side reuses the existing `editJsonFile` helper.

A small shared Rust helper (in `vite_shared`) will own "edit one field in package.json, preserving formatting", used by pin, auto-pin, and unpin.

## Spec Compliance Matrix

| Spec feature                                           | Vite+ support after this RFC                                                                                                                                                               |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `runtime` (single + array)                             | Yes, for `name: "node"`; other runtimes surfaced by doctor, not managed                                                                                                                    |
| `packageManager` (single + array)                      | Yes, for pnpm / yarn / npm / bun                                                                                                                                                           |
| `version` optional, semver range syntax                | Yes (lenient read, strict write)                                                                                                                                                           |
| `onFail` (`ignore` / `warn` / `error` / `download`)    | Partial: drives the unsupported-name fallthrough for `packageManager`; otherwise parsed/preserved, not yet acted on (see section 2.3 and [Deferred / Future Work](#deferred--future-work)) |
| Array `onFail` defaults (prior `ignore`, last `error`) | Yes                                                                                                                                                                                        |
| `os` / `cpu` / `libc`                                  | Out of scope (possible future doctor checks)                                                                                                                                               |
| Integrity hash                                         | Not part of the spec; the `packageManager` field hash remains supported                                                                                                                    |

## Non-Goals

- Managing non-Node runtimes (`deno`, `bun` as a runtime) via `devEngines.runtime`.
- Validating `devEngines.os` / `cpu` / `libc`.
- Acting as a general enforcement layer for arbitrary package manager names beyond pnpm / yarn / npm / bun.
- Changing session-override behavior (`vp env use`, `VITE_PLUS_NODE_VERSION`).

## Deferred / Future Work

`onFail` is parsed, preserved, and validated (`vp env doctor` flags unknown values), but the full behavioral matrix is **not yet implemented** in this PR. What is and isn't acted on today:

| Field            | `onFail` behavior today                                                                                                                                                                                                                   |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `runtime`        | Not acted on. Managed mode always resolves and downloads (equivalent to `download`); failures error out regardless of `onFail`. The section 2.3 matrix is future work.                                                                    |
| `packageManager` | Acted on only when no array entry names a supported package manager: `ignore`/`warn` continue the detection chain, `error`/`download` fail. A selected (supported) entry's `onFail` is not consulted on a later resolve/download failure. |

The deferred behavior, in priority order:

1. **Per-entry `packageManager` fallback.** Try each supported array entry in order; when an entry's range cannot be resolved or its version fails to download/install, apply that entry's effective `onFail` (`ignore`/`warn` advance to the next entry or fall through the detection chain; `error` fails). A supported entry is always tried before its `onFail` is consulted (i.e. `onFail` never skips an entry pre-emptively).
2. **Runtime `onFail` matrix** (section 2.3): differentiate `ignore`/`warn`/`error`/`download` in managed and system-first modes for `devEngines.runtime`, in shim dispatch and `vp env use`.

Both are intentionally separated from this PR: the per-entry fallback threads `onFail` through the async download path, and the runtime matrix touches version resolution and the shim dispatch hot path. Until they land, the doc surfaces above describe only the implemented subset.

## Compatibility Impact Summary

| Scenario                                                               | Before                                                    | After                                                        |
| ---------------------------------------------------------------------- | --------------------------------------------------------- | ------------------------------------------------------------ |
| Project with `.node-version`                                           | `.node-version` wins; pin updates it                      | Unchanged                                                    |
| Project with `packageManager` field                                    | Field wins; no auto-write                                 | Unchanged (plus consistency warning if devEngines conflicts) |
| Project with `devEngines.packageManager` + lockfile                    | devEngines ignored; auto-pin **injects** `packageManager` | devEngines drives selection; no injected field               |
| Project with lockfile only (neither field)                             | Auto-pin writes `packageManager` exact                    | Auto-pin writes `devEngines.packageManager` exact            |
| Project with both `engines.node` and `devEngines.runtime`, disagreeing | `engines.node` wins                                       | `devEngines.runtime` wins; doctor warns                      |
| `vp env pin` in a dir with `package.json`, no `.node-version`          | Creates `.node-version`                                   | Writes `devEngines.runtime`                                  |
| `vp migrate` of `.nvmrc` / Volta pins                                  | Creates `.node-version`                                   | Writes `devEngines.runtime` (aliases converted to semver)    |

## Implementation Plan

### Phase 1: Shared parsing and JSON editing

1. Generalize `crates/vite_shared/src/package_json.rs` to the spec-compliant `DevEngineDependency` / `DevEngineField` / `OnFail` types; add `package_manager` to `DevEngines`; lenient-parse rules; effective-`onFail` computation; unit tests for every spec shape (single, array, missing version, missing onFail, malformed entries).
2. Add the formatting-preserving package.json edit helper to `vite_shared`.

### Phase 2: Package manager detection

1. Insert `devEngines.packageManager` into `get_package_manager_type_and_version()` (replacing the TODO at `crates/vite_install/src/package_manager.rs:288`); name validation; array handling; `onFail` handling.
2. Range resolution against downloaded versions, with registry fallback via the npm abbreviated metadata document.
3. Suppress auto-write when the source is `devEngines.packageManager`; retarget auto-pin to `devEngines.packageManager` when neither field exists.
4. Consistency warning when `packageManager` and `devEngines.packageManager` disagree (warn-now, error-later transition messaging).
5. Expose the new source through the NAPI binding and `vp env --current --json`.

### Phase 3: `vp env` commands

1. `vp env pin` target selection, `--target` flag, value rules, sync prompt (TTY) / warning (non-interactive); `vp env unpin` symmetric removal.
2. Runtime read-priority reorder.
3. ~~`onFail` matrix in shim dispatch and `vp env use` / system-first paths.~~ Deferred: runtime `onFail` is parsed but not yet acted on (see [Deferred / Future Work](#deferred--future-work)).
4. All new `vp env doctor` checks.

### Phase 4: create / migrate

1. Template `devEngines` blocks (alongside the existing `engines.node`, which stays unchanged); retarget `setPackageManager()`.
2. `.nvmrc` / Volta migration to `devEngines.runtime`, including the alias-to-semver conversion table.

### Phase 5: Documentation and tests

1. Update `docs/guide/env.md`, `docs/guide/install.md`, `docs/config/*` as applicable.
2. ~~Update [rfcs/package-manager-detection.md](./package-manager-detection.md) (move `devEngines.packageManager` from Future Enhancements into the algorithm) and [rfcs/env-command.md](./env-command.md) (resolution chain).~~ Done alongside this RFC, together with [rfcs/js-runtime.md](./js-runtime.md) and [rfcs/migration-command.md](./migration-command.md).
3. Snap tests (local and global) covering: pin into devEngines, pin with existing `.node-version`, unpin from devEngines, install with `devEngines.packageManager` (exact, range, array, unsupported name, conflict with `packageManager` field), doctor conflict output, create/migrate output.
4. Rust unit tests alongside the existing suites in `package_manager.rs` and `package_json.rs`.

Phases 1 to 3 are the core; 4 and 5 can land in follow-up PRs.

## Resolved Questions

Decisions from RFC review (2026-06-04):

| #   | Question                                                          | Decision                                                                                                                                 |
| --- | ----------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `devEngines.runtime` vs `engines.node` read priority              | Move `devEngines.runtime` above `engines.node`, landing with this RFC; doctor flags the projects where behavior changes                  |
| 2   | Auto-pin target and value when neither field exists               | Write `devEngines.packageManager` with the exact resolved version and `onFail: "download"`                                               |
| 3   | Pin when both `.node-version` and `devEngines.runtime` exist      | Update `.node-version`; if the devEngines range is broken, prompt to sync in interactive terminals, warn in non-interactive environments |
| 4   | Pin override flag                                                 | `--target node-version` / `--target dev-engines`; an explicit flag always wins, even when the other source exists                        |
| 5   | Unsupported `devEngines.packageManager` names                     | `onFail`-driven: `ignore`/`warn` continue down the detection chain; `error` (the default) and `download` fail with a clear message       |
| 6   | Template `engines.node`                                           | Revised: existing `engines.node` is never deleted or modified anywhere; templates keep it unchanged and add `devEngines` alongside it    |
| 7   | Migration target for `.nvmrc` / Volta                             | `devEngines.runtime`; alias values are converted to semver at migration time (see the conversion table in section 5.2)                   |
| 8   | `packageManager` vs `devEngines.packageManager` conflict severity | Warn now with a notice that it becomes an error in a future release, then flip to hard error                                             |

## References

- Issue: [voidzero-dev/vite-plus#864](https://github.com/voidzero-dev/vite-plus/issues/864) (plan: [comment](https://github.com/voidzero-dev/vite-plus/issues/864#issuecomment-4582332165), review notes from @TheAlexLichter)
- Spec: [OpenJS devEngines field proposal](https://github.com/openjs-foundation/package-metadata-interoperability-working-group/blob/main/devengines-field-proposal.md), [discussion #15](https://github.com/openjs-foundation/package-metadata-interoperability-working-group/issues/15)
- npm: [package.json devEngines docs](https://docs.npmjs.com/cli/v11/configuring-npm/package-json#devengines)
- pnpm: [devEngines.runtime support](https://github.com/pnpm/pnpm/issues/8153)
- Related RFCs: [package-manager-detection.md](./package-manager-detection.md), [env-command.md](./env-command.md), [js-runtime.md](./js-runtime.md)

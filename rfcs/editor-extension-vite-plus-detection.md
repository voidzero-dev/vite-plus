# RFC: Vite+ Project Detection for Editor Extensions

> Tracking issue: [#1557](https://github.com/voidzero-dev/vite-plus/issues/1557)
> Status: **Draft for discussion** — not yet a final design.

## Summary

Define a single, portable rule that the four oxc editor extensions —
`oxc-vscode`, `oxc-zed`, `oxc-intellij-plugin`, `coc-oxc` — can use to
answer: _"Given this workspace folder, is it part of a Vite+ project?"_
The rule decides whether the extension should launch `vp lint --lsp` /
`vp fmt --lsp` (instead of plain `oxlint` / `oxfmt`) and which executable
path to spawn.

**The rule, in one sentence:**
A workspace is a **Vite+ project** iff the `vp` binary can be located
using the same resolution logic the extension already uses to find
`oxlint` / `oxfmt`. If `vp` is not resolvable, fall back to a
declarative check of
`package.json#{dependencies,devDependencies}.vite-plus`.

## Motivation

Issue #1557 deprecates the per-package `bin/oxlint` and `bin/oxfmt`
wrappers that `vite-plus` ships today
(`packages/cli/bin/oxlint`, `packages/cli/bin/oxfmt`). Editor extensions
currently lean on those wrappers — the package manager installs them
into `node_modules/.bin/`, so the same `findBinary("oxlint")` code path
that works for a plain oxlint project automatically picks up the
`vite.config.ts`-aware wrapper for a Vite+ project. Once the wrappers
go away, that implicit handoff breaks: each extension must explicitly
notice "this is a Vite+ project" and launch `vp lint --lsp` /
`vp fmt --lsp` instead.

Without a shared rule, each extension reinvents it. Today the four
extensions have four different stories:

- `oxc-zed` (`src/lsp.rs:28`) loops over `[package_name, "vite-plus"]`
  in `package.json` deps and, on match, points at
  `node_modules/vite-plus/bin/oxlint` (the wrapper that #1557
  deprecates).
- `oxc-intellij-plugin` has a dedicated
  `viteplus/VitePlusPackage.kt` that resolves `vite-plus` via
  IntelliJ's Node package descriptor and returns
  `<vite-plus>/bin/oxlint`.
- `oxc-vscode` (`client/findBinary.ts:96, 208`) has comments
  acknowledging the Vite+ case but no explicit detection; it relies on
  `node_modules/.bin/oxlint` being the wrapper bin.
- `coc-oxc` (`src/common.ts:30`) has no Vite+ awareness at all.

## Insight

Each extension **already has a battle-tested function for resolving a
Node CLI binary in a workspace** — that's how `findBinary("oxlint")`
works today. If we point the same function at `"vp"`, the answer to
"is this a Vite+ project?" falls out for free, **and the call site gets
the resolved `vp` binary path it needed anyway** to launch
`vp lint --lsp`.

This avoids inventing a new "vite-plus marker" concept. The `vp` binary,
which `vite-plus` publishes via its `package.json#bin.vp` field, is the
canonical marker.

## How each extension resolves a CLI today

The four extensions all converge on roughly the same pattern, with
different fallbacks.

### `oxc-vscode` — `client/findBinary.ts`

```
1. settingsBinary (user-configured `oxc.<tool>.binPath`)
   → searchSettingsBin()
2. node_modules/.bin/<name> in every workspace folder
   → searchProjectNodeModulesBin() → searchNodeModulesDefaultBinPath()
3. node_modules/.bin/<name> from every nested package.json found in the workspace (monorepo)
4. require.resolve(<name>) anchored at workspace folders, then walk up to package.json#bin
   → replaceTargetFromMainToBin()
5. Yarn PnP: load `.pnp.cjs` / `.pnp.js`, call `resolveRequest(<name>, …)`
   → findPnpApi(), searchYarnPnpBin()
6. Global node_modules from `npm root -g`, `pnpm root -g`, `~/.bun/install/global/node_modules`
   → searchGlobalNodeModulesBin()
7. $PATH
   → searchEnvPath()
```

The whole chain returns a `BinarySearchResult` with `{path, loader, yarnPnpLoaderPath?}`.

### `coc-oxc` — `src/common.ts:23`

```ts
function findBinary(config: ClientConfig): Optional<string> {
  const cfg = workspace.getConfiguration(`oxc.${config.name}`);
  let bin = cfg.get<string>('binPath', '');
  if (bin && existsSync(bin)) return bin;
  bin = join(workspace.root, 'node_modules', '.bin', config.name);
  return existsSync(bin) ? bin : null;
}
```

User setting → workspace `node_modules/.bin/<name>`. That's it.

### `oxc-zed` — `src/lsp.rs`

```rust
fn get_workspace_exe_path(&self, worktree: &Worktree) -> Result<Option<PathBuf>> {
    let package_json = worktree.read_text_file("package.json")
        .unwrap_or(String::from(r#"{}"#));
    let package_json: Option<Value> = from_str(&package_json).ok();
    let package_name = self.get_package_name();           // "oxlint" or "oxfmt"
    let workspace_root = Path::new(worktree.root_path().as_str());

    for package_dir in [package_name.as_str(), "vite-plus"] {
        if package_json.as_ref().is_some_and(|p| package_exists(p, package_dir)) {
            return self.get_exe_path_from(workspace_root, package_dir, package_name.as_str()).map(Some);
        }
    }
    Ok(None)
}
```

Zed reads `package.json` at the worktree root (Zed's WASM API cannot
list arbitrary `node_modules` contents — see zed#10760), checks deps
for `oxlint`/`oxfmt` first then falls back to `vite-plus`, and
constructs `node_modules/<package_dir>/bin/<exe>`. Crucially Zed
_avoids_ `node_modules/.bin` because pnpm stores shell-script shims
there (see `lsp.rs:47`).

### `oxc-intellij-plugin` — `viteplus/VitePlusPackage.kt`

```kotlin
fun getPackage(virtualFile: VirtualFile?): NodePackage? {
    // NodePackageDescriptor("vite-plus").listAvailable(...)
    // or .findUnambiguousDependencyPackage(project)
    // or NodePackage.findDefaultPackage(...)
}
fun findOxlintExecutable(virtualFile: VirtualFile): String? {
    val pkg = getPackage(virtualFile) ?: return null
    val path = pkg.getAbsolutePackagePathToRequire(project) ?: return null
    return Paths.get(path, "bin/oxlint").toString()
}
```

IntelliJ already has a dedicated `VitePlusPackage` class that locates
the `vite-plus` package via the IDE's Node descriptor and returns
`<vite-plus>/bin/oxlint` or `<vite-plus>/bin/oxfmt`. This is the
strongest existing precedent for the "vp binary as marker" model.

### Common shape

Despite the different surface areas, every extension's resolution chain
includes one or more of:

- a **user-configured override** path (highest priority);
- a **workspace `node_modules` lookup** for the target package;
- an optional **`require.resolve` / IDE-package-descriptor** fallback;
- (some) **PnP / global / `$PATH`** fallbacks.

What we standardize is **what target name** they look up, not _how_
they look it up.

## The canonical rule

```
fn detect_vite_plus_project(workspace_root: AbsolutePath) -> Option<DetectResult>:
    # Signal #1: locate the `vp` binary.
    # Each extension plugs in its own existing bin-resolution chain,
    # parameterized by the target name "vp" instead of "oxlint"/"oxfmt".
    if let Some(vp) = find_binary("vp", workspace_root):
        return Some({
            root: workspace_root,
            vp_path: vp.path,
            reason: "vp-binary-found",
        })

    # Signal #2: declarative fallback for pre-install / git-fresh clones,
    # Yarn PnP without `node_modules`, and CI before `pnpm install`.
    if walk_up_finds_vite_plus_in_deps(workspace_root, &mut root_out):
        return Some({
            root: root_out,
            vp_path: None,
            reason: "declared-in-package-json",
        })

    return None
```

Where:

- `find_binary("vp", root)` is **the project-scoped subset** of the
  extension's existing `findBinary` chain — only entries anchored to
  the workspace count:
  - workspace `node_modules/vite-plus/bin/vp` (walk-up to the workspace
    root);
  - workspace `node_modules/.bin/vp`;
  - `require.resolve("vite-plus")` anchored at a workspace folder;
  - Yarn PnP API loaded from `.pnp.cjs` / `.pnp.js` inside the
    workspace.

  Explicitly excluded from Signal #1, even when the extension's
  general chain consults them: **user-configured override paths**,
  **global `node_modules`**, and **`$PATH`**. A user with `vp` on
  `$PATH` or a globally installed `vite-plus` does not turn an
  unrelated workspace into a Vite+ project. The same exclusion applies
  to user settings like `oxc.<tool>.binPath` — those settings target
  `oxlint`/`oxfmt`, not `vp`, and reusing them with `"vp"` as the
  target is meaningless.

- Additionally, the install must be a real `vite-plus` package: at the
  ancestor where `bin/vp` is found, `node_modules/vite-plus/package.json`
  must parse and have `name === "vite-plus"`. This rules out orphan
  files left behind by a partial uninstall or by hand-crafted
  directories.

- `walk_up_finds_vite_plus_in_deps` walks from `root` up to (and
  including) the nearest workspace root (`pnpm-workspace.yaml`,
  `package.json#workspaces`, or `lerna.json`), checking each
  `package.json` for `vite-plus` in `dependencies` or
  `devDependencies`. **The walk stops at the workspace root** — it
  does not cross into the parent of the workspace, even when no
  ancestor declares `vite-plus`. A workspace whose grandparent
  directory happens to have a `vite-plus` install is not itself a
  Vite+ project.

### Why this rule

- **The `vp` binary is the strongest evidence Vite+ is actually present
  and runnable.** No `vite.config.ts`, no `vite-task.json`, no
  hand-maintained marker file required.
- **Every extension already has the lookup code.** Zero new
  infrastructure in any of the four — they call their existing function
  with a different argument.
- **It survives pnpm's shell-shim layout, npm hoisting, Yarn PnP,
  monorepos, and global installs**, because each extension's
  resolution chain was already designed for `oxlint`/`oxfmt` and
  inherits the same robustness.
- **The fallback handles pre-install state** — `package.json` is the
  source of truth before `node_modules` exists. This matters for CI
  workflows that lint before `pnpm install`.

### What we deliberately do **not** check

- `vite.config.ts` / `vite-task.json` — exist in plain-Vite projects.
- `.oxlintrc.json` / `.oxfmtrc.json` — exist in plain-oxlint projects.
- `node_modules/.bin/oxlint` being the wrapper bin — #1557 deletes those.
- A globally-installed `vp` on `$PATH`, a `vp` in the user's global
  `node_modules`, or a user-configured `oxc.<tool>.binPath`. None of
  these tell us anything about whether _this workspace_ uses Vite+.
- A `node_modules/vite-plus/` directory that doesn't actually contain
  a valid `vite-plus` package (parseable `package.json` with
  `name === "vite-plus"`). Orphan trees from partial uninstalls do not
  count.
- Any ancestor above the workspace root. The walk stops there.

## Reference TypeScript implementation

`oxc-vscode` and `coc-oxc` can copy this directly into their codebase
and adapt it to their existing `findBinary` chains. Pure stdlib, no
runtime dependencies.

```ts
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';

export interface DetectResult {
  /** Absolute path of the workspace ancestor that owns vite-plus. */
  root: string;
  /** Absolute path to the resolved `vp` executable, if Signal #1 fired. */
  vpPath?: string;
  reason: 'vp-binary-found' | 'declared-in-package-json';
}

function isWorkspaceRoot(dir: string): boolean {
  if (existsSync(join(dir, 'pnpm-workspace.yaml'))) return true;
  if (existsSync(join(dir, 'lerna.json'))) return true;
  try {
    const pkg = JSON.parse(readFileSync(join(dir, 'package.json'), 'utf8'));
    if (pkg.workspaces) return true;
  } catch {}
  return false;
}

function readDeps(pkgPath: string): Record<string, string> | null {
  try {
    const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
    return { ...(pkg.dependencies ?? {}), ...(pkg.devDependencies ?? {}) };
  } catch {
    return null;
  }
}

/**
 * Signal #1 acceptance check: `bin/vp` must exist AND
 * `node_modules/vite-plus/package.json` must parse and identify itself
 * as the `vite-plus` package. Rejects orphan directories left behind
 * by partial uninstalls or hand-crafted trees.
 */
function isValidVitePlusInstall(dir: string): string | null {
  const vpPath = join(dir, 'node_modules', 'vite-plus', 'bin', 'vp');
  if (!existsSync(vpPath)) return null;
  try {
    const pkg = JSON.parse(
      readFileSync(join(dir, 'node_modules', 'vite-plus', 'package.json'), 'utf8'),
    );
    if (pkg?.name !== 'vite-plus') return null;
  } catch {
    return null;
  }
  return vpPath;
}

export function detectVitePlusProjectSync(start: string): DetectResult | null {
  // Walk up; check both signals at each ancestor. Stop AT the workspace
  // root — do not cross into its parent.
  let dir = start;
  while (true) {
    // Signal #1: real, validated binary.
    const vpPath = isValidVitePlusInstall(dir);
    if (vpPath) {
      return { root: dir, vpPath, reason: 'vp-binary-found' };
    }
    // Signal #2: declared in package.json.
    const deps = readDeps(join(dir, 'package.json'));
    if (deps && deps['vite-plus']) {
      return { root: dir, reason: 'declared-in-package-json' };
    }
    // Stop AT the workspace root, not after.
    if (isWorkspaceRoot(dir)) return null;
    const parent = dirname(dir);
    if (parent === dir) return null;
    dir = parent;
  }
}
```

The async variant is the same algorithm with `fs.promises` — left as an
exercise for the consumer.

## Per-extension migration plan

Each extension keeps its existing bin-resolution code and adds a thin
"detect Vite+ first" pass on top.

### `oxc-vscode`

Add a Vite+ detection pass that runs **before** the existing
oxlint/oxfmt `findBinary` chain. Detection must reuse only the
**project-scoped** parts of that chain — steps 2 through 5 in the
existing chain (workspace `node_modules/.bin`, monorepo
`node_modules/.bin`, workspace-anchored `require.resolve`, and Yarn
PnP loaded from a workspace `.pnp.cjs`). It must **not** consult
`searchSettingsBin`, `searchGlobalNodeModulesBin`, or `searchEnvPath`
when the target is `"vp"`.

```ts
// On tool startup, for each workspaceFolder:
//   1. Run the project-scoped subset of findBinary with target "vp".
//      Validate node_modules/vite-plus/package.json before accepting.
//   2. If not found, walk up from workspaceFolder to the workspace root
//      checking package.json deps for "vite-plus".
//   3. If either signal fires, launch `<vp> lint --lsp` (or `fmt --lsp`).
//      Otherwise, fall through to the existing oxlint/oxfmt chain.
```

### `coc-oxc`

Add a check before the existing `node_modules/.bin` lookup. Reuse the
reference algorithm directly; **do not** consult `oxc.<tool>.binPath`
when looking for `vp` (that setting targets oxlint/oxfmt).

```ts
// In findBinary(), before the node_modules/.bin lookup:
// 1. From workspace.root, run the reference detector:
//    - check node_modules/vite-plus/bin/vp + validate package.json
//    - walk up to the workspace root checking package.json deps
// 2. If positive, launch vp <subcmd> --lsp.
// 3. Else, fall through to existing logic.
```

### `oxc-zed`

Zed's existing `get_workspace_exe_path` loop already iterates
`[package_name, "vite-plus"]`. Change the `package_dir == "vite-plus"`
branch to return `<root>/node_modules/vite-plus/bin/vp` and invoke it
with `["lint", "--lsp"]` (or `["fmt", "--lsp"]`). Add the package.json
parse check (`name === "vite-plus"`) to reject orphan trees. Zed is
already project-scoped (it only reads the worktree root), so no
exclusion of `$PATH` is needed.

### `oxc-intellij-plugin`

The existing `VitePlusPackage.kt` resolves `vite-plus` through
IntelliJ's `NodePackageDescriptor`, which is naturally project-scoped
(it consults the project's interpreter and dependency graph). Change
`findOxlintExecutable` / `findOxfmtExecutable` to return
`vite-plus/bin/vp` and update the launch args to `lint --lsp` /
`fmt --lsp`. Continue using the IDE's package resolution rather than
walking the filesystem.

## Decisions

### "Find the `vp` binary" is the primary signal

Locked. Replaces the earlier proposal of "stat
`node_modules/vite-plus/package.json`," which was functionally
equivalent but conceptually weaker — the binary's existence is what
actually matters for invocation, and every extension already has the
lookup machinery.

### Hybrid two-signal algorithm

Locked. Signal #1 (`vp` binary) + Signal #2 (declared in package.json).
Rejected alternatives:

- **Signal #1 alone** — wrong answer on fresh clones / CI before install.
- **Signal #2 alone** — slower (always parses JSON) and ignores the
  evidence that Vite+ is actually installed and runnable.
- **A new manifest file** (`vite-plus.json` / `.vite-plus`) — adds a
  hand-maintained marker that can drift from the install state.

### Workspace-wide granularity

If any ancestor up to the workspace root resolves `vp` or declares
`vite-plus`, the entire workspace is Vite+. Editor LSPs operate at
workspace granularity; per-package granularity would surprise users by
toggling LSP behaviour as they move between folders.

### Avoid `node_modules/.bin/vp` in the reference and in Zed

Mirroring oxc-zed's choice (`lsp.rs:47`): point at
`<root>/node_modules/vite-plus/bin/vp`, not `node_modules/.bin/vp`,
because pnpm stores shell-script shims in `.bin` that don't behave
like real Node binaries when invoked headlessly. Extensions whose own
chain (like oxc-vscode) does prefer `.bin` are free to keep it — they
resolve to the same underlying entry.

### Yarn PnP deferred to v2

Berry with PnP has no `node_modules`. Signal #1 in the simple
walk-up fails; PnP users still detect correctly via Signal #2 (deps
check). Explicit `.pnp.cjs` lookup is deferred. **Note:** oxc-vscode
has its own PnP support in `searchYarnPnpBin` — when oxc-vscode calls
the project-scoped subset of `findBinary("vp")` through its own
chain, the workspace-anchored PnP lookup is included.

### Signal #1 is strictly project-scoped

User-configured override paths (`oxc.<tool>.binPath`), global
`node_modules` (`npm root -g`, `pnpm root -g`, bun global), and `$PATH`
are explicitly **excluded** from Signal #1. None of them say anything
about whether _this workspace_ uses Vite+. A globally installed `vp`
or a `vp` shim on `$PATH` does not turn an unrelated workspace into a
Vite+ project.

### Signal #1 requires a valid `vite-plus` package, not just `bin/vp`

At the ancestor where `bin/vp` is found,
`node_modules/vite-plus/package.json` must parse and have
`name === "vite-plus"`. Orphan `bin/vp` files (partial uninstall, hand
crafted directories, stale caches) do not count.

### Walk stops at the workspace root

Once the walk-up evaluates a directory that is itself a workspace root
(`pnpm-workspace.yaml`, `package.json#workspaces`, or `lerna.json`),
the walk terminates. We do not check the parent. Otherwise a nested
checkout placed under a parent that happens to have its own
`vite-plus` install would inherit Vite+ behaviour from a completely
unrelated workspace.

## Downstream coordination

Each extension's own repo owns its PR and its own test fixtures.

- `oxc-vscode` PR: extend the existing `findBinary` chain with `"vp"`
  as a target; route through `vp lint --lsp` / `vp fmt --lsp` when
  found.
- `coc-oxc` PR: add the ~15-line Vite+ check before the existing
  `node_modules/.bin` lookup.
- `oxc-zed` PR: change the `package_dir == "vite-plus"` branch in
  `lsp.rs:28` to target `bin/vp` with `--lsp` args plumbed through
  `language_server_command`.
- `oxc-intellij-plugin` PR: keep `VitePlusPackage.kt`; change
  `findOxlintExecutable` / `findOxfmtExecutable` to return `bin/vp`
  and update the launch args.

## Open questions

1. **Caching policy** in editor extensions — documented best-practice
   only, or also illustrated in the reference snippet (an opt-in
   memoizing variant with a watcher-invalidation hook)?
2. **Zed launch args plumbing.** The `--lsp` switch is already there
   for oxlint/oxfmt; for `vp` we need to pass `["lint", "--lsp"]` /
   `["fmt", "--lsp"]`. The Zed extension API accepts this via
   `Command { command, args, env }` — confirmed in `oxlint.rs:29-34`.
3. **Transitive-install false positives.** Someone could pull
   `vite-plus` in transitively. Signal #1 still fires. Proposal:
   accept it — `vp lint --lsp` degrades to plain oxlint behaviour
   when no `vite.config.ts` is present.
4. **"Installed but not configured."** Should we additionally require
   `vite.config.ts` to exist? Proposal: **no**. Presence of `vp` is
   intent enough.

## Conformance fixtures

Every implementation must produce identical answers on the following
fixtures. Each extension replicates the set inside its own test suite.

| Fixture                         | Tree                                                                                                                                                                                      | Expected `detectVitePlusProject` result                                                                                                                                      |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `pnpm-root-installed`           | `pnpm-workspace.yaml` + root `package.json` + `node_modules/vite-plus/bin/vp` + `node_modules/vite-plus/package.json` + a `packages/app/package.json` subpackage                          | `{ root: "<repo>", vpPath: "<repo>/node_modules/vite-plus/bin/vp", reason: "vp-binary-found" }` regardless of whether detection starts from the root or from `packages/app/` |
| `pnpm-root-declared-no-install` | `pnpm-workspace.yaml` + root `package.json` declaring `vite-plus`, no `node_modules`                                                                                                      | `{ root: "<repo>", reason: "declared-in-package-json" }`                                                                                                                     |
| `npm-package-installed`         | Root `package.json` with `workspaces`, `node_modules/vite-plus/...` inside `packages/app/` (un-hoisted)                                                                                   | Detection from inside `packages/app/` returns `vp-binary-found` rooted at `packages/app`                                                                                     |
| `yarn1-workspaces`              | yarn1-style hoisting, `node_modules/vite-plus/` at root                                                                                                                                   | `vp-binary-found` rooted at the workspace root                                                                                                                               |
| `yarn4-pnp`                     | Berry/PnP, no `node_modules`, `vite-plus` declared in root `package.json`                                                                                                                 | `declared-in-package-json` rooted at the workspace root (Signal #2 fallback)                                                                                                 |
| `plain-non-vite-plus`           | A normal Node project, no Vite+ anywhere                                                                                                                                                  | `null`                                                                                                                                                                       |
| `plain-vite-no-vp`              | Uses Vite but not Vite+ (`vite` in deps, `vite.config.ts` present, no `vite-plus`)                                                                                                        | `null`                                                                                                                                                                       |
| `transitive-install`            | `vite-plus` only present as a transitive dep (in `node_modules` but not declared in any walked-up `package.json`)                                                                         | `vp-binary-found` — documents v1 behaviour; accepted as a false-positive trade                                                                                               |
| `bin-vp-without-package-json`   | `node_modules/vite-plus/bin/vp` exists but `node_modules/vite-plus/package.json` is missing                                                                                               | `null`                                                                                                                                                                       |
| `bin-vp-with-malformed-package` | `bin/vp` + `package.json` exists but `package.json` is unparseable or has `name !== "vite-plus"`                                                                                          | `null`                                                                                                                                                                       |
| `parent-vite-plus-nested-repo`  | Outer dir has `node_modules/vite-plus/...` + declares `vite-plus`; inner subdirectory is its own workspace root (own `pnpm-workspace.yaml`/`package.json#workspaces`) without `vite-plus` | Detection from inside the nested workspace returns `null` — the walk stops at the inner workspace root and does not see the outer install                                    |
| `global-vp-on-path`             | A plain Node project; `vp` is installed globally (on `$PATH` and/or in the user's global `node_modules`); no workspace-local `node_modules/vite-plus`                                     | `null` — Signal #1 ignores `$PATH` and global installs                                                                                                                       |
| `user-binpath-override`         | A plain Node project; `oxc.oxlint.binPath` is configured to point at a `vp` binary; no workspace-local `vite-plus`                                                                        | `null` — Signal #1 ignores user-configured override paths                                                                                                                    |

## Verification plan

1. **Each downstream PR** replicates the fixture table above inside its
   own test suite and asserts the expected detector result.
2. **Manual editor smoke test** before each downstream PR is merged:
   point the extension at a real Vite+ project and at a plain-oxlint
   project; verify correct LSP routing in both.

# RFC: Node-Independent Environment Management and Project Activation

## Status

Draft. The accompanying implementation covers startup separation. Automatic activation is proposed as a separate phase; `vp env auto` is not implemented yet.

## Summary

Separate native `vp env` startup from JavaScript toolchain installation so environment management does not require a Node runtime. Add an opt-in `auto` mode that uses managed tools in Vite+ projects and delegates to system tools elsewhere.

## Motivation

Developers working across legacy and modern projects need to manage Node.js versions without upgrading each application's runtime. Some want to replace fnm or nvm across all projects. Others want to keep their existing manager and use Vite+ environment management only in Vite+ projects.

[Discussion #2607](https://github.com/voidzero-dev/vite-plus/discussions/2607) describes both workflows for projects using Node 12, 16, and 18, and asks whether package-manager management applies outside Vite+ projects. The report does not include a reproduction, so the cause of that specific failure remains unconfirmed.

The native manager already resolves and installs older Node versions. However, first-run setup installs the JavaScript toolchain before command dispatch, which requires a separate modern Node runtime. The existing managed and system-first modes also require users to choose one mode across projects; they do not provide automatic project-based activation.

This RFC covers environment management and manager coexistence. Package-script forwarding and changes to the runtime requirements of the JavaScript toolchain are separate requirements.

## User Stories

The following scenarios cover first-time setup, replacing an existing manager, and using Vite+ alongside fnm or nvm.

| Scenario                                                                                         | Expected result                                                                                                                                                                                                                                   | Implementation scope                                                       |
| ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| Start a standalone `vp` binary with no working Node, then select Node 16                         | Native setup and `vp env install 16.20.2` succeed without installing or executing a modern runtime for Vite+. Management commands remain available if toolchain installation fails.                                                               | Startup separation in this change                                          |
| Replace fnm for a legacy backend with `.nvmrc` or `.node-version`, without migrating the project | `node`, `vp node`, and `vp env exec node` use the declared runtime. Switching directories selects each project's version unless a session override is active. No Vite config or dependency is required.                                           | Existing resolution and shims; validate with legacy runtimes               |
| Keep fnm/nvm while using Vite+ in newer projects                                                 | In opt-in `auto` mode, entering a Vite+ project selects managed tools. Elsewhere, shims delegate to the tools exposed by fnm/nvm or the system. The user does not toggle ownership on each directory change.                                      | New project-based activation policy below                                  |
| Manage npm, pnpm, Yarn, or Bun without migrating a project                                       | Use native version management and respect the selected Node and manager versions. Node and package-manager ownership can differ, including individual manager families. Executing a selected manager retains that manager's runtime requirements. | Existing management surface; extend activation consistently to each family |
| Inspect or repair an unexpected environment                                                      | `vp env current`, `which`, and `doctor` explain which manager owns each tool, why, and which executable will run. Native management remains usable with a missing or unusable Node executable.                                                    | Startup independence plus activation diagnostics                           |
| Use the JavaScript toolchain after native-only setup                                             | Install the global JavaScript payload when required. Installation uses its own runtime without changing the project's Node pin or ownership mode. A failure leaves environment management usable.                                                 | Startup separation in this change                                          |

Legacy-version coverage includes Node 12, 16, and 18 where a supported release binary is available for the host platform. Node 16 is the first end-to-end target for this change.

## Goals

Treat the native environment manager as usable across Node projects. Do not require a project to install `vite-plus`, load `vite.config.ts`, or raise its Node pin to manage its environment.

Keep three runtime requirements distinct: the runtime needed to launch the manager, the runtime selected for the user's project, and the runtime needed by Vite+ JavaScript tools. Native management needs no Node runtime. The application and each package manager retain their own requirements. Deferred toolchain installation can use its own managed runtime without changing the application's selection.

Add project-based activation as an opt-in mode. Preserve explicit `on/off` ownership and existing defaults so an upgrade does not change which manager owns a user's tools.

## Background

The relevant paths before startup separation are:

| Layer                                                                                                                                        | Current dependency                                                                      | Consequence                                                                                                            |
| -------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| [`main.rs`](../crates/vp_global_cli/src/main.rs) and [`self_setup.rs`](../crates/vp_global_cli/src/self_setup.rs)                            | Unmarked standalone startup installs the global JavaScript dependencies before dispatch | A first `vp env` command needs the installer runtime even though the command itself is native                          |
| [`commands/env`](../crates/vp_global_cli/src/commands/env) and [`vp_js_runtime`](../crates/vp_js_runtime/src)                                | Native resolution, downloads, verification, configuration, and shim preparation         | Management has no JavaScript toolchain Node floor; execution and system-tool probes can still launch the selected tool |
| [`cli.rs`](../crates/vp_global_cli/src/cli.rs) and [`vp_pm_cli::dispatch`](../crates/vp_pm_cli/src/dispatch.rs)                              | Project package-manager commands dispatch through Rust and execute the selected manager | Removing startup installation does not solve an incompatible Node/package-manager pair                                 |
| [`env/config.rs`](../crates/vp_global_cli/src/commands/env/config.rs) and [`shim/dispatch.rs`](../crates/vp_global_cli/src/shim/dispatch.rs) | Ownership is currently `managed` or `system_first`, with per-family preferences         | A Vite+ dependency does not currently trigger automatic ownership changes                                              |
| [`js_executor.rs`](../crates/vp_global_cli/src/js_executor.rs)                                                                               | Toolchain commands launch JavaScript with a selected runtime                            | Their runtime requirements remain separate from native environment management                                          |

Implement startup separation and automatic activation as separate layers. The first removes the bootstrap runtime dependency. The second determines which manager owns a tool in the current project.

## Automatic Project Activation

### Command Interface

Add `vp env auto [node|pm|npm|pnpm|yarn|bun]`, with the same selectors as `on` and `off`. Bare `auto` configures Node and all package-manager families; a selector changes only that scope. Store `auto` alongside the existing per-component mode settings.

| Configured mode        | In a Vite+ project                                        | Outside a Vite+ project                                        |
| ---------------------- | --------------------------------------------------------- | -------------------------------------------------------------- |
| `on` / `managed`       | Use managed tools                                         | Use managed tools                                              |
| `off` / `system_first` | Prefer system tools; retain the existing managed fallback | Prefer system tools; retain the existing managed fallback      |
| `auto`                 | Use managed tools                                         | Delegate to system tools; report a missing tool if none exists |

Outside a Vite+ project, `auto` must not silently install a managed runtime when no system tool exists. The error should explain that the user can configure their other manager or select `vp env on`. The existing `off` mode retains its managed fallback.

For a user replacing fnm, `on` remains the appropriate mode: `.nvmrc` and `.node-version` work across projects regardless of Vite+ adoption. For a user keeping fnm, `auto` delegates outside Vite+ projects. The two modes address different scenarios.

Older binaries reject `auto` in the existing mode fields. Before downgrading to such a version, users must run `vp env on` or `vp env off` with the newer binary for each scope configured as `auto`. Phase two must document and test this downgrade procedure; it must not map unknown modes to managed ownership.

### Project Detection

Determine activation from static project metadata in Rust. Do not load JavaScript, execute a Vite config, inspect an installed toolchain, or contact a registry to decide ownership.

Treat a project as Vite+-enabled when its package or applicable workspace root declares `vite-plus` in `dependencies`, `devDependencies`, or `optionalDependencies`. A `peerDependencies` entry alone does not opt a project in. Detection must work before `node_modules` exists.

Use the nearest package and the workspace that contains it. Workspace membership matters: a workspace-root declaration applies to its members, but must not activate an unrelated nested repository or a neighboring project. Stop at a nested repository boundary. Reuse workspace metadata parsing and test pnpm and `package.json` workspaces, including exclusions.

An ordinary Vite dependency, `vite.config.*`, `.node-version`, `.nvmrc`, or `engines.node` does not indicate Vite+ ownership. Those files also occur in projects managed by fnm/nvm. With no applicable package declaration, use the outside-project behavior.

If relevant metadata is unreadable or invalid, report a detection error rather than changing ownership based on a guess. Diagnostics must identify the file. Do not edit project metadata as a side effect of detection.

Run detection only for scopes that need an automatic ownership decision. Broken project metadata must not prevent `env on/off`, an explicit `env install <version>`, or an explicit execution override from repairing that scope.

### Overrides and Shell Integration

Resolve explicit command and session choices before automatic activation. For example, `vp env exec --node 16.20.2 node -v` uses that managed version even outside a Vite+ project. In `auto` mode, `vp env use 16.20.2` temporarily selects managed Node until `vp env use --unset node`; it does not change package-manager ownership. Apply the same rule to family-specific package-manager session overrides. Keep existing `on/off` precedence unchanged.

Within `auto`, use the first applicable source for each tool:

| Source                                               | Effect                                                                               |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------ |
| Explicit execution option, such as `env exec --node` | Start a new selection with the requested managed tool                                |
| Inherited tool selection in a child invocation       | Retain the parent's selection under the existing recursion rules                     |
| Shell environment override                           | Use the selected managed tool in that shell and its children                         |
| Fallback session file                                | Use the selected managed tool for processes sharing the state directory              |
| Project detection                                    | Use managed tools in an applicable Vite+ project; otherwise delegate to system tools |

The shell wrapper sets session overrides through environment variables. Direct invocation without the wrapper can write `<STATE>/.session-node-version` or a family-specific session file; these files affect other shells that share the state directory until unset. Diagnostics must distinguish these sources and identify the file. CI jobs that need independent overrides should use separate state directories.

Evaluate activation when a shim or environment-aware command runs, using the effective working directory after `-C`. Do not rewrite configuration on `cd` or create a session override when entering a project. A fresh tool invocation from the shell after leaving the project restores delegation unless an override is active. A package script that changes directory retains its inherited selection; a direct `vp env exec` starts a new selection. Preserve these existing child-process rules and explain inherited ownership in diagnostics.

Vite+ can only make this decision when the shell invokes its shim. The Vite+ shim directory must precede the external manager's executable directory on `PATH`, and the external tools must remain discoverable after excluding Vite+ shims. Document shell initialization order and test managers that change `PATH` on directory changes. If another manager places its binary before the Vite+ shim, the shell bypasses Vite+; `doctor` should explain that condition.

### Resolution API (Phase Two)

Use a shared, side-effect-free `EnvironmentPlan` to resolve ownership before downloading tools or constructing a child environment. Its inputs are the effective working directory, original `PATH`, configuration, overrides, inherited selections, and applicable project metadata. For each tool, return the configured mode, effective owner, selection source, activation reason, and an external executable or managed requirement to resolve. Inspection and execution consume the same plan; diagnostic version probes remain a separate operation.

Resolve external tools against the original `PATH`, skipping Vite+ shims with the existing external-tool lookup. The current `system_first` lookup stops at the first match and cannot supply this behavior when the Vite+ shim comes first. Preserve its existing behavior for `off`. Resolve Node and package-manager ownership before prepending either tool, so managed Node's bundled npm cannot replace the intended system npm during resolution.

### Diagnostics

Report both configured mode (`auto`) and effective ownership (`managed` or `system`) in `vp env current --json`, along with an activation reason and the declaring package path when applicable. Keep the existing `mode` field for the configured mode and add fields for the effective result. Human-readable `current` and `doctor` should explain the same decision; `which` must report the executable dispatch would use.

Centralize ownership resolution so Node shims, package-manager shims, `vp node`, `vp env exec`, `vp env print`, and package-manager command dispatch agree. A configured Node mode and a configured package-manager mode can differ; a system package-manager launcher must receive the Node selected by the Node policy. Review cleanup protection against the same effective environment.

Automatic activation does not install the global Vite+ JavaScript payload. Managing a selected runtime or manager may download that selected tool. Running a JavaScript toolchain command still follows the deferred-installation design below.

## Startup Dependency

The global CLI implements environment management in Rust. Version resolution, downloads, signature checks, archive extraction, configuration edits, and shim setup do not execute JavaScript. Before this change, the dependency comes before command dispatch: `main()` calls `self_setup::maybe_run()`, which installs the global JavaScript payload when the setup marker is absent. That installation downloads the release's pinned Node runtime and runs pinned pnpm, even for `vp env --help`.

The installer selects its own runtime, so Node.js 16 does not prevent setup. However, environment management still depends on downloading and executing an unrelated modern Node version before the first command can run.

## Proposed Startup Behavior

Separate ordinary native command startup from global JavaScript dependency installation. Complete native setup on first use. Install the JavaScript payload when a command needs it. Explicit installers using `VP_SELF_SETUP_SHELL` retain eager installation, progress reporting, and the selected-runtime output contract; `vp upgrade` also remains eager.

| Operation                                          | Required setup                                                  |
| -------------------------------------------------- | --------------------------------------------------------------- |
| Global help and `vp env` management operations     | Native executable, configuration, and shims                     |
| `vp env exec node` and the `node` shim             | Native setup and the selected runtime                           |
| Package-manager execution                          | Native setup, the selected manager, and its required runtime    |
| Delegation to an installed project-local CLI       | Native setup, the local payload, and the selected runtime       |
| Global JavaScript CLI execution or global fallback | Native setup and the global JavaScript payload                  |
| `vp toolchain --global`                            | Native setup and the global payload containing `toolchain.json` |

For example, a fresh `vp env install 16.20.2` installs Node.js 16.20.2 without provisioning a modern runtime for Vite+. A later global toolchain command installs the Vite+ package and its dependencies on demand.

`env exec` must still launch the requested program. In system-first mode, `env current` and `env doctor` may execute tools with `--version`. These operations inherit the target program's requirements. They do not impose a minimum Node version on management operations.

## Native Setup

Retain first-run deployment, environment-management preferences, shell integration, Windows trampolines, and the handoff to the installed executable. Continue to preserve arguments, the working directory, environment, standard input, and exit status across that handoff.

Preserve a supplied `vp-install-lock.yaml` companion locally during native setup, without recovering it from the registry. Write the wrapper `package.json` for the deployed version during native setup. It records the matching `vite-plus` dependency and pinned installer pnpm version.

Choose the explicit installation registry first, then a saved registry, then the preview-build fallback when applicable. Apply current registry environment overrides on deferred attempts as well. Save the selected registry in that version's `.npmrc`. Update only the registry setting and retain authentication, scoped registries, and other settings. Persist the registry even when a failed attempt has left a payload `package.json`. A later shell can then finish toolchain installation without the original installer's environment variables.

Do not resolve, download, or execute Node or pnpm during ordinary native command setup. Explicit installer setup retains its runtime selection and dependency installation. Windows can still fetch its native trampoline package when the standalone executable has no sibling trampoline.

Keep `.vp-setup-complete` as the native setup marker. Existing installations retain their markers. Keep external-install receipts under the resolved state directory and preserve the read-only bundled-package path described in [Directory Layout](./directory-layout.md).

Record `.vp-owned` when setup creates a new managed version directory. This ownership evidence is independent of JavaScript readiness, so `vp implode` can remove a native-only installation from a shared data root. Never claim a preexisting version directory. Preserve the unowned root, unrelated files, and nested unowned directory categories. The setup-complete marker alone must not authorize deletion.

## Global JavaScript Dependencies

Keep script-path lookup free of installation side effects. Add an asynchronous preparation operation at the points that use the global payload:

- Global CLI delegation, before reading the CLI's runtime metadata.
- Global fallback after project-local resolution fails.
- Global toolchain inspection.

Project-local resolution must not install the global payload when a usable local entrypoint exists. Explicit script-directory overrides remain caller-owned. Existing complete managed payloads with valid installation receipts require no writes. External bundles remain owned by their installer and do not require Vite+ installation receipts. Managed payloads must pass the release lockfile and completion-receipt checks before use; recover a missing companion using the saved installation registry when JavaScript preparation is needed.

Use the existing production-dependency installer for deferred installation. Its default managed runtime is the release's Node.js version from the repository's `.node-version`, and it invokes the pinned pnpm entrypoint by absolute path. Explicit setup can reuse a compatible requested runtime. When upgrade metadata declares an exact runtime, use that version or fail before activating the release. Preserve the project's selected runtime for the command itself; the installer runtime must not become a session override or project pin.

Bootstrap pnpm from the installation directory's npm configuration, including its saved registry and authentication. Do not inherit registry configuration from the invoking project or an ancestor workspace. This must work with an empty pnpm cache in a later shell, as well as with an already cached installer.

Installation status belongs on stderr so JSON and command output remain parseable. Retain the existing captured install log on failure.

### Follow-up: Payload Ownership API

A later refactor should resolve the payload location and owner once: managed release, external bundle, local bootstrap, or explicit scripts override. A shared payload service can then return the prepared package and scripts directories to the executor. Keep ownership separate from readiness; missing files in a caller-owned payload do not authorize a managed release install. Retain the existing disk markers for compatibility. This typed API is a follow-up, not part of the startup implementation.

## Installation State and Concurrency

Track JavaScript readiness separately from native setup and payload ownership. An installed `package.json` alone does not establish readiness: pnpm can write package files before it finishes the dependency graph. Explicit and deferred installation must use the same readiness check.

Use the following state in the version directory:

| State                                                                                               | Meaning                                                                                                      |
| --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `bin/.vp-setup-complete` exists                                                                     | Native setup is complete; it does not establish JavaScript readiness                                         |
| `.vp-local-deps` exists                                                                             | The local bootstrap owns the payload; managed installation must not replace it                               |
| `node_modules/vite-plus/package.json` and `dist/bin.js` exist, with no pending marker               | A candidate payload; managed installations must also pass the release lockfile and completion-receipt checks |
| Candidate payload, matching completion receipt, and working lockfile equal to the release companion | The managed payload is ready; reuse it without writes                                                        |
| Payload missing                                                                                     | Install on first global toolchain use                                                                        |
| `.vp-js-deps-pending` exists                                                                        | An installation started but has not completed; retry before using the payload                                |

For releases that supply an installation lockfile, a missing or invalid receipt requires preparation under the installation lock. Preserve compatibility with older releases that do not supply a companion.

Write the pending marker before installation. Remove it after successful completion and recording the installation receipt. Keep it on failure or interruption, even if package files now exist. A failed toolchain installation must leave native setup and `vp env` usable.

Serialize managed payload writers with a per-version `.vp-js-deps.lock` file. The shared `ProductionDepsTransaction` guard owns the lock for deferred installation, explicit setup, upgrades, and the legacy installer. Native metadata preparation also uses the guard. Acquire the operating-system lock outside the async executor before extracting the payload or changing its companion, wrapper manifest, registry configuration, or readiness state. Recheck readiness after acquiring it, then hold the lock through installation and completion recording. Callers must not bypass the transaction by invoking the dependency installer directly.

Process exit releases the lock; a leftover lock file does not indicate an active installation. Failure handling must keep the lock path stable for waiting installers. Failed upgrade or legacy installation attempts leave an inactive directory available for retry, rather than deleting the directory and its lock beneath those processes. Activate the new version only after preparation succeeds. Release the guard before invoking the installed binary for shim refresh or command handoff, because its native setup can acquire the same lock.

Preserve the saved registry and authentication configuration when applying the installer's `minimum-release-age=0` override. Use preserving updates for both native registry persistence and dependency installation, under the same transaction.

## Backward Compatibility

Existing complete installations continue to use their payloads. Externally managed bundles retain their ownership and read-only behavior. The local development bootstrap continues to provision its checkout dependencies after native setup; its version override remains supported. Native setup records `.vp-local-deps` for this caller-owned payload. Deferred preparation must not replace it with a published release; an incomplete payload asks the developer to finish the local bootstrap.

Keep explicit `vp upgrade` installation eager. An upgrade must prepare its dependencies before replacing the current version, as required by its existing rollback behavior.

The project-local `vite-plus` launcher remains a JavaScript entrypoint with its existing Node requirements. This RFC changes the native global CLI. It does not lower the Node requirements of Vite, Vitest, package managers, or other tools that the user executes.

Older Node releases also need a published binary for the host platform and valid release verification data. This change does not add architecture emulation, source builds, or weaker signature verification. It leaves system-tool diagnostic probes unchanged.

## Testing

Add CLI cases to the [PTY snapshot suite](../crates/vp_cli_snapshots/tests/cli_snapshots/README.md). Start from an unmarked standalone executable with a fresh installation directory; a preinstalled or premarked binary would miss the startup dependency.

For startup separation, cover these cases:

- Native help and environment management with no Node on `PATH`, with an unusable Node executable, and without the dependency-install bypass flag. Assert that no global JavaScript payload or installer runtime appears.
- Selection and execution of Node.js 16, followed by further management commands.
- Deferred global execution, global fallback, and global toolchain inspection.
- Project-local execution without installing the global payload.
- An installer failure after it writes package files, followed by successful native commands and a retry.
- Concurrent first-use commands, including an explicit installer racing a deferred command, sharing one completed installation.
- Explicit and deferred retries after pending state or a missing entrypoint, even when `package.json` exists.
- Saved registry configuration across setup retries, preserving authentication and scoped registries, and clean stdout during installation.
- Cold pnpm downloads from the saved registry in a later shell, current registry overrides, and isolation from project and ancestor workspace configuration.
- Uninstalling native-only versions from owned and shared data roots, without claiming preexisting directories or removing unrelated files.
- Existing setup handoff, retry, external-bundle, and upgrade behavior.

Use controlled installer fixtures for failure and concurrency cases. Supplement them with a real Node.js 16 smoke test on a supported platform.

For automatic activation, cover these cases:

- Keep a simulated fnm/nvm runtime on `PATH`, enter a Vite+ project, run `node`, leave for a legacy project, and run `node` again. Assert the selected executable and version in each directory, with no manual mode changes.
- Repeat through `vp node`, `vp env exec`, direct package-manager shims, and environment inspection. Assert consistent ownership and clean machine-readable output.
- Detect an uninstalled Vite+ project, a workspace member with a root declaration, a member with its own declaration, an excluded directory, a nested repository, and an ordinary Vite project. Do not use `node_modules` as the activation signal.
- Exercise scoped modes, shell overrides, and shared session files across two shells. Unsetting an override restores directory-based activation. Switching Node ownership does not change a package-manager family's configured mode.
- Preserve a child's inherited tool after it changes directory; reevaluate ownership for a fresh invocation or direct `env exec`. Test mixed Node and package-manager modes using the original `PATH`.
- Outside a Vite+ project with no system Node, report a missing tool without installing a managed runtime. Explicit `env install` and `env exec --node` remain available.
- Change or remove the declaring dependency and verify that the next invocation reflects the change. Invalid relevant metadata produces a useful error without a mode change; explicit recovery commands remain usable.
- Test shell initialization and directory hooks that reorder `PATH`; ensure `doctor` identifies a bypassed Vite+ shim. Cover supported shells on Unix and Windows.
- Reset `auto` scopes with the newer binary before running an older binary that cannot read `auto` configuration.

## Implementation Plan

1. Ship native startup separation with deferred JavaScript installation and legacy-runtime checks. This layer is independently useful for users replacing fnm.
2. Implement the opt-in `auto` mode with the shared `EnvironmentPlan`, static project detection, shell diagnostics, and the coexistence tests above. This layer addresses users who keep fnm/nvm.

The typed payload service is an optional follow-up refactor. Measure warm JavaScript startup before changing receipt validation or adding a persistent readiness cache.

## Alternatives Considered

Rewriting the environment subcommands would leave the startup dependency in place. The current Rust implementations already provide the required management behavior.

An early `env` dispatch before setup would avoid installing JavaScript, but would also skip deployment, shims, and external-install receipts. Separating native and JavaScript setup preserves those behaviors without duplicating command parsing for aliases and `-C`.

Keeping eager installation under a separate modern runtime supports old project runtimes, but still prevents environment management when that installer runtime cannot run. Deferred installation removes that requirement from the management path.

A preassembled, verified JavaScript payload archive could remove the installation-time Node and pnpm dependency. It would require changes to release packaging and platform distribution. Reuse deferred pnpm installation with a shared transaction for this change; consider an archive if installation cost or reliability justifies that release work.

For `auto`, shell directory-change hooks could avoid metadata checks on each invocation, but would compete with other managers' `PATH` changes and miss IDE or CI invocations. Resolve ownership at invocation time. An explicit project activation marker would remove ambiguity from dependency detection, but would require repository changes; reserve it for a future override if workspace cases demonstrate the need.

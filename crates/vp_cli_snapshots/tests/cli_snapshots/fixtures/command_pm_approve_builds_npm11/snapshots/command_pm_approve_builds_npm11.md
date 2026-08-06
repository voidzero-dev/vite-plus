# command_pm_approve_builds_npm11

## `vp pm approve-builds --help`

should show help with pnpm/npm deny + --all caveats

```
VITE+ - The Unified Toolchain for the Web

Usage: vp pm approve-builds [OPTIONS] [PACKAGES]... [-- <PASS_THROUGH_ARGS>...]

Approve dependency lifecycle scripts (install/postinstall) to run

Arguments:
  [PACKAGES]...           Packages to approve. Prefix with `!` to deny (pnpm >= 11.0.0, npm >= 11.16.0). Omit to launch interactive mode (pnpm) or list pending packages (npm >= 11.16.0)
  [PASS_THROUGH_ARGS]...  Additional arguments to pass through to the package manager

Options:
  --all       Approve every package currently pending approval (pnpm >= 10.32.0, npm >= 11.16.0). Mutually exclusive with positional packages
  -h, --help  Print help

Documentation: https://viteplus.dev/guide/install
```

## `vp pm approve-builds`

no args -> npm approve-scripts --allow-scripts-pending (lists pending)

```
No packages with unreviewed install scripts.
```

## `vp pm approve-builds esbuild`

-> npm approve-scripts esbuild (advisory note)

**Exit code:** 1

```
note: npm's allowScripts policy is advisory in npm 11.x: install scripts still run; npm only warns about unreviewed packages at install time. Enforcement is planned for a future npm release.
npm error code ENOMATCH
npm error No installed packages match: esbuild
npm error A complete log of this run can be found in: <home>/.npm/_logs/<timestamp>-debug-0.log
```

## `vp pm approve-builds !core-js`

deny-only -> npm deny-scripts core-js (advisory note)

**Exit code:** 1

```
note: npm's allowScripts policy is advisory in npm 11.x: install scripts still run; npm only warns about unreviewed packages at install time. Enforcement is planned for a future npm release.
npm error code ENOMATCH
npm error No installed packages match: core-js
npm error A complete log of this run can be found in: <home>/.npm/_logs/<timestamp>-debug-0.log
```

## `vp pm approve-builds esbuild !core-js`

mixed approve+deny -> rejected, exit non-zero

**Exit code:** 1

```
npm manages approvals and denials separately. Run them as two invocations, e.g. `vp pm approve-builds <approve-pkg>...` then `vp pm approve-builds !<deny-pkg>...`.
```

## `vp pm approve-builds -- esbuild`

positional via -- on the pending path -> rejected, exit non-zero

**Exit code:** 1

```
Pass package names as positionals (`vp pm approve-builds <pkg>...`), not after `--`.
```

## `vp pm approve-builds --all`

-> npm approve-scripts --all (advisory note)

```
note: npm's allowScripts policy is advisory in npm 11.x: install scripts still run; npm only warns about unreviewed packages at install time. Enforcement is planned for a future npm release.
No packages with unreviewed install scripts.
```

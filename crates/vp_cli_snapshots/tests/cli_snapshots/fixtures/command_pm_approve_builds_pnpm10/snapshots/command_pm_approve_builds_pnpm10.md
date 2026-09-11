# command_pm_approve_builds_pnpm10

## `vp pm approve-builds --help`

should show help

```
Approve dependency lifecycle scripts (install/postinstall) to run

Usage: vp pm approve-builds [OPTIONS] [PACKAGES]... [-- <PASS_THROUGH_ARGS>...]

Arguments:
  [PACKAGES]...           Packages to approve. Prefix with `!` to deny (pnpm >= 11.0.0, npm >= 11.16.0). Omit to launch interactive mode (pnpm) or list pending packages (npm >= 11.16.0)
  [PASS_THROUGH_ARGS]...  Additional arguments to pass through to the package manager

Options:
      --all   Approve every package currently pending approval (pnpm >= 10.32.0, npm >= 11.16.0). Mutually exclusive with positional packages
  -h, --help  Print help
```

## `vp pm approve-builds --all`

forwards to pnpm approve-builds --all (nothing to approve)

```
There are no packages awaiting approval
```

## `vp pm approve-builds esbuild fsevents`

forwards positional packages to pnpm

```
There are no packages awaiting approval
```

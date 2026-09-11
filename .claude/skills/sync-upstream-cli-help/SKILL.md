---
name: sync-upstream-cli-help
description: Sync Vite+'s static CLI help with Vite, Vitest, Oxlint, Oxfmt, and tsdown while preserving intentional omissions. Use when an upstream dependency upgrade changes CLI help.
allowed-tools: Read, Grep, Glob, Edit, Bash
---

# Sync upstream CLI help

## Input and target

- Read the diff from `$CLI_HELP_DIFF_REPORT`; first run
  `test -r "$CLI_HELP_DIFF_REPORT"`. Act only when `$CLI_HELP_DIFF_CHANGED` is
  `true`.
- Treat the upgraded tool's `--help` output as the source of truth. The report locates
  changes and versions; rerun the exact version when its diff is truncated.
- Edit `commandHelpDocs` in `packages/cli/src/help.ts`.
- Do not edit `packages/cli/src/utils/help.ts` for content drift. It owns terminal
  wrapping, alignment, and the right margin.

| Upstream help         | Document entry |
| --------------------- | -------------- |
| `vite --help`         | `dev`          |
| `vite build --help`   | `build`        |
| `vite preview --help` | `preview`      |
| `vitest --help`       | `test`         |
| `oxlint --help`       | `lint`         |
| `oxfmt --help`        | `fmt`          |
| `tsdown --help`       | `pack`         |

## Change

- For items Vite+ exposes, copy upstream labels, descriptions, section titles, and
  section guidance exactly. Preserve intentional lines and lists, but not terminal
  padding, automatic wrapping, ANSI color, or version banners.
- Remove an upstream item only after confirming Vite+ no longer supports or
  deliberately retains it.

## Do not change

- Keep Vite+-owned usage, summaries, examples, and documentation URLs.
- Keep config selectors/loaders hidden, including `--config`, `--configLoader`, and
  `--disable-nested-config`.
- Do not add standalone modes such as `--init`, `--migrate`, or `--lsp`, top-level
  `--version`, or options Vite+ does not forward.
- Do not change runtime forwarding or rewrite upstream wording for a Vite+-specific
  runtime default. Use `sync-tsdown-cli` for tsdown runtime changes.

Re-record the affected CLI help snapshots and inspect their diffs. Do not modify help
documents when the report contains no actionable exposed change.

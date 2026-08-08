---
name: sync-tsdown-cli
description: Sync tsdown runtime CLI options with vp pack after a tsdown upgrade. Use for option forwarding changes; use sync-upstream-cli-help for static help wording.
allowed-tools: Read, Grep, Glob, Edit, Bash
---

# Sync tsdown CLI

Runtime options live in `packages/cli/src/pack-bin.ts`; static help lives in
`packages/cli/src/help.ts`.

1. Run `npx tsdown --help` from `packages/cli/` and compare it with `pack-bin.ts`.
2. Add new forwarded options using the existing cac `.option()` pattern. For removed
   options, add `// NOTE: removed from tsdown CLI in vX.Y.Z` for reviewer follow-up.
3. Preserve runtime differences: `-c, --config` stays disabled because Vite+ uses
   `vite.config.ts`, and `--env-prefix` keeps the `['VITE_PACK_', 'TSDOWN_']` default.
4. For static labels and descriptions, follow `sync-upstream-cli-help`; do not adapt
   upstream wording to explain runtime differences.
5. Run `pnpm --filter vite-plus build-ts` and `vp pack -h`. Add a focused PTY snapshot
   case under `crates/vp_cli_snapshots/tests/cli_snapshots/fixtures/` when a new runtime
   option is exposed.

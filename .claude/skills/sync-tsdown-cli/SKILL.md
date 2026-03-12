---
name: sync-tsdown-cli
description: Compare tsdown CLI options with vp pack and sync any new or removed options. Use when tsdown is upgraded or when you need to check for CLI option drift between tsdown and vp pack.
allowed-tools: Read, Grep, Glob, Edit, Bash
---

# Sync tsdown CLI Options with vp pack

Compare the upstream `tsdown` CLI options with `vp pack` (defined in `packages/cli/src/pack-bin.ts`) and sync any differences.

## Steps

1. Run `npx tsdown --help` from `packages/cli/` to get tsdown's current CLI options
2. Read `packages/cli/src/pack-bin.ts` to see vp pack's current options
3. Compare and add any new tsdown options to `pack-bin.ts` using the existing cac `.option()` pattern
4. If tsdown removed options, do NOT remove them from `pack-bin.ts` -- instead add a code comment like `// NOTE: removed from tsdown CLI in vX.Y.Z` above the option so reviewers can decide whether to follow up
5. Preserve intentional differences:
   - `-c, --config` is intentionally commented out (vp pack uses vite.config.ts)
   - `--env-prefix` has a different default (`['VITE_PACK_', 'TSDOWN_']`)
6. Verify with `pnpm --filter vite-plus build-ts` and `vp pack -h`
7. If new parameters were added, add a corresponding snap test under `packages/cli/snap-tests/` to verify the new option works correctly

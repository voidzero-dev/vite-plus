---
name: fix-rolldown-vite-brand
description: Fix brand-rolldown-vite.ts search strings when upstream rolldown-vite code changes after sync
allowed-tools: Bash, Read, Edit, Grep, Glob
---

# Fix rolldown-vite Brand Patches

When `packages/tools/src/brand-rolldown-vite.ts` fails after a rolldown-vite sync, this skill guides you through updating the search strings to match the new upstream code.

## Background

`brand-rolldown-vite.ts` applies string replacements to 4 files in `rolldown-vite/packages/vite/src/node/`:

| File | What it patches |
|------|----------------|
| `constants.ts` | Adds `VITE_PLUS_VERSION` constant after `VERSION` |
| `cli.ts` | Imports `VITE_PLUS_VERSION`, changes banner `'VITE'` → `'VITE+'` |
| `build.ts` | Imports `VITE_PLUS_VERSION`, changes build banner and `[vite]:` error prefix |
| `logger.ts` | Changes default prefix `'[vite]'` → `'[vite+]'` |

When upstream changes the code around these strings, the search no longer matches and the script throws an error like:

```
Error: [brand-rolldown-vite] Patch failed in .../build.ts:
  Could not find search string: "  ROLLUP_HOOKS,\n  VERSION,\n} from './constants'"
  The upstream code may have changed. Please update the search string in brand-rolldown-vite.ts.
```

## Step 1: Identify the Failure

Run the brand script to see which patches fail:

```bash
node --import @oxc-node/core/register packages/tools/src/index.ts brand-rolldown-vite
```

Note which file(s) failed and the search string that was not found.

## Step 2: Find the New Code in Upstream

For each failed patch, read the corresponding upstream source file to find what changed:

- `rolldown-vite/packages/vite/src/node/constants.ts` — look for `VERSION` export
- `rolldown-vite/packages/vite/src/node/cli.ts` — look for `VERSION` import and the `VITE` banner string
- `rolldown-vite/packages/vite/src/node/build.ts` — look for `VERSION` import, the `vite v${VERSION}` banner, and `[vite]:` error prefix
- `rolldown-vite/packages/vite/src/node/logger.ts` — look for `prefix = '[vite]'`

## Step 3: Update the Search Strings

Edit `packages/tools/src/brand-rolldown-vite.ts` to update the `search` argument in each failed `replaceInFile()` call to match the new upstream code.

**Rules:**

1. The search string must be an **exact substring** of the upstream file content (including whitespace, newlines, quotes)
2. The replacement string must produce valid code after substitution
3. The replacement must check for **replacement first, then search** (the function already does this — do NOT change the `replaceInFile` logic)
4. Each `replaceInFile` call has a pair: `(search, replacement)`. Only update the `search` side to match the new upstream. Keep the `replacement` producing the same branded output

**Branding principles (do NOT change these):**

- Banner: `'VITE'` → `'VITE+'`
- Version: `VERSION` → `VITE_PLUS_VERSION` (in user-facing output only)
- Logger prefix: `'[vite]'` → `'[vite+]'`
- Error prefix: `'[vite]:'` → `'[vite+]:'`
- Do NOT change: internal plugin names (`vite:*`), project name references ("Vite requires..."), `VITE_*` env var detection

## Step 4: Verify

Run the brand script again to confirm all patches apply:

```bash
node --import @oxc-node/core/register packages/tools/src/index.ts brand-rolldown-vite
```

Expected output — all checkmarks:

```
[brand-rolldown-vite] Applying Vite+ branding patches...
[brand-rolldown-vite]   ✓ constants.ts: Added VITE_PLUS_VERSION
[brand-rolldown-vite]   ✓ cli.ts: Updated imports and banner
[brand-rolldown-vite]   ✓ build.ts: Updated imports, banner, and error prefix
[brand-rolldown-vite]   ✓ logger.ts: Changed prefix to '[vite+]'
[brand-rolldown-vite] Done!
```

Then verify idempotency (run again — should show "Already patched"):

```bash
node --import @oxc-node/core/register packages/tools/src/index.ts brand-rolldown-vite
```

## Step 5: Update Snap Tests

Rebuild and regenerate snap tests that capture build output:

```bash
pnpm bootstrap-cli
pnpm -F vite-plus snap-test-local build-vite-env
pnpm -F vite-plus snap-test-local synthetic-build-cache-disabled
```

Check diffs to confirm `vite+` branding appears in build output:

```bash
git diff packages/cli/snap-tests/*/snap.txt
```

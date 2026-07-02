---
name: release-manager
description: Run the standard vite-plus release process end-to-end as the release manager. Covers preparing the release PR, syncing the NAPI binding version, writing the categorized changelog, smoke-testing via pkg.pr.new, handling release-branch CI failures, merging, and post-release verification and announcements. Use when asked to cut, prepare, or manage a vite-plus release (e.g. "release v0.2.3", "act as release manager").
allowed-tools: Bash, Read, Edit, Write, Grep, Glob, WebFetch
---

# Vite+ Release Manager

Run a standard vite-plus release from version bump to published announcement. Any maintainer with repo write access can follow this; the only extra privilege needed is approval rights on the `release` GitHub environment (step 7).

## Usage

```
/release-manager                    # start a new release: ask for the target version, begin at step 1
/release-manager X.Y.Z              # start a new release for that version
/release-manager <PR URL or #N>     # take over an in-flight release
```

When given a release PR (URL or number), do not start from step 1. First audit the release's current state, then continue from the earliest unfinished step:

- Is the binding version synced? (step 2: `grep -c "'<prev>'" packages/cli/binding/index.cjs` on the release branch)
- Is the PR description still the `prepare_release` boilerplate, or already a categorized changelog? (step 4)
- Does `main` have commits the release branch lacks? (`git log origin/release/vX.Y.Z..origin/main`, step 5)
- What is CI status? (`gh pr checks <PR#>`, step 5)
- Is a pkg.pr.new build present and for the current head? (step 3)

Report the detected state before making changes, so the previous release manager's work is not redone or overwritten.

## Pipeline overview

1. `Prepare Release` workflow bumps versions and opens the release PR (`release/vX.Y.Z` -> `main`).
2. Release manager: sync `binding/index.cjs`, write the changelog PR description, get CI green, optionally smoke-test via pkg.pr.new.
3. Merging the PR pushes a `packages/cli/package.json` change to `main`, which triggers `release.yml`: build, manual approval gate, npm publish, GitHub release, Docker image, Discord notification.
4. Release manager: polish the GitHub release notes, verify installs, announce.

Canonical sources: `.github/workflows/prepare_release.yml`, `.github/workflows/release.yml`, `.github/workflows/publish-to-pkg.pr.new.yml`.

## 1. Start the release

```bash
gh workflow run prepare_release.yml --repo voidzero-dev/vite-plus -f version=X.Y.Z
```

The workflow bumps `packages/cli/package.json`, `packages/core/package.json`, `packages/cli/binding/Cargo.toml`, and `crates/vite_global_cli/Cargo.toml`, refreshes `Cargo.lock`, and opens a PR titled `release: vX.Y.Z` from branch `release/vX.Y.Z`. The PR body ends with `Merging this PR will trigger the release workflow.` and that line must survive every later edit.

## 2. Sync the NAPI binding version (required every release)

NAPI bakes the package version into version checks in `packages/cli/binding/index.cjs` (26+ sites). `prepare_release` bumps `package.json` but does not regenerate this file, so CI's `Ensure no unexpected file changes after build` step in the `CLI E2E test` job fails until it is synced. Do this immediately; do not wait for CI to fail.

```bash
git fetch origin release/vX.Y.Z && git checkout release/vX.Y.Z
grep -c "'<prev>'" packages/cli/binding/index.cjs          # non-zero means sync needed
grep -c "expected <prev> but got" packages/cli/binding/index.cjs
```

Apply two whole-file text replacements (`<prev>` is the previous release version, e.g. `0.2.1`):

1. `'<prev>'` -> `'<curr>'`
2. `expected <prev> but got` -> `expected <curr> but got`

Do not regenerate via a full build; the text replace is deterministic and byte-identical to what `napi build` would produce for a version-only bump. Commit with this exact message shape (it makes `git log --grep` find the sync across releases) and push:

```
chore(release): sync binding/index.cjs version to <curr>

NAPI bakes the package.json version into binding/index.cjs version
checks. The prepare_release workflow bumps package.json but does not
regenerate this file, so the CI build's regeneration step produces a
diff that the post-build no-unexpected-changes guard rejects.
```

This is the only kind of commit that goes directly on the release branch. Everything else goes through `main` (see step 5).

## 3. Optional: pkg.pr.new smoke build

Add the `pkg.pr.new` label to the release PR to publish installable `0.0.0-commit.<head-sha>` builds:

```bash
gh pr edit <PR#> --repo voidzero-dev/vite-plus --add-label "pkg.pr.new"
```

The workflow triggers only on the `labeled` event, not on new pushes. To rebuild after the head moves, remove and re-add the label (this cancels an in-flight build for the branch). Use the `test-pkg-pr-new-migrate` skill to verify the build against a real project.

## 4. Write the release PR description

The release tag does not exist yet, so read release files from the PR head branch and generate notes against `main`:

```bash
git fetch --tags && git tag --sort=-version:refname | head -5   # find <prev>
git log --oneline v<prev>..origin/main
gh api repos/voidzero-dev/vite-plus/releases/generate-notes \
  -f tag_name=v<curr> -f previous_tag_name=v<prev> -f target_commitish=main

git show origin/release/v<curr>:packages/core/package.json          # bundledVersions: vite, rolldown, tsdown
git show origin/release/v<curr>:packages/tools/.upstream-versions.json  # vite/rolldown commit hashes
git show origin/release/v<curr>:pnpm-workspace.yaml                 # vitest/oxlint/oxlint-tsgolint/oxfmt catalog pins
```

### Structure

```markdown
Release vite-plus vX.Y.Z: <theme>.

<One or two sentences on the release theme. Link a blog post here when one accompanies the release.>

### Highlights
### Features
### Fixes & Enhancements
### Refactor
### Docs
### Chore
### Bundled Versions
### Upgrade
### New Contributors

**Full Changelog**: https://github.com/voidzero-dev/vite-plus/compare/v<prev>...v<curr>

---

Merging this PR will trigger the release workflow.
```

### Categorization rules

- Every PR from `generate-notes` appears exactly once. No PR is listed both in Highlights and a section below.
- `feat` -> Features, `fix` -> Fixes & Enhancements, `refactor` and `revert` -> Refactor (never Chore), `docs` -> Docs, `test` / `ci` / `chore` -> Chore.
- `feat(docs)` goes in Docs when the user-facing surface is the docs site.
- Highlights: 3-5 changes a vite-plus user will notice (new capabilities, security, major fixes). Skip developer-tooling-only conveniences. Each highlight ends with `, by @<author>`, same as every other entry.
- Entry format: `Description ([#N](https://github.com/voidzero-dev/vite-plus/pull/N)), by @author`. Describe the user-visible behavior, not the implementation.
- **Upstream dependency upgrade PRs** (`feat(deps): upgrade upstream dependencies`): consolidate all of them into one Features entry with net oldest-to-latest version changes (e.g. `vite 8.0.16 -> 8.1.2`), listing every PR number. Check the upgraded range for security fixes (search the upstream changelog for CVE/GHSA); if present, add a dedicated security entry quoting severity and linking the advisory.
- **vite-task bumps** (`bump vite-task to <commit>`): expand the full rev range (compare `Cargo.toml` at `v<prev>` vs the release branch), run `git log <old>..<new>` in the local vite-task checkout, and read vite-task's `CHANGELOG.md` at the new commit for wording. Promote user-visible upstream changes into Features / Fixes with `[vite-task#N](https://github.com/voidzero-dev/vite-task/pull/N)` links, crediting the upstream PR author (`gh pr view N --repo voidzero-dev/vite-task --json author`). Cross-repo link format is `[vite-task#N]` / `[vite#N]`, not `[owner/repo#N]`.
- New Contributors: copy from `generate-notes`, exclude bots (`renovate[bot]`, `voidzero-guard[bot]`, `github-actions[bot]`), list as inline `@mentions`.

### Bundled Versions table

| Tool | Version | Source |
| --- | --- | --- |
| vite | `X.Y.Z` | [`<short-sha>`](https://github.com/vitejs/vite/commit/<full-sha>) |
| rolldown | `X.Y.Z` | [`<short-sha>`](https://github.com/rolldown/rolldown/commit/<full-sha>) |
| tsdown | `X.Y.Z` | [npm](https://npmx.dev/package/tsdown/v/X.Y.Z) |
| vitest | `X.Y.Z` | [npm](https://npmx.dev/package/vitest/v/X.Y.Z) |
| oxlint | `X.Y.Z` | [npm](https://npmx.dev/package/oxlint/v/X.Y.Z) |
| oxlint-tsgolint | `X.Y.Z` | [npm](https://npmx.dev/package/oxlint-tsgolint/v/X.Y.Z) |
| oxfmt | `X.Y.Z` | [npm](https://npmx.dev/package/oxfmt/v/X.Y.Z) |

vite and rolldown are built from pinned commits, so link the commit. The npm-installed tools link to npmx.dev.

### Style rules

- No em dashes or en dashes anywhere in the title or body. Use commas, colons, or parentheses.
- The Upgrade section is a `vp upgrade` code block.
- Apply via a temp file, never a heredoc (heredoc quoting can escape backticks inside the table and break rendering):

```bash
gh pr edit <PR#> --repo voidzero-dev/vite-plus --title "release: vX.Y.Z: <theme>" --body-file /tmp/pr-body.md
```

### Validate before finishing

```bash
BODY=$(gh pr view <PR#> --repo voidzero-dev/vite-plus --json body -q '.body')
# every generate-notes PR present, none duplicated:
echo "$BODY" | grep -oE 'voidzero-dev/vite-plus/pull/[0-9]+' | sort -u | wc -l
echo "$BODY" | grep -oE '(vite-plus|vite-task)/pull/[0-9]+' | sort | uniq -d   # must be empty
echo "$BODY" | grep -nE '[—–]'                                                  # must be empty
echo "$BODY" | grep -c '\\`'                                                    # must be 0 (escaped backticks)
echo "$BODY" | tail -1                                                          # boilerplate closing line intact
```

## 5. Release-branch CI

Fixes for CI failures go through a **separate PR to `main`**, never as commits on the release branch (the binding sync in step 2 is the sole exception). After the fix PR merges:

```bash
git checkout release/vX.Y.Z && git merge origin/main --no-edit && git push origin release/vX.Y.Z
```

Then add the fix PR to the changelog (rerun the step 4 validation; the `generate-notes` count grows by one per merged PR).

Known release-branch-only failure modes:

- **Binding version drift**: `CLI E2E test` fails its no-unexpected-changes guard with a diff flipping version strings in `binding/index.cjs`. Fix: step 2.
- **Unpublished-version installs**: the release branch carries version `X.Y.Z` before it exists on npm. Snap fixtures that run a real package-manager install (`vp create` / `vp migrate` followed by install) fail with `ERR_PNPM_NO_MATCHING_VERSION` or bun/yarn `failed to resolve` for `vite-plus@X.Y.Z` / `@voidzero-dev/vite-plus-core@X.Y.Z`. Fix: pin `"VP_VERSION": "<published version>"` in the fixture's `steps.json` env (see #2017); the snapshot normalizer rewrites versions to `<semver>`, so output does not change. Any new install-performing fixture must ship with this pin or it will break the next release.
- **Registry flakes**: registry-bound fixtures can time out (about 50s) and look like regressions. Rerun before diagnosing, and never commit a `[timeout]` snapshot.

## 6. Merge

Merging the release PR is the release trigger. Before merging confirm: CI green, changelog validated, binding synced, and (if used) the pkg.pr.new build verified.

## 7. Automated release pipeline (what happens after merge)

`release.yml` runs on the `main` push because `packages/cli/package.json` changed:

1. `check`: compares the local version against `unpkg.com/vite-plus@latest`; everything below is skipped unless it changed.
2. `build-rust`: full multi-platform build.
3. `request-approval`: posts an approval request to the releases Discord channel, and the `Release` job waits on the `release` GitHub environment. **A person with environment approval rights must approve the run in the Actions UI.**
4. `Release`: publishes `@voidzero-dev/vite-plus-core` and `vite-plus` to npm (`--tag latest`), creates the `vX.Y.Z` GitHub release (draft, with installer/binary assets, then undrafted). The generated body has only Published Packages and Installation sections.
5. `publish-docker`: multi-arch toolchain image to `ghcr.io/voidzero-dev/vite-plus`, after npm publish (the image installs vp from npm).
6. `discord-notify`: announces to Discord with a link to the release.

## 8. Post-release

1. **Polish the GitHub release notes**: the auto-created release body lacks the changelog. Merge the release PR body into it with `gh release edit vX.Y.Z --repo voidzero-dev/vite-plus --notes-file ...`: changelog sections first, keep the generated Published Packages and Installation sections and the asset list, drop the `Merging this PR ...` line, and re-run the step 4 validation greps against the release body.
2. **Verify**:
   ```bash
   npm view vite-plus version                       # X.Y.Z
   npm view @voidzero-dev/vite-plus-core version    # X.Y.Z
   vp upgrade && vp --version                       # bundled tool versions sane
   docker pull ghcr.io/voidzero-dev/vite-plus:X.Y.Z
   ```
3. **Announce on Discord** (concise format only; do not produce a shorter variant). No PR links, no tables, no per-entry credits, no em dashes. One emoji per highlight by theme (`:lock:` security, `:zap:` performance, `:sparkles:` DX, `:seedling:` scaffolding, `:hammer_and_wrench:` tooling, `:package:` deps). The secondary list is titled **Also in this release** and must not repeat any highlight:

   ```markdown
   **vite-plus vX.Y.Z is out** :tada:

   <One-line theme summary.>

   **Highlights**
   :emoji: **Heading**: explanation.
   (3-5 lines)

   **Also in this release**
   - 5-7 bullets, no PR links or credits

   **Bundled versions**
   vite `X`, rolldown `X`, tsdown `X`, vitest `X`, oxlint `X`, oxlint-tsgolint `X`, oxfmt `X`

   **Upgrade**: `vp upgrade`

   Full release notes: <https://github.com/voidzero-dev/vite-plus/releases/tag/vX.Y.Z>
   Thanks to new contributors @a, @b :wave:
   ```

   The release-notes URL stays in `<angle brackets>` to suppress the embed; a blog post link (if any) goes bare so it unfurls.

## Checklist

- [ ] `prepare_release` run for the target version; release PR open
- [ ] `binding/index.cjs` synced on the release branch (step 2 commit message shape)
- [ ] PR description written from the head branch data; every PR exactly once; no em/en dashes; closing boilerplate intact
- [ ] Dependency-upgrade PRs consolidated; vite-task bump expanded with upstream credits; security advisories linked
- [ ] CI green; any fixes landed via separate PRs to main, merged back, and added to the changelog
- [ ] Release PR merged; `release` environment approved; npm + GitHub release + Docker image all published
- [ ] GitHub release notes polished and validated
- [ ] Installs verified (npm, `vp upgrade`, ghcr)
- [ ] Discord announcement posted (concise only)

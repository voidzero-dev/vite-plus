---
name: release-manager
description: Run the standard vite-plus release process end-to-end as the release manager. Covers preparing the release PR, syncing the NAPI binding version, writing the categorized changelog, smoke-testing via a preview build, handling release-branch CI failures, merging, and post-release verification and announcements. Use when asked to cut, prepare, or manage a vite-plus release (e.g. "release v0.2.3", "act as release manager").
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
- Is the PR description still the `prepare_release` boilerplate, or already a categorized changelog? (step 3)
- Is a preview build present and for the current head? (step 4)
- Does `main` have commits the release branch lacks? (`git log origin/release/vX.Y.Z..origin/main`, step 5)
- What is CI status? (`gh pr checks <PR#>`, step 5)
- Already merged? Check the Release workflow (`gh run list --workflow Release --repo voidzero-dev/vite-plus`) and whether the GitHub release body is still the generated stub, then continue at step 7 or 8.

Report the detected state before making changes, so the previous release manager's work is not redone or overwritten.

## Pipeline overview

1. `Prepare Release` workflow bumps versions and opens the release PR (`release/vX.Y.Z` -> `main`).
2. Release manager: sync `binding/index.cjs`, write the changelog PR description, offer the preview-build smoke test (recommend it when the release has more than 10 commits since the previous tag), get CI green.
3. Merging the PR pushes a `packages/cli/package.json` change to `main`, which triggers `release.yml`: build, manual approval gate, npm publish, GitHub release, Docker image, Discord notification.
4. Release manager: polish the GitHub release notes, verify installs, announce.

Canonical sources: `.github/workflows/prepare_release.yml`, `.github/workflows/release.yml`, `.github/workflows/publish-to-pkg.pr.new.yml`.

## 1. Start the release

```bash
gh workflow run prepare_release.yml --repo voidzero-dev/vite-plus -f version=X.Y.Z
```

The workflow bumps `packages/cli/package.json`, `packages/core/package.json`, `packages/cli/binding/Cargo.toml`, and `crates/vp_global_cli/Cargo.toml`, refreshes `Cargo.lock`, and opens a PR titled `release: vX.Y.Z` from branch `release/vX.Y.Z`. The PR body ends with `Merging this PR will trigger the release workflow.` and that line must survive every later edit.

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

Then confirm the replacement took; both counts must now be zero:

```bash
grep -c "'<prev>'" packages/cli/binding/index.cjs                 # 0
grep -c "expected <prev> but got" packages/cli/binding/index.cjs  # 0
```

Do not regenerate via a full build; the text replace is deterministic and byte-identical to what `napi build` would produce for a version-only bump. Commit with this exact message shape (it makes `git log --grep` find the sync across releases) and push:

```
chore(release): sync binding/index.cjs version to <curr>

NAPI bakes the package.json version into binding/index.cjs version
checks. The prepare_release workflow bumps package.json but does not
regenerate this file, so the CI build's regeneration step produces a
diff that the post-build no-unexpected-changes guard rejects.
```

This is the only kind of commit that goes directly on the release branch. Everything else goes through `main` (see step 5).

## 3. Write the release PR description

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

<One or two sentences on the release theme. When a blog post accompanies the release, read it first (via its preview URL if not yet deployed), align the theme with it, and link the final URL here even if that URL is not live yet.>

### Breaking Changes

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

- Every PR from `generate-notes` appears exactly once, with one exception: omit bot-authored PRs that carry nothing for a user to read or act on (a docs stats refresh, a badge update). Keep bot PRs that do change what users get, such as the upstream dependency upgrades. When you omit one, say so when reporting the validation counts so the mismatch reads as deliberate rather than missed. No PR is listed both in Highlights and a section below.
- **Breaking Changes goes first, above Highlights, and only when the release has one.** A rename is breaking only when the old name stops working; if a deprecated alias is retained it is not breaking, so keep the two in different sections rather than merging them into one entry. Give each breaking entry an old -> new table when several names change, plus one line telling readers where to update (shell profile, CI job, Dockerfile). Do not editorialize about the version number.
- **Describe the net change between the two released versions, not intra-cycle churn.** When several PRs touch the same area within one release (one narrows a behavior, a later one broadens it back), the reader only sees the delta from `v<prev>` to `v<curr>`; describe that once, listing every PR number, and do not narrate a regression that was introduced and then fixed inside the cycle. Apply this to the intro/theme sentence too.
- `feat` -> Features, `fix` -> Fixes & Enhancements, `refactor` and `revert` -> Refactor (never Chore), `docs` -> Docs, `test` / `ci` / `chore` -> Chore.
- `feat(docs)` goes in Docs when the user-facing surface is the docs site.
- Highlights: 3-5 changes a vite-plus user will notice (new capabilities, security, major fixes). Skip developer-tooling-only conveniences. Each highlight ends with `, by @<author>`, same as every other entry.
- Entry format: `Description ([#N](https://github.com/voidzero-dev/vite-plus/pull/N)), by @author`. Describe the user-visible behavior, not the implementation. Group supporting implementation PRs under the user-visible change they enable instead of giving them separate entries. Never include defensive edge cases or internal mechanics unless users need them to use or understand the feature; use concrete behavior instead of internal UI taxonomy that needs extra context.
- **Upstream dependency upgrade PRs** (`feat(deps): upgrade upstream dependencies`): consolidate all of them into one Features entry with net oldest-to-latest version changes (e.g. `vite 8.0.16 -> 8.1.2`), listing every PR number. Check the upgraded range for security fixes (search the upstream changelog for CVE/GHSA); if present, add a dedicated security entry quoting severity and linking the advisory. When oxfmt or oxlint changed version, add one clause telling users the new versions can flag code that passed before, so they should run `vp fmt` after upgrading if their CI runs `vp check`; in ecosystem testing this is reliably the largest single class of post-upgrade CI failures.
- **vite-task bumps** (`bump vite-task to <commit>`): expand the full rev range (compare `Cargo.toml` at `v<prev>` vs the release branch), run `git log <old>..<new>` in the local vite-task checkout, and read vite-task's `CHANGELOG.md` at the new commit for wording. Promote user-visible upstream changes into Features / Fixes with `[vite-task#N](https://github.com/voidzero-dev/vite-task/pull/N)` links, crediting the upstream PR author (`gh pr view N --repo voidzero-dev/vite-task --json author`). Cross-repo link format is `[vite-task#N]` / `[vite#N]`, not `[owner/repo#N]`.
- New Contributors: copy from `generate-notes`, exclude bots (`renovate[bot]`, `voidzero-guard[bot]`, `github-actions[bot]`), list as inline `@mentions`.

### Bundled Versions table

| Tool            | Version | Source                                                                  |
| --------------- | ------- | ----------------------------------------------------------------------- |
| vite            | `X.Y.Z` | [`<short-sha>`](https://github.com/vitejs/vite/commit/<full-sha>)       |
| rolldown        | `X.Y.Z` | [`<short-sha>`](https://github.com/rolldown/rolldown/commit/<full-sha>) |
| tsdown          | `X.Y.Z` | [npm](https://npmx.dev/package/tsdown/v/X.Y.Z)                          |
| vitest          | `X.Y.Z` | [npm](https://npmx.dev/package/vitest/v/X.Y.Z)                          |
| oxlint          | `X.Y.Z` | [npm](https://npmx.dev/package/oxlint/v/X.Y.Z)                          |
| oxlint-tsgolint | `X.Y.Z` | [npm](https://npmx.dev/package/oxlint-tsgolint/v/X.Y.Z)                 |
| oxfmt           | `X.Y.Z` | [npm](https://npmx.dev/package/oxfmt/v/X.Y.Z)                           |

vite and rolldown are built from pinned commits, so link the commit. The npm-installed tools link to npmx.dev.

### Style rules

- No em dashes or en dashes anywhere in the title or body. Use commas, colons, or parentheses.
- Lead the title and opening theme with the most important user-visible behavior. Avoid vague benefit-only wording that the body must explain.
- The Upgrade section is a `vp upgrade` code block.
- Apply via a temp file, never a heredoc (heredoc quoting can escape backticks inside the table and break rendering):

```bash
gh pr edit <PR#> --repo voidzero-dev/vite-plus --title "release: vX.Y.Z: <theme>" --body-file /tmp/pr-body.md
```

### Validate before finishing

```bash
BODY=$(gh pr view <PR#> --repo voidzero-dev/vite-plus --json body -q '.body')
# every generate-notes PR present (minus any deliberately omitted bot PR), none duplicated:
echo "$BODY" | grep -oE 'voidzero-dev/vite-plus/pull/[0-9]+' | sort -u | wc -l
echo "$BODY" | grep -oE '(vite-plus|vite-task)/pull/[0-9]+' | sort | uniq -d   # must be empty
echo "$BODY" | grep -nE '[—–]'                                                  # must be empty
echo "$BODY" | grep -c '\\`'                                                    # must be 0 (escaped backticks)
echo "$BODY" | tail -1                                                          # boilerplate closing line intact
```

Diff the body's PR numbers against `generate-notes` rather than only counting them: a count alone hides one missing entry offsetting one extra. Every number in the missing list must be a bot PR you chose to omit.

## 4. Preview build smoke test (before merging)

This step runs **after the changelog (step 3) is complete and before merging (step 6)**. **Always ask the release manager whether to run it, and ask about both levels explicitly** (the local `vp migrate` sweep, and the fork-PR CI validation below); never silently skip either, and do not add the label on your own.

Decide what to recommend before you ask, by counting the commits the release actually contains:

```bash
git rev-list --count v<prev>..origin/main
```

**Recommend running the smoke test when that count is above 10**, or when the release touches migrate/create behavior, package-manager or install-path handling, or the native bindings, whatever the count. Recommend skipping only for a release that is both small (10 commits or fewer) and clear of those areas. State the count and your recommendation in the question so the release manager can overrule it, and say roughly what it costs, since the full catalog runs for hours.

If the release manager approves, **read and follow [`vite-plus-ecosystem-ci/.github/TESTING.md`](https://github.com/vite-plus-ecosystem-ci/.github/blob/main/TESTING.md) first**, then validate against the **full ecosystem-ci catalog** (every runnable fork), not a single project.

If the release manager says yes:

1. Add the `preview-build` label to the release PR to publish installable `0.0.0-commit.<head-sha>` builds through the registry bridge:

   ```bash
   gh pr edit <PR#> --repo voidzero-dev/vite-plus --add-label "preview-build"
   ```

2. Wait for the `Publish preview build` workflow run on the release branch to succeed (it packs the built package directories and registers the commit with the registry bridge, then comments the build info on the PR).
3. Verify the build against every runnable project in the catalog with the `test-pkg-pr-new-migrate` skill: it runs `vp migrate` from the preview commit against each local checkout, with dependencies resolved through the registry bridge. Report the outcome to the release manager before moving on.

**The catalog.** The smoke-test catalog and the local-setup rules live in the ecosystem-ci org: [`vite-plus-ecosystem-ci/.github/TESTING.md`](https://github.com/vite-plus-ecosystem-ci/.github/blob/main/TESTING.md), with the machine-readable list in [`ecosystem.json`](https://github.com/vite-plus-ecosystem-ci/.github/blob/main/ecosystem.json) (each fork's upstream, tracked branch, and package manager). Run every runnable fork; filter `ecosystem.json` with `jq` to skip non-JS `other` repos and any the release manager says to ignore. Forks pinned to the immediately previous release exercise a real upgrade rather than a no-op.

> **Mandatory: open any test PR against the `vite-plus-ecosystem-ci` fork, never the upstream repo.** `gh pr create` inside a fork defaults its base repo to the parent (upstream), so pass `--repo vite-plus-ecosystem-ci/<repo>` (or run `gh repo set-default vite-plus-ecosystem-ci/<repo>` first). See TESTING.md.

`test-pkg-pr-new-migrate` needs a **local** checkout on the fork's **tracked branch** (often not the default branch, e.g. `vue-core` tracks `minor`). Clone under one directory so the whole test environment cleans up in one step:

```bash
repo=<repo>; branch=<tracked-branch>   # from ecosystem.json
git clone git@github.com:vite-plus-ecosystem-ci/$repo.git ~/git/github.com/vite-plus-ecosystem-ci/$repo
git -C ~/git/github.com/vite-plus-ecosystem-ci/$repo checkout "$branch"
# ... run the harness against ~/git/github.com/vite-plus-ecosystem-ci/$repo ...
# cleanup after the release: rm -rf ~/git/github.com/vite-plus-ecosystem-ci
```

The `.github` repo also ships `scripts/setup-local.sh <repo>` (or `--all`), which does the clone, tracked-branch checkout, remotes, and fork base-repo pinning from the manifest in one step.

**Sync every fork to upstream before you test anything.** The forks drift, often by hundreds of commits, so a checkout straight from `origin` validates stale code and any PR you open against it carries all that drift instead of just the upgrade. For each fork, fetch `source` and fast-forward the tracked branch, skipping any fork whose branch has commits upstream does not have rather than clobbering it:

```bash
git -C "$dir" fetch source
git -C "$dir" rev-list --left-right --count "origin/$branch...source/$branch"   # left must be 0 to fast-forward
git -C "$dir" push --no-verify origin "source/$branch:refs/heads/$branch"
```

Do this before both the local sweep and the fork PRs. If PRs were already opened against a stale base, GitHub will not recompute their merge base when the base branch moves; close and reopen each one to force it (a reopened draft stays a draft). TESTING.md carries the full procedure.

**Validate in the project's own CI.** Beyond the local `vp migrate`, exercise the prerelease in the fork's real CI by opening a draft PR on the fork, following "Smoke-test via a fork PR" in TESTING.md: branch `update-vite-plus-prerelease-test-<version>` synced from `source`, apply the upgrade, open a **draft** PR on the fork (never upstream) **assigned to the release manager**, then watch its checks for upgrade-related failures. Offer this alongside the local sweep rather than treating it as an afterthought; it is the only level that exercises each project's own build and tests. Some projects' CIs install with a non-standard tool that cannot resolve preview builds through the bridge `.npmrc` (e.g. cnpmcore's `utoo`), so check the install step before trusting fork-CI results.

The workflow triggers only on the `labeled` event, not on new pushes. To rebuild after the head moves (e.g. after a step 5 merge from `main`), remove and re-add the label (this cancels an in-flight build for the branch). A stale build whose diff to the new head is test-only is still valid for smoke testing; ask before re-triggering.

### Example (v0.2.2, PR #2016)

Changelog complete, CI green, release manager approved the smoke test. A build existed for head `06708538`; the head had since moved by a test-only merge from `main`, so that build was still valid and was not re-triggered.

Here the target was vibe-dashboard `main` (a `vite-plus-ecosystem-ci` fork, pnpm monorepo on `vite-plus 0.2.1`, i.e. the previous release), so the run exercised the common upgrade path. Pass the release PR number; the harness resolves it through the bridge to the latest published immutable commit and prints the resolved SHA (confirm it matches the build you expect). Pass a full commit SHA instead only to pin a specific build when several have been published:

```bash
.github/scripts/test-pkg-pr-new-migrate.sh 2016 ~/git/github.com/vite-plus-ecosystem-ci/vibe-dashboard --no-interactive
```

A passing run looks like:

```
◇ Updated . to Vite+ 0.0.0-commit.06708538...
• Dependencies:
    vite-plus  0.2.1 → 0.0.0-commit.06708538...
    vite             → 8.1.2
✓ Dependencies installed in 5.7s

Migration worktree changes (.npmrc force-staged so it survives .gitignore):
A  .npmrc                      # bridge registry written by vp migrate
 M package.json / pnpm-workspace.yaml / pnpm-lock.yaml

Found 1 version of @voidzero-dev/vite-plus-core
Found 1 version of vite-plus
Found 1 version of vitest
```

Pass criteria: the upgrade lands on the `0.0.0-commit.<sha>` build, the install succeeds through the bridge registry, and each of `@voidzero-dev/vite-plus-core`, `vite-plus`, and `vitest` resolves to exactly ONE version (`vitest` at the bundled upstream version). Multiple or stale versions mean the migration or install is broken: stop and treat it as a release blocker. Report the outcome to the release manager either way.

### Triaging failures across the catalog

Across the full catalog most failures are not regressions, and reporting them as "N failed" without triage is useless to the release manager. Sort every failure into one of these before drawing any conclusion:

- **Preview-build artifacts.** These are caused by the `0.0.0-commit.<sha>` version string itself and cannot happen for a real npm release, so they are never blockers. The recurring ones: pnpm `ERR_PNPM_TRUST_DOWNGRADE` ("possible package takeover"), npm `ETARGET` from a `before`/min-release-age policy, bun `minimum release age`, `ERR_PNPM_INVALID_PEER_DEPENDENCY_SPECIFICATION` when a project declares `vite` as a peer (migrate writes the `npm:@voidzero-dev/vite-plus-core@...` alias there), and Docker builds whose context does not carry the bridge `.npmrc`.
- **Pre-existing failures.** Prove it rather than asserting it, with whichever control is cheaper: install the previous release into an isolated home and re-run the same command, or check whether the fork's base branch CI already fails. The isolated-home control is the highest-value technique in this step, since it converts a scary-looking failure into a one-line fact:

  ```bash
  VP_HOME=$HOME/.cache/vp-control-<prev> VP_VERSION=<prev> VP_NODE_MANAGER=no bash packages/cli/install.sh
  VP_HOME=$HOME/.cache/vp-control-<prev> PATH="$HOME/.cache/vp-control-<prev>/bin:$PATH" vp migrate <project> --no-interactive
  ```

  Reset the project (`git reset --hard`, `git clean -fd`, delete lockfile and `node_modules`) before the control run so it starts from the same state as the real run.

- **Project-side and infra failures.** Dependency conflicts between the project's own packages, missing fork secrets, third-party GitHub Apps not installed on the fork, network timeouts. Retry once before classifying anything as a network failure; they pass on retry.
- **Harness artifacts.** Failures your own test setup caused, such as a lockfile the harness deleted and the install never regenerated. Fix these and re-run rather than reporting them.

Report the tally by cause, not just pass/fail, and state plainly which failures you controlled for and which you classified from the error text alone. Only a failure that reproduces on the candidate but not on the previous release is a regression.

Two long-run mechanics worth knowing: `vp migrate` installs Vite+ git hooks in the project, so any later `git commit`/`git push` there needs `--no-verify`; and macOS has no GNU `timeout`, so a driver script that time-boxes runs needs its own watchdog.

## 5. Release-branch CI

Fixes for CI failures go through a **separate PR to `main`**, never as commits on the release branch (the binding sync in step 2 is the sole exception). After the fix PR merges:

```bash
git checkout release/vX.Y.Z && git merge origin/main --no-edit && git push origin release/vX.Y.Z
```

Do not assume the merge brought in only the fix PR: `main` may have accumulated several. Before merging, list everything that will come in with `git log origin/release/vX.Y.Z..origin/main --oneline`, then add a changelog entry for **every** newly included PR and rerun the step 3 validation (its missing/extra diff against `generate-notes` catches any entry you missed).

Known release-branch-only failure modes:

- **Binding version drift**: CI's no-unexpected-changes guard reports a diff flipping version strings in `binding/index.cjs`. Fix: step 2.
- **Registry flakes**: registry-bound fixtures can time out (about 50s) and look like regressions. Rerun before diagnosing, and never commit a `[timeout]` snapshot.

## 6. Merge

Merging the release PR is the release trigger. Before merging confirm: CI green, changelog validated, binding synced, and (if used) the preview build verified.

## 7. Automated release pipeline (what happens after merge)

`release.yml` runs on the `main` push because `packages/cli/package.json` changed:

1. `check`: compares the local version against `unpkg.com/vite-plus@latest`; everything below is skipped unless it changed.
2. `build-rust`: full multi-platform build.
3. `request-approval`: posts an approval request to the releases Discord channel, and the `Release` job waits on the `release` GitHub environment. **A person with environment approval rights must approve the run in the Actions UI.** The environment sets `prevent_self_review: true`, so whoever merged the release PR triggered the run and cannot approve it: a _different_ reviewer must. Check who can, and tell the release manager rather than leaving them waiting on themselves:

   ```bash
   gh api repos/voidzero-dev/vite-plus/actions/runs/<run-id>/pending_deployments \
     -q '.[] | "\(.environment.name) can_approve=\(.current_user_can_approve) reviewers=\([.reviewers[]?.reviewer.login] | join(","))"'
   ```

4. `Release`: publishes the platform-native CLI packages (`@voidzero-dev/vite-plus-cli-<platform>`, via `packages/cli/publish-native-addons.ts`) and then `@voidzero-dev/vite-plus-core` and `vite-plus` to npm (`--tag latest`), creates the `vX.Y.Z` GitHub release (draft, with installer/binary assets, then undrafted). The generated body has only Published Packages and Installation sections.
5. `publish-docker`: multi-arch toolchain image to `ghcr.io/voidzero-dev/vite-plus`, after npm publish (the image installs vp from npm).
6. `discord-notify`: announces to Discord with a link to the release.

## 8. Post-release

1. **Polish the GitHub release notes** (ask first): the auto-created release body has only Published Packages and Installation. Build the polished notes from the final release PR body:
   - Drop the `Release vite-plus vX.Y.Z: ...` opener line (the release title carries it) and the closing `---` / `Merging this PR ...` boilerplate.
   - Keep every changelog section through **Full Changelog** unchanged.
   - Append the generated Published Packages and Installation sections, and end Installation with a Docker usage block (keep the explanation to one short sentence):

     ````markdown
     **Docker:**

     ```bash
     docker run --rm -it -v "$PWD:/app" -w /app ghcr.io/voidzero-dev/vite-plus:X.Y.Z vp build
     ```

     Run any `vp` command without installing it; see the [Docker guide](https://viteplus.dev/guide/docker) for more.
     ````

   - **Present the draft to the release manager and apply only after approval.** Before review, write the complete draft to a temporary Markdown file with the proposed release title at the top and the full body below it. Update that file after every requested revision; do not treat chat excerpts as the canonical draft. After approval, use a body-only notes file (without the review title) to retitle the release and apply the notes:

     ```bash
     gh release edit vX.Y.Z --repo voidzero-dev/vite-plus \
       --title "vite-plus vX.Y.Z: <theme>" --notes-file /tmp/release-notes.md
     ```

   - Re-run the step 3 validation greps against the live release body, plus `grep -c 'Merging this PR'` (must be 0).

2. **Verify**:

   ```bash
   npm view vite-plus version                       # X.Y.Z
   npm view @voidzero-dev/vite-plus-core version    # X.Y.Z
   npm view @voidzero-dev/vite-plus-cli-darwin-arm64 version   # X.Y.Z, spot-check a native platform package
   npm view vite-plus dist-tags.latest              # X.Y.Z
   vp upgrade && vp --version                       # bundled tool versions sane
   docker run --rm ghcr.io/voidzero-dev/vite-plus:X.Y.Z vp --version
   ```

   `vp upgrade` reporting `Already up to date (X.Y.Z)` also passes. Caveat: `vp upgrade` exists only on standalone-installer (`~/.vite-plus`) installs; if the release manager's `vp` is managed another way (e.g. mise), `vp upgrade` is missing and `vp update` is not a substitute (it runs `pnpm update` on the current project, so never run it inside the vite-plus checkout). In that case rely on the npm/GHCR checks plus the in-container `vp --version`. Do not trust `vp upgrade`'s success message on its own: on a machine where `~/.vite-plus/current` is a symlink to a `local-dev-*` directory (a `pnpm bootstrap-cli` build), it can print `Updated vite-plus from A -> B` while creating no `~/.vite-plus/X.Y.Z` directory and leaving `current` untouched. Confirm with `readlink ~/.vite-plus/current` and `ls ~/.vite-plus`, and if that is the case say the check was inconclusive rather than reporting it as passing. The Docker check must run `vp --version` inside the image, not just pull it: the output must report `vp vX.Y.Z`. Outside a project that output lists no bundled tools, so inspect the installed image package tree under `~/.vite-plus/X.Y.Z/node_modules/.pnpm` and confirm the bundled tool packages and versions match the changelog's Bundled Versions table. `tsdown` will be absent from that tree because it is bundled into `@voidzero-dev/vite-plus-core`; verify it with `npm view @voidzero-dev/vite-plus-core@X.Y.Z bundledVersions --json` instead. If no local Docker daemon is running, confirm the `publish-docker` job succeeded and the GHCR manifest exists, then still run the in-container check once a daemon is available:

   ```bash
   TOKEN=$(curl -s "https://ghcr.io/token?scope=repository:voidzero-dev/vite-plus:pull" \
     | python3 -c "import json,sys; print(json.load(sys.stdin)['token'])")
   curl -sI -H "Authorization: Bearer $TOKEN" \
     -H "Accept: application/vnd.oci.image.index.v1+json" \
     "https://ghcr.io/v2/voidzero-dev/vite-plus/manifests/X.Y.Z" | head -1   # HTTP/2 200
   ```

3. **Announce on Discord** (concise format only; do not produce a shorter variant). Keep it tight: every line is a single short phrase, no heading-plus-explanation sentences, the whole message around 20 lines. No PR links, no tables, no per-entry credits, no em dashes. Make the theme and highlights self-contained by naming the affected capability rather than using vague benefit-only wording. Use verbs that match the actual behavior, especially distinguishing guidance or suggestions from automatic actions. One emoji per line by theme (`:lock:` security, `:zap:` performance, `:sparkles:` DX, `:seedling:` scaffolding, `:hammer_and_wrench:` tooling, `:package:` deps). Use **Upstream Upgrades** for dependency/tool version bumps, not Highlights, and list only tools whose version actually changed; a security fix caused by a dependency bump can still have a Highlight focused on the vulnerability, and that line must link the CVE/GHSA/advisory when one exists. A breaking change gets its own `:warning:` Highlight naming the old and new names and what the reader must update. Include **Also in this release** only when there are meaningful secondary user-facing items, and omit the whole section for a narrow hotfix.

   ```markdown
   :viteplus: **vite-plus vX.Y.Z is out** :tada:

   <One short theme line.>

   **Highlights**
   :emoji: one short user-impact line per highlight
   (1-5 lines; link CVE/GHSA/advisory text for security items)

   **Upstream Upgrades**
   :package: tool `old` -> `new`
   (omit if no upstream version changes worth naming)

   **Also in this release**

   - 2-6 short bullets, no PR links or credits
     (omit this whole section when there are no meaningful secondary user-facing items)

   **Bundled versions**
   vite `X`, rolldown `X`, tsdown `X`, vitest `X`, oxlint `X`, oxlint-tsgolint `X`, oxfmt `X`

   **Upgrade**: `vp upgrade`

   Full notes: <https://github.com/voidzero-dev/vite-plus/releases/tag/vX.Y.Z>
   Thanks to new contributors [@a](https://github.com/a), [@b](https://github.com/b) :wave:
   ```

   The release-notes URL stays in `<angle brackets>` to suppress the embed; a blog post link (if any) goes bare so it unfurls. Lead the header with the server custom emoji `:viteplus:` (before the bold title, since it is a custom emoji). Link contributors as `[@user](https://github.com/user)` because Discord does not auto-link a bare GitHub handle. Keep the whole message user-facing: exclude vite-plus's own tooling/CI work.

   Never post to Discord yourself. Save the draft to a file, update that file after every requested revision, and post the approved contents as a comment on the release PR wrapped in a fenced ` ```markdown ` block, so the `@mentions` do not ping anyone on GitHub, the emoji shortcodes stay literal, and any team member can copy-paste it into Discord. After the release manager approves the Discord draft, proceed directly to step 9; do not wait for another prompt or treat the skill update as optional.

## 9. Update this skill (post-release)

After the release ships and the Discord announcement draft is approved, review the session for durable learnings and fold them into this file. Then ask for approval before pushing or opening a PR.

- Capture only what generalizes: a step whose instructions drifted from what actually worked, a gotcha or corrected mistake, or a command/flag that was wrong. Write it as **general guidance**, with no release-specific versions, project names, PR numbers, or one-off examples.
- Be surgical: change only what was wrong or missing; do not reword content that was already correct. If nothing generalizes, make no change.
- This skill lives on `main`, so do not push to `main` directly: make the edit on a branch, commit it, present the local diff and summary to the release manager, and **only push or open a `docs(skill): ...` PR after explicit approval**.

## Checklist

- [ ] `prepare_release` run for the target version; release PR open
- [ ] `binding/index.cjs` synced on the release branch (step 2 commit message shape)
- [ ] PR description written from the head branch data; every PR exactly once except deliberately omitted bot-noise PRs; breaking changes in their own section above Highlights; no em/en dashes; closing boilerplate intact
- [ ] Dependency-upgrade PRs consolidated; vite-task bump expanded with upstream credits; security advisories linked
- [ ] Smoke test offered to the release manager at both levels (local sweep and fork-PR CI), with the commit count stated and a recommendation to run it when that count is above 10; if accepted, forks synced to upstream first, preview build published, and the full ecosystem-ci catalog verified via `test-pkg-pr-new-migrate` (following TESTING.md), with every failure triaged and regressions ruled out against the previous release
- [ ] CI green; any fixes landed via separate PRs to main, merged back, and added to the changelog
- [ ] Release PR merged; `release` environment approved by someone other than the merger; npm + GitHub release + Docker image all published
- [ ] GitHub release notes polished (release manager approved before applying), retitled, and validated; Installation ends with the Docker usage block
- [ ] Installs verified (npm versions + latest tag, `vp upgrade`, `vp --version` output inside the ghcr Docker image)
- [ ] Discord announcement drafted (concise only) and shared as a fenced code block comment on the release PR
- [ ] Skill reviewed for durable learnings; any that generalize folded in and a `docs(skill)` PR proposed

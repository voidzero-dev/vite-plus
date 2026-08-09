# RFC: Deploy docs on release

- Motivating example: [#2346](https://github.com/voidzero-dev/vite-plus/pull/2346) (XDG directory layout, rewrites `install.sh` / `install.ps1`)

## Summary

Stop deploying viteplus.dev on every push to `main`. Deploy it from the release
workflow after the npm packages and the GitHub release are published, and keep
`workflow_dispatch` for manual deploys. The site and the install scripts then
always describe the latest released `vp`.

Pushes to `main` keep an automatic deploy, but to a dedicated preview project
at `https://main.viteplus.dev` (a CNAME to `viteplus-main.void.app`), so
developers can read the latest docs on `main` before the next release.

## Current behavior

`deploy-docs.yml` runs on every push to `main` that touches `docs/**`,
`packages/cli/install.sh`, `packages/cli/install.ps1`, or the workflow file. It
builds the VitePress site and deploys it to the `viteplus` void.app project.

The docs build copies the install scripts into the site
(`docs/package.json` `build` script copies them into `docs/public/`).
`https://vite.plus` redirects to `https://viteplus.dev/install.sh`, and
`https://vite.plus/ps1` to `install.ps1`. The docs deploy is therefore also the
production channel for the installer.

The release pipeline is separate. Merging a release PR bumps
`packages/cli/package.json`; `release.yml` verifies the version changed, builds,
waits for manual approval in the `release` environment, then publishes the npm
packages, the GitHub release, and the Docker image.

This creates two failure modes:

1. **Feature docs go live before the release exists.** A PR that adds code and
   documents it deploys the docs at merge time. Users read about commands and
   flags the released `vp` does not have. The gap between merge and release can
   be days, since releases need a version bump and manual approval.
2. **Install-script changes go live before the binaries that match them.**
   [#2346](https://github.com/voidzero-dev/vite-plus/pull/2346) is the concrete
   case: merging it would serve an `install.sh` that installs into the split
   XDG layout, while `vp` from the latest release still resolves the legacy
   `~/.vite-plus` layout. Fresh installs break until the next release ships.

## Proposal

Three parts:

1. Change `deploy-docs.yml` from a push-triggered workflow into a reusable one.
2. Call it from `release.yml` after the release is published.
3. Add `deploy-docs-main.yml`, which takes over the push trigger and deploys
   `main` to the `viteplus-main` preview project.

### `deploy-docs.yml`

1. Remove the `push` trigger. Keep `workflow_dispatch`. Add `workflow_call`
   with `VOID_TOKEN` declared as a required secret.
2. Move the `deploy-docs` concurrency group from the workflow level to the
   `deploy` job. Jobs of a called workflow run inside the caller's run, so
   workflow-level concurrency in the called file does not apply there.
   Job-level concurrency serializes production deploys across both entry
   paths (`cancel-in-progress: false` as today).

```yaml
on:
  workflow_dispatch:
  workflow_call:
    secrets:
      VOID_TOKEN:
        required: true

jobs:
  deploy:
    if: github.repository == 'voidzero-dev/vite-plus'
    runs-on: ubuntu-latest
    concurrency:
      group: deploy-docs
      cancel-in-progress: false
    permissions:
      contents: read
    env:
      VOID_PROJECT: viteplus
    # ... existing steps unchanged
```

### `release.yml`

Add one job:

```yaml
  deploy-docs:
    name: Deploy docs
    needs: [check, Release]
    if: >-
      needs.check.outputs.version_changed == 'true' &&
      !contains(needs.check.outputs.version, '-')
    permissions:
      contents: read
    uses: ./.github/workflows/deploy-docs.yml
    secrets:
      VOID_TOKEN: ${{ secrets.VOID_TOKEN }}
```

- The called workflow checks out `github.sha`, which in the release run is the
  release commit. The deployed site matches the released version.
- `needs: [check, Release]` runs the deploy once npm and the GitHub release are
  out, in parallel with `publish-docker`. Docs do not depend on the image.
- The `!contains(version, '-')` guard skips prereleases. An alpha publish must
  not overwrite the production site with docs for unreleased behavior.
- A docs-deploy failure does not undo the release. Re-run the job or dispatch
  the workflow manually.

### `deploy-docs-main.yml`: standing preview of `main`

A new workflow takes over the push trigger that `deploy-docs.yml` loses. It
runs the same build steps and deploys to a dedicated `viteplus-main` void.app
project instead of production:

```yaml
name: Deploy Docs Main Preview

permissions: {}

on:
  push:
    branches: [main]
    paths:
      - 'docs/**'
      - 'packages/cli/install.sh'
      - 'packages/cli/install.ps1'
      - '.github/workflows/deploy-docs-main.yml'

concurrency:
  group: deploy-docs-main
  cancel-in-progress: true

jobs:
  deploy:
    if: github.repository == 'voidzero-dev/vite-plus'
    runs-on: ubuntu-latest
    permissions:
      contents: read
    env:
      VOID_PROJECT: viteplus-main
    # ... same build and deploy steps as deploy-docs.yml
```

- `https://main.viteplus.dev` always shows the docs at the head of `main`,
  including unreleased features. Developers get a stable link to share,
  without a release or a manual dispatch.
- A dedicated project, not the shared `viteplus-staging` one: PR previews
  deploy there and would overwrite the `main` preview on every PR push.
- `cancel-in-progress: true`: only the newest `main` deploy matters for a
  preview. Production keeps `false`.
- The workflow can keep the `vite-task-docs-*-main-*` cache keys that
  `deploy-docs.yml` uses today, since both build `main`; the release-run
  deploy restores from the same key family.
- Setup before the workflow lands: create the `viteplus-main` project on the
  void platform (the deploy uses the same `VOID_TOKEN` secret), add the DNS
  CNAME `main.viteplus.dev` -> `viteplus-main.void.app`, and attach the
  custom domain to the project.

`deploy-docs-preview.yml` stays unchanged. Per-PR staging deploys to
`viteplus-staging.void.app` remain the place to review docs changes before
merge.

### Manual deploys

`workflow_dispatch` covers urgent updates outside the release cycle:

- Main carries no unreleased docs since the last release commit: merge the fix
  to `main`, dispatch Deploy Docs on `main`.
- Main already carries unreleased docs: cut a branch from the release tag,
  cherry-pick the fix, dispatch Deploy Docs on that branch. `workflow_dispatch`
  accepts any branch or tag as the ref.

A dispatch on `main` publishes everything on `main`, including unreleased docs
if present. The operator has to check for that; the release-tag branch is the
safe path.

### No chicken-and-egg on the installer

The docs build in the release run installs `vp` through `setup-vp`, which
fetches the install script currently deployed on viteplus.dev. The run builds
the site with the previous script, then the deploy replaces it. Fresh installs
after the deploy get the new script together with the new binaries.

## Why `workflow_call` and not another trigger

- `on: release: types: [published]` does not fire. `release.yml` publishes the
  release with the default `GITHUB_TOKEN` (`gh release edit --draft=false`),
  and events created with `GITHUB_TOKEN` do not start workflow runs. A PAT or
  GitHub App token would work around this at the cost of another credential.
- `workflow_run` on Release completion runs at the head of the default branch,
  not at the release commit. Docs merged after the release commit would deploy
  with it, which reintroduces problem 1.
- `gh workflow run` at the end of the Release job works (`workflow_dispatch`
  is exempt from the `GITHUB_TOKEN` restriction) but needs `actions: write`
  and detaches the deploy from the release run in the Actions UI.
  `workflow_call` keeps the deploy visible and gated inside the release run.

## Behavior changes

| Event | Before | After |
| --- | --- | --- |
| Docs change merges to `main` | Production deploy | Preview deploy to `main.viteplus.dev` |
| `install.sh` / `install.ps1` change merges to `main` | Production deploy | Preview deploy to `main.viteplus.dev` |
| Stable release published | No docs deploy | Production deploy from the release commit |
| Prerelease published | No docs deploy | No docs deploy (`main` preview already current) |
| Manual dispatch | Redundant with push deploys | The escape hatch for urgent production updates |

## Drawbacks

- Docs-only fixes (typos, clarifications) reach production with the next
  release unless someone dispatches a deploy. Today they go live within
  minutes of merge. They do reach `viteplus-main` within minutes.
- The production site lags `main` by design. Contributors who expect merged
  docs on viteplus.dev must link to `main.viteplus.dev` until the next
  release.
- One more job in `release.yml`, and the release run gains the docs build time
  (a few minutes, in parallel with the Docker publish).
- Two sites can index in search engines. The preview project should send
  `noindex` (or the theme should emit it when the site URL is not
  viteplus.dev) so `main.viteplus.dev` does not compete with production.

## Alternatives considered

- **Gate only the install scripts, keep push deploys for `docs/**`.** Fixes
  problem 2 but not problem 1, and lets `docs/guide/install.md` drift from the
  script it documents. Two freshness channels on one site.
- **Versioned docs.** Publish `main` but hide unreleased sections until their
  release. Needs authoring conventions and theme/tooling support; out of scope.
- **One workflow file for production and the `main` preview, with the project
  chosen by trigger.** Selecting `VOID_PROJECT` from `github.event_name` does
  not work: a called workflow inherits the caller's event, and `release.yml`
  itself runs on `push`, so the release-run deploy would look like a push and
  target the preview project. A `workflow_call` input can disambiguate, but
  the fallback logic for the direct-push case is easy to get wrong. Two small
  files with explicit projects read clearer.
- **Deploy `main` preview to the existing `viteplus-staging` project.** PR
  previews deploy there and would overwrite the `main` preview on every PR
  push.
- **Factor the shared build and deploy steps into one reusable workflow with a
  `project` input**, called by the production, `main`-preview, and PR-preview
  workflows. Removes the step duplication that already exists between
  `deploy-docs.yml` and `deploy-docs-preview.yml`. Reasonable follow-up
  cleanup; kept out of this change to keep the diff reviewable.

## Open questions

- Should the deploy also wait for `publish-docker`, so Docker install docs
  never precede the image? Waiting adds a few minutes to the deploy.

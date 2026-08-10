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

Four parts:

1. Extract the docs build and deploy steps into a
   `.github/actions/deploy-docs` composite action, the repo convention for
   shared step sequences (see `.github/actions/clone`, `build-windows-cli`).
2. Reduce `deploy-docs.yml` to a manual (`workflow_dispatch`) production
   deploy that runs the composite.
3. Add a `deploy-docs` job to `release.yml` that runs the composite after the
   release is published.
4. Add `deploy-docs-main.yml`, which takes over the push trigger and deploys
   `main` to the `viteplus-main` preview project.
5. Switch `deploy-docs-preview.yml` to the composite; its trigger, staging
   target, and PR comment step stay as they are.

### `.github/actions/deploy-docs`

The composite action holds the steps shared by every docs deploy: `setup-vp`,
the Vite Task cache restore/save, `vp run build`, and `vpx void deploy`. Its
inputs:

- `void-project`: the deploy target.
- `void-token`: composite actions cannot read secrets, so the caller passes
  `secrets.VOID_TOKEN`.
- `cache-ref` / `cache-sha` (optional, default `main` / `github.sha`): scope
  the Vite Task cache key. PR previews pass `pr-<number>` and the head sha,
  which reproduces their current per-PR keys with a fallback to the `main`
  cache.

Callers check out the repo first, then run the action.

### `deploy-docs.yml`

Remove the `push` trigger; keep `workflow_dispatch` only. The build and
deploy steps move to the composite:

```yaml
on:
  workflow_dispatch:

concurrency:
  group: deploy-docs
  cancel-in-progress: false

jobs:
  deploy:
    if: github.repository == 'voidzero-dev/vite-plus'
    runs-on: ubuntu-latest
    permissions:
      contents: read
    steps:
      - uses: taiki-e/checkout-action@... # v1.4.2

      - uses: ./.github/actions/deploy-docs
        with:
          void-project: viteplus
          void-token: ${{ secrets.VOID_TOKEN }}
```

### `release.yml`

Add one job:

```yaml
deploy-docs:
  name: Deploy docs
  runs-on: ubuntu-latest
  needs: [check, Release]
  if: >-
    needs.check.outputs.version_changed == 'true' &&
    !contains(needs.check.outputs.version, '-')
  concurrency:
    group: deploy-docs
    cancel-in-progress: false
  permissions:
    contents: read
  steps:
    - uses: taiki-e/checkout-action@... # v1.4.2

    - uses: ./.github/actions/deploy-docs
      with:
        void-project: viteplus
        void-token: ${{ secrets.VOID_TOKEN }}
```

- The job checks out `github.sha`, which in the release run is the release
  commit. The deployed site matches the released version.
- The job-level `deploy-docs` concurrency group is shared with
  `deploy-docs.yml`, so production deploys serialize across both entry paths.
- GitHub keeps one pending run per concurrency group: a newer queued deploy
  replaces a pending one, while the running deploy always completes. So
  production converges to the newest queued deploy. If a release's pending
  deploy is the one replaced, the canceled job shows in the release run and
  holds back `discord-notify`; re-run it if the replacing deploy carried
  older content.
- `needs: [check, Release]` runs the deploy once npm and the GitHub release are
  out, in parallel with `publish-docker`. Docs do not depend on the image.
- The `!contains(version, '-')` guard skips prereleases. An alpha publish must
  not overwrite the production site with docs for unreleased behavior.
- `discord-notify` adds `deploy-docs` to its `needs` and gates on
  `result == 'success' || result == 'skipped'`, so a stable release announces
  only after the site is updated, while prereleases (where `deploy-docs` is
  skipped) still announce.
- A docs-deploy failure does not undo the release; it holds back the Discord
  announcement. Re-run the job or dispatch the workflow manually.

### `deploy-docs-main.yml`: standing preview of `main`

A new workflow takes over the push trigger that `deploy-docs.yml` loses. It
runs the same composite and deploys to a dedicated `viteplus-main` void.app
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
      - '.github/actions/deploy-docs/**'

concurrency:
  group: deploy-docs-main
  cancel-in-progress: true

jobs:
  deploy:
    if: github.repository == 'voidzero-dev/vite-plus'
    runs-on: ubuntu-latest
    permissions:
      contents: read
    steps:
      - uses: taiki-e/checkout-action@... # v1.4.2

      - uses: ./.github/actions/deploy-docs
        with:
          void-project: viteplus-main
          void-token: ${{ secrets.VOID_TOKEN }}
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

`deploy-docs-preview.yml` keeps its trigger, its `viteplus-staging.void.app`
target, and its PR comment step, and now runs the same composite. Per-PR
staging deploys remain the place to review docs changes before merge.

### Origin-aware install URLs in the docs

The docs hardcode the production installer shortcuts (`https://vite.plus`,
`https://vite.plus/ps1`) in markdown code blocks and in the homepage Vue
components. A preview deploy must point them at its own install scripts
instead (for example `https://main.viteplus.dev/install.sh`), or readers of
unreleased install docs run the production installer.

The composite action passes its `site-origin` input to the docs build as
`DOCS_SITE_ORIGIN`. When the variable is set:

- A markdown-it rule in `.vitepress/config.mts` rewrites the install URLs in
  fenced code, inline code, text, and link hrefs. Other `viteplus.dev`
  subdomains (`setup.`, `registry-bridge.`) stay untouched.
- The Vue components (homepage install command, AI copy prompt) read
  `__DOCS_*__` define constants computed from the same origin. The AI prompt
  also points at the deploy's own `llms-full.txt`.
- The `build:site` run task lists `DOCS_SITE_ORIGIN` in `env`, so each deploy
  target keeps its own Vite Task cache entry.

Production builds leave the content untouched: the variable is unset there.
Known gap: the llms dumps (`llms.txt`, `llms-full.txt`, per-page `.md`) copy
raw markdown, so on previews they keep the production install URLs.

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

## Why a job in the release run and not another trigger

- `on: release: types: [published]` does not fire. `release.yml` publishes the
  release with the default `GITHUB_TOKEN` (`gh release edit --draft=false`),
  and events created with `GITHUB_TOKEN` do not start workflow runs. A PAT or
  GitHub App token would work around this at the cost of another credential.
- `workflow_run` on Release completion runs at the head of the default branch,
  not at the release commit. Docs merged after the release commit would deploy
  with it, which reintroduces problem 1.
- `gh workflow run` at the end of the Release job works (`workflow_dispatch`
  is exempt from the `GITHUB_TOKEN` restriction) but needs `actions: write`
  and detaches the deploy from the release run in the Actions UI. A job in
  the release run keeps the deploy visible and gated inside it.

## Behavior changes

| Event                                                | Before                      | After                                           |
| ---------------------------------------------------- | --------------------------- | ----------------------------------------------- |
| Docs change merges to `main`                         | Production deploy           | Preview deploy to `main.viteplus.dev`           |
| `install.sh` / `install.ps1` change merges to `main` | Production deploy           | Preview deploy to `main.viteplus.dev`           |
| Stable release published                             | No docs deploy              | Production deploy from the release commit       |
| Prerelease published                                 | No docs deploy              | No docs deploy (`main` preview already current) |
| Manual dispatch                                      | Redundant with push deploys | The escape hatch for urgent production updates  |

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
- **A reusable workflow (`workflow_call`) called from `release.yml`, instead
  of a composite action.** `reusable-release-build.yml` sets a precedent, but
  it reuses a whole job matrix; the docs deploy reuses a step sequence inside
  jobs with different triggers, gates, and concurrency, which is what the
  `.github/actions` composites are for. `workflow_call` also carries
  subtleties: the called workflow inherits the caller's `github` context and
  event, its workflow-level concurrency has no effect, and secrets must be
  declared and forwarded. The composite avoids all three and also serves
  `deploy-docs-preview.yml`.
- **One workflow file for production and the `main` preview, with the project
  chosen by trigger.** Selecting `VOID_PROJECT` from `github.event_name` is
  implicit and fragile, and `release.yml` still needs its own release-gated
  job. Thin wrappers with explicit projects read clearer.
- **Deploy `main` preview to the existing `viteplus-staging` project.** PR
  previews deploy there and would overwrite the `main` preview on every PR
  push.

## Open questions

- Should the deploy also wait for `publish-docker`, so Docker install docs
  never precede the image? Waiting adds a few minutes to the deploy.

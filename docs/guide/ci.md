# Continuous Integration

You can use `voidzero-dev/setup-vp` to use Vite+ in CI environments.

## Overview

[`voidzero-dev/setup-vp`](https://github.com/voidzero-dev/setup-vp) provides integrations for GitHub Actions and GitLab CI/CD. Both install Vite+ and can install project dependencies. The GitHub Action can also set up Node.js and cache package manager data automatically, while the GitLab CI/CD template uses the Node.js runtime and cache configuration provided by the job.

## setup-vp Versioning

Set `<setup-vp-version>` in each example to an exact version from the [`setup-vp` releases page](https://github.com/voidzero-dev/setup-vp/releases). You can use a commit SHA instead. Do not use the `v1` tag. The `v1` tag no longer receives updates.

### Automatic Version Updates

Dependabot and Renovate can update exact versions in GitHub Actions workflows.

To use [Dependabot version updates](https://docs.github.com/en/code-security/dependabot/dependabot-version-updates/configuring-dependabot-version-updates), add a `github-actions` entry to `.github/dependabot.yml`:

```yaml [.github/dependabot.yml]
version: 2
updates:
  - package-ecosystem: github-actions
    directory: /
    schedule:
      interval: weekly
```

Dependabot checks `uses:` entries in `.github/workflows` each week.

[Renovate's GitHub Actions manager](https://docs.renovatebot.com/modules/manager/github-actions/) detects `uses:` entries by default. You do not need a package rule for `setup-vp`.

When you use a commit SHA, add the exact release tag in a comment. Renovate uses the comment to find updates:

```yaml
- uses: voidzero-dev/setup-vp@<commit-sha> # <setup-vp-version>
```

These settings apply only to GitHub Actions workflows. For GitLab CI/CD, update both version values together.

## Dependency Update Bots

`vp create` and `vp migrate` write two npm entries that must stay on one version: the `vite-plus` dependency and the `vite` alias (`npm:@voidzero-dev/vite-plus-core@<version>`). Projects that use `vp test` also carry a `vitest` override pinned to the version bundled in Vite+. A dependency bot sees unrelated packages and updates each one in its own PR. Either PR leaves the project on a Vite+ pairing that was never published together, and `vp dev`, `vp build`, `vp preview`, and `vp test` fail on the mismatch.

Configure your bot to update these packages in one grouped PR.

### Renovate

Extend the official preset from the Vite+ repository:

```json [renovate.json]
{
  "$schema": "https://docs.renovatebot.com/renovate-schema.json",
  "extends": ["config:recommended", "github>voidzero-dev/vite-plus"]
}
```

The preset groups `vite-plus`, `@voidzero-dev/vite-plus-*`, `vitest`, and `@vitest/*` into one `vite+` PR. It also sets `minimumReleaseAge` and `schedule` on the group, so every member becomes eligible at the same time even when another preset sets a different age gate or schedule for some of them. Rules later in your configuration override these values.

### Dependabot

Add a `groups` entry for the npm ecosystem in `.github/dependabot.yml`:

```yaml [.github/dependabot.yml]
version: 2
updates:
  - package-ecosystem: npm
    directory: /
    schedule:
      interval: weekly
    groups:
      vite-plus:
        patterns:
          - vite-plus
          - '@voidzero-dev/vite-plus-*'
          - vitest
          - '@vitest/*'
```

Grouping combines updates that arrive together. A `vitest` release with no Vite+ release still produces a separate `vitest` PR ahead of the bundled version. Run `vp migrate` to realign the `vitest` pin with the bundled version.

## GitHub Actions

The GitHub Action sets up Vite+, the required Node.js version, and the package manager. This means you usually do not need separate `setup-node`, package-manager setup, or manual dependency caching steps in your workflow.

```yaml [.github/workflows/ci.yml]
- uses: voidzero-dev/setup-vp@<setup-vp-version>
  with:
    node-version: '24'
    cache: true
- run: vp install
- run: vp check
- run: vp test
- run: vp build
```

With `cache: true`, `setup-vp` handles dependency caching for you automatically.

::: tip
`setup-vp` caches package-manager data. To reuse Vite Task results across CI runs, add a separate [GitHub Actions cache for Vite Task](/guide/github-actions-cache).
:::

## GitLab CI/CD

Use the reusable `setup-vp` remote template in your GitLab CI/CD configuration. Set the remote URL and `setup-ref` to the same release tag or commit SHA:

```yaml [.gitlab-ci.yml]
include:
  - remote: 'https://raw.githubusercontent.com/voidzero-dev/setup-vp/<setup-vp-version>/gitlab/setup-vp.yml'
    inputs:
      setup-ref: '<setup-vp-version>'

test:
  extends: .setup-vp
  image: node:24
  script:
    - vp check
    - vp test
    - vp build
```

The GitLab CI/CD integration differs from the GitHub Action in a few ways:

- The template does not install Node.js. Use a Node.js image, as shown above, or otherwise provide Node.js in the job.
- Configure dependency caching with the job's GitLab [`cache`](https://docs.gitlab.com/ci/yaml/#cache) keyword.
- Use a Unix-like runner environment with Bash and either `curl` or `wget`.

For advanced configuration and the complete input reference, see the [`setup-vp` GitLab CI/CD documentation](https://github.com/voidzero-dev/setup-vp#gitlab-cicd).

## Simplifying Existing Workflows

If you are migrating an existing GitHub Actions workflow, you can often replace large blocks of Node, package-manager, and cache setup with a single `setup-vp` step.

#### Before:

```yaml [.github/workflows/ci.yml]
- uses: pnpm/action-setup@v6
  with:
    version: 11

- uses: actions/setup-node@v6
  with:
    node-version: '24'
    cache: pnpm

- run: pnpm ci && pnpm dev:setup
- run: pnpm check
- run: pnpm test
```

#### After:

```yaml [.github/workflows/ci.yml]
- uses: voidzero-dev/setup-vp@<setup-vp-version>
  with:
    node-version: '24'
    cache: true

- run: vp install && vp run dev:setup
- run: vp check
- run: vp test
```

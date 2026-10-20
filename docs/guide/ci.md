# Continuous Integration

You can use `voidzero-dev/setup-vp` to use Vite+ in CI environments.

## Overview

[`voidzero-dev/setup-vp`](https://github.com/voidzero-dev/setup-vp) provides integrations for GitHub Actions, GitLab CI/CD, and Azure Pipelines. All three install Vite+, can select the project's Node.js version, and can install project dependencies. The GitHub Action and Azure Pipelines template can also cache package-manager data automatically. GitLab CI/CD uses the job's native cache configuration and requires Node.js to start the setup runtime.

## setup-vp Versioning

Set `<setup-vp-version>` in each example to an exact version from the [`setup-vp` releases page](https://github.com/voidzero-dev/setup-vp/releases). You can use a commit SHA instead. Do not use the `v1` tag. The `v1` tag no longer receives updates.

Run `vp migrate` to replace exact `voidzero-dev/setup-vp@v1` references in GitHub Actions workflows and composite actions under `.github` with the latest exact release known to your Vite+ version. Existing exact versions and commit SHAs remain unchanged.

## Node.js During Installation

The Vite+ installer uses the latest Node.js LTS by default. With versions that support [`VP_SETUP_NODE_VERSION`](/guide/global-cli#vp-setup-node-version), `setup-vp` can use the project's version for installation too. This avoids a second Node.js download when that version supports both the global CLI and its pinned `pnpm`.

`setup-vp` selects the version before it starts the installer. An explicit Node.js version takes precedence over a version-file input. Without either input, it checks the working directory and its parents using the [project declaration order](/guide/env#node-js-selection). The integration passes the selection through `VP_SETUP_NODE_VERSION`; the installer itself does not read project declarations.

Without a selection, with Node.js management disabled, or when the selected version is incompatible, installation uses LTS. Older installers keep their existing behavior. Automatic project discovery does not create a session override, so later commands retain normal directory-based version selection.

The integration handles these installer variables automatically. Configure `node-version` or `node-version-file` in GitHub Actions and GitLab CI/CD, or `nodeVersion` or `nodeVersionFile` in Azure Pipelines. See the [`setup-vp` documentation](https://github.com/voidzero-dev/setup-vp#inputs) for the complete input reference.

## GitHub Actions

The GitHub Action sets up Vite+, the required Node.js version, and the package manager. This means you usually do not need separate `setup-node`, package-manager setup, dependency installation, or manual dependency caching steps in your workflow.

```yaml [.github/workflows/ci.yml]
- uses: voidzero-dev/setup-vp@<setup-vp-version>
  with:
    node-version: '24'
    cache: true
- run: vp check
- run: vp test
- run: vp build
```

`setup-vp` runs `vp install` by default. If you set `run-install: false`, make sure to add a `vp install` step before running other commands. With `cache: true`, `setup-vp` handles dependency caching for you automatically.

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

- The template requires Node.js to start the setup runtime. Use a Node.js image, as shown above, or otherwise provide Node.js in the job. Use `node-version` or `node-version-file` to select a managed project runtime with `vp env use`.
- Configure dependency caching with the job's GitLab [`cache`](https://docs.gitlab.com/ci/yaml/#cache) keyword.
- Use a Unix-like runner environment with Bash and either `curl` or `wget`.

For advanced configuration and the complete input reference, see the [`setup-vp` GitLab CI/CD documentation](https://github.com/voidzero-dev/setup-vp#gitlab-cicd).

## Azure Pipelines

Use the reusable `setup-vp` step template in your Azure Pipelines configuration. Create a GitHub service connection named `github`, then reference the template from the `setup-vp` repository:

```yaml [azure-pipelines.yml]
resources:
  repositories:
    - repository: setupVp
      type: github
      endpoint: github
      name: voidzero-dev/setup-vp
      ref: refs/tags/<setup-vp-version>

pool:
  vmImage: ubuntu-latest

steps:
  - checkout: self
  - template: azure/setup-vp.yml@setupVp
    parameters:
      setupRef: '<setup-vp-version>'
      nodeVersion: 24.x
      cache: true
      runInstall: true
  - script: vp check
  - script: vp test
  - script: vp build
```

Pin `ref` and `setupRef` to the same tag or commit SHA for strict reproducibility.

The Azure Pipelines template supports Microsoft-hosted Linux, macOS, and Windows agents. `UseNode@1` provides Node.js to start the setup runtime through `bootstrapNodeVersion`. The `nodeVersion` and `nodeVersionFile` parameters select the managed project runtime with `vp env use`. `Cache@2` caches package-manager data.

For advanced configuration and the complete parameter reference, see the [`setup-vp` Azure Pipelines documentation](https://github.com/voidzero-dev/setup-vp#azure-pipelines).

## Automatic Version Updates

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

These settings apply only to GitHub Actions workflows. For GitLab CI/CD and Azure Pipelines, update both version values together.

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

- run: vp run dev:setup
- run: vp check
- run: vp test
```

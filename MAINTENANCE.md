# Maintenance

## Publishing preview packages

Add the `preview-build` label to the PR. Each labeled commit is published to the
[registry bridge](https://registry-bridge.viteplus.dev/-/refs) as the npm
version `0.0.0-commit.<sha>`; the PR gets a sticky comment with the exact
version and install steps.

Install a preview build with the install script (PR number or commit sha):

```sh
curl -fsSL https://vite.plus | VP_PR_VERSION=1569 bash
```

Or pin it in a project through the bridge registry (`.npmrc`:
`registry=https://registry-bridge.viteplus.dev/`):

```sh
pnpm add vite-plus@0.0.0-commit.<sha>
```

## Publishing releases

The [release workflow](.github/workflows/release.yml) publishes platform packages before `@voidzero-dev/vite-plus-core`, then publishes `vite-plus`. It checks that the required package versions and tarballs are available before continuing to dependent steps.

The [propagation helper](.github/scripts/wait-for-npm-packages.ts) defaults to a 10-minute timeout and an additional 60-second wait for CDN propagation after availability checks pass.

### Recovering from a propagation timeout

A propagation timeout fails the release job and stops subsequent steps. This does not mean npm rejected the upload; npm may have accepted it and still be scanning the packages.

Keep the same version when resuming the release:

1. Wait until the affected package versions and tarballs are available on npm.
2. Open the failed workflow run in GitHub Actions and select **Re-run failed jobs**.

The [publish helper](.github/scripts/publish-npm-package.ts) skips versions that npm has published, and the workflow checks availability again before continuing.

# Maintenance

## Publishing Preview Packages

Add the `pkg.pr.new` label to the PR (the label name predates the registry
bridge). Each labeled commit is published to the
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

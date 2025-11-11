# Vite+

## Pull upstream dependencies

```
pnpm tools sync-remote
```

## Build Vite+ and upstream dependencies

```
just build
```

## Install internal global cli

Add the following lines to your `~/.npmrc` file:

```
//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}
@voidzero-dev:registry=https://npm.pkg.github.com/
```

Create a classic personal access token, following this guide: https://docs.github.com/en/packages/learn-github-packages/about-permissions-for-github-packages#about-scopes-and-permissions-for-package-registries

Use this token to install the global cli:

```
GITHUB_TOKEN=<your-token> npm install -g @voidzero-dev/global
```

Use 1Password cli:

```
GITHUB_TOKEN=$(op read "op://YOUR_GITHUB_TOKEN_PATH") npm install -g @voidzero-dev/global
```

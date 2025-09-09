# Vite+

## Install internal global cli

Add the following lines to your `~/.npmrc` file:

```
//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}
@voidzero-dev:registry=https://npm.pkg.github.com/
```

Create a fine-grained personal access token with the `read:packages` scope, following this guide: https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens

Use this token to install the global cli:

```
GITHUB_TOKEN=<your-token> npm install -g @voidzero-dev/global
```

Use 1Password cli:

```
GITHUB_TOKEN=$(op read "op://YOUR_GITHUB_TOKEN_PATH") npm install -g @voidzero-dev/global
```

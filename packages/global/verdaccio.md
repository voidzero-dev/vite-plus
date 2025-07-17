# Verdaccio

Install [Verdaccio][1] for actual package installs ([pkg.pr.new][2] publishes only from CI):

```sh
npm i -g verdaccio
```

Use this minimal configuration (e.g. `~/.config/verdaccio/config.yaml`):

```yaml
storage: storage
uplinks:
  npmjs:
    url: https://registry.npmjs.org/
packages:
  "**":
    access: $all
    publish: $all
    proxy: npmjs
```

Start registry:

```sh
verdaccio
```

Add dummy user:

```sh
npm adduser --registry http://localhost:4873
```

Publish any package (remove `"private": true`):

```sh
pnpm publish --registry http://localhost:4873 --tag latest --no-git-checks
```

To install `vp` globally:

```sh
npm i -g vp --registry http://localhost:4873
```

Add this to `.npmrc` to run commands without `--registry http://localhost:4873`:

```
registry=http://localhost:4873
//localhost:4873/:_authToken=fake
```

If a package is not found locally it goes to npm registry.

[1]: https://verdaccio.org
[2]: https://github.com/stackblitz-labs/pkg.pr.new
[3]: ../../.npmrc

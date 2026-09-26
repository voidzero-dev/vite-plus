# tools for internal development use

## Bins

- json-edit: A CLI tool to edit JSON files such as package.json, used by the release workflow to stamp build versions

## `tool` subcommands

Run with `tool <name>`:

- sync-remote: Sync upstream dependency sources and catalog versions from `.upstream-versions.json`
- install-global-cli: Install the local build of the global `vp` CLI. The tool
  reuses an existing `~/.vite-plus` install. Otherwise, it uses the split
  platform data directory.
- brand-vite: Apply Vite+ branding patches to the synced vite source (also runs at the end of sync-remote)
- local-npm-registry: Serve locally packed checkout packages behind a real registry HTTP interface for snapshot tests, ecosystem e2e, and local `vp migrate`/`vp create` iteration

## Local npm registry

See the [contributing guide](../../CONTRIBUTING.md#test-vp-migrate--vp-create-through-a-local-npm-registry) for local build and usage commands.

[`src/local-npm-registry.ts`](src/local-npm-registry.ts) packs the checkout, serves its tarballs through a registry HTTP interface, and proxies other packages upstream.

- Served versions use an old publish time so package-manager minimum-release-age checks allow local builds immediately.
- Wrapped commands use temporary Yarn Berry and bun caches to avoid reusing stale local builds with the same package version.
- The server also backs [PTY snapshot cases](../../crates/vp_cli_snapshots/tests/cli_snapshots/README.md) with `local-registry = true` and [ecosystem e2e tests](../../ecosystem-ci/patch-project.ts).

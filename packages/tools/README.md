# tools for internal development use

Run subcommands with `tool <name>`:

- sync-remote: Sync upstream dependency sources and catalog versions from `.upstream-versions.json`
- install-global-cli: Install the locally built `vp` global CLI into `~/.vite-plus`
- brand-vite: Apply Vite+ branding patches to the synced vite source (also runs at the end of sync-remote)
- local-npm-registry: Serve locally packed checkout packages behind a real registry HTTP interface for snapshot tests, ecosystem e2e, and local `vp migrate`/`vp create` iteration

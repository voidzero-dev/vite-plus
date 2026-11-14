# Update Vite+

Updating `vite-plus` and its related project dependencies. To upgrade the global `vp` binary, see [Upgrading Vite+](/guide/upgrade).

## Update with Migrate

The recommended way to update a project is to use `vp migrate`, which keeps the toolchain dependencies aligned.

Before updating the project's dependencies, [run the target migrator through your package manager](./migrate.md#without-the-global-cli).

If your global CLI is newer than the project's version, running `vp migrate` upgrades the project to that global version instead:

```bash
vp migrate
```

On a project that is already on Vite+, migrate does a toolchain version upgrade only: it re-pins `vite-plus`, the `vite` -> `@voidzero-dev/vite-plus-core` alias, and the `vitest` pin to the versions bundled with the CLI running the migration, across every workspace package. It skips the first-time setup steps (git hooks, editor and agent files, lint migration), so a version bump does not re-touch things you already configured. Pass `--full` to also run that setup.

## Manually Updating

Update `vite-plus` and the `vite` alias to `@voidzero-dev/vite-plus-core` together. The two packages have different versions: the core version encodes the bundled Vite version (for example, `8.3.5000` for Vite `8.3.5`). Read the alias that a `vite-plus` version uses before you install it:

```bash
npm view vite-plus@<version> dependencies.vite
# npm:@voidzero-dev/vite-plus-core@<core-version>
```

Set the `vite` alias to that value. Update these entries wherever they are declared in your workspace, including overrides or catalogs, then install dependencies to refresh the lockfile. Also [update the Vitest pin](#updating-the-vitest-pin) to match the bundled version. After the install, `vp toolchain vite-plus-core` shows the same core version.

### Dependency Bots

Dependabot and Renovate see `vite-plus` and the `vite` alias as two separate dependencies with different versions. A bot can update one without the other. Vite+ then stops before it runs a Vite command and asks you to run `vp migrate`. Before you merge a bot update, check that the `vite` alias matches the output of `npm view vite-plus@<version> dependencies.vite`. If it does not match, run `vp migrate` on the bot's branch. Migrate pins the matching alias, the overrides, and the Vitest version.

Without the global CLI, run the `vp` commands on this page through your package manager, for example `pnpm exec vp toolchain vitest`.

### Updating the Vitest Pin

If you migrated with `vp migrate`, your project pins `vitest` to an exact version so the whole project shares a single Vitest copy with the bundled `vp test` runner. The pin lives in your package manager's override block:

- **npm / Bun:** a `vitest` entry under `overrides` in `package.json`
- **Yarn:** a `vitest` entry under `resolutions` in `package.json`
- **pnpm:** a `vitest@*` entry under `overrides` in `pnpm-workspace.yaml`. If your `package.json` already has a `pnpm` field, the entry lives under `pnpm.overrides` in `package.json` instead. pnpm ignores `pnpm-workspace.yaml` overrides when `package.json` defines `pnpm.overrides`.

A Vite+ release can bump the bundled Vitest. Because that pin also applies to `vite-plus`'s own `vitest` dependency, an out-of-date pin keeps installing the previous runner even after you upgrade `vite-plus` — splitting Vitest's internals (mocks, `expect`, runner state) between the pinned copy and the one `vp test` loads.

After upgrading `vite-plus`, re-pin `vitest` to the version Vite+ now bundles. Check that version with:

```bash
vp toolchain vitest
```

Then set the `vitest` override to that exact version and reinstall dependencies.

::: details Why pnpm overrides use `@*`
Under pnpm the managed keys use an explicit `@*` range (`vite@*`, `vitest@*`). pnpm applies an override by replacing the declared spec on every manifest, importer manifests included. A bare key matches any spec, including `catalog:`. The `@*` range keeps the override on the semver ranges that transitive and peer declarations use, and leaves `catalog:` references intact. `vp up` therefore no longer rewrites them to a concrete version.
:::

## Preview Builds

After [installing a preview build of the global CLI](/guide/upgrade#global-vp-preview), run migrate in the project to move its local `vite-plus` onto the same build:

```bash
vp migrate
```

Migrate writes the bridge registry to `.npmrc`. For Yarn Berry, it writes the registry to `.yarnrc.yml`. It pins `vite-plus` and the `vite` alias to the matching `0.0.0-commit.<sha>` version. The `vite` alias points to `@voidzero-dev/vite-plus-core`. Commit the registry line if the project CI must test the preview.

After the install, run `vp toolchain` to show the selected versions. After testing, set `vite-plus` to `latest` and the `vite` alias to `npm:@voidzero-dev/vite-plus-core@latest` wherever they are declared. Remove the preview bridge setting from `.npmrc` or `.yarnrc.yml`, restoring any previous registry. Then run `vp install`.

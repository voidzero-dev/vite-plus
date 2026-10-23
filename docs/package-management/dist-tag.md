# vp pm dist-tag

Manage the tags that map package names such as `my-package@next` to a published version.

```bash
vp pm dist-tag list my-package
vp pm dist-tag add my-package@2.0.0 next
vp pm dist-tag rm my-package next
```

## Options

- `list [package]` lists tags. Alias: `ls`.
- `add <package@version> <tag>` assigns a tag.
- `rm <package> <tag>` removes a tag.

There are no additional Vite+ options or native pass-through arguments on these subcommands. Changing a tag changes what future installs of that tag resolve to; it does not delete the published version.

### `list`

There are no command-specific options.

### `add`

There are no command-specific options.

### `rm`

There are no command-specific options.

## Compatibility

### `list`

- **npm:** `npm dist-tag list`.
- **pnpm:** 11 or later: `pnpm dist-tag list`. Earlier versions use `npm dist-tag list` as a fallback.
- **Yarn Classic:** `yarn tag list`.
- **Yarn Modern:** `yarn npm tag list`.
- **Bun:** Fallback: `npm dist-tag list`.

### `add`

- **npm:** `npm dist-tag add`.
- **pnpm:** 11 or later: `pnpm dist-tag add`. Earlier versions use `npm dist-tag add` as a fallback.
- **Yarn Classic:** `yarn tag add`.
- **Yarn Modern:** `yarn npm tag add`.
- **Bun:** Fallback: `npm dist-tag add`.

### `rm`

- **npm:** `npm dist-tag rm`.
- **pnpm:** 11 or later: `pnpm dist-tag rm`. Earlier versions use `npm dist-tag rm` as a fallback.
- **Yarn Classic:** `yarn tag rm`.
- **Yarn Modern:** `yarn npm tag rm`.
- **Bun:** Fallback: `npm dist-tag rm`.

On the native pnpm path, Vite+ does not fill in a missing package for `list`, resolve version ranges for `add`, or bypass restrictions on removing `latest`. pnpm validates these requests and returns its own errors.

## Native Options

For native options, see [npm dist-tag](https://docs.npmjs.com/cli/v11/commands/npm-dist-tag), [pnpm dist-tag](https://pnpm.io/cli/dist-tag), [Yarn npm tag list](https://yarnpkg.com/cli/npm/tag/list), [Yarn npm tag add](https://yarnpkg.com/cli/npm/tag/add), [Yarn npm tag remove](https://yarnpkg.com/cli/npm/tag/remove), and [Yarn Classic tag](https://classic.yarnpkg.com/en/docs/cli/tag). The upstream commands may expose additional options not accepted by this Vite+ interface.

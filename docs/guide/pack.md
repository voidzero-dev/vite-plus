# Pack

`vp pack` builds libraries for production with [tsdown](https://tsdown.dev/guide/).

## Overview

`vp pack` builds libraries and standalone executables with tsdown. Use it for publishable packages and binary outputs. If you want to build a web application, use `vp build`. `vp pack` covers everything you need for building libraries out of the box, including declaration file generation, multiple output formats, source maps, and minification.

For more information about how tsdown works, see the official [tsdown guide](https://tsdown.dev/guide/).

## Usage

```bash
vp pack
vp pack src/index.ts --dts
vp pack --watch
```

## Configuration

Put packaging configuration directly in the `pack` block in `vite.config.ts` so all your configuration stays in one place. We do not recommend using `tsdown.config.ts` with Vite+.

See the [tsdown guide](https://tsdown.dev/guide/) and the [tsdown config file docs](https://tsdown.dev/options/config-file) to learn more about how to use and configure `vp pack`.

Use it for:

- [declaration files (`dts`)](https://tsdown.dev/options/dts)
- [output formats](https://tsdown.dev/options/output-format)
- [watch mode](https://tsdown.dev/options/watch-mode)
- [standalone executables](https://tsdown.dev/options/exe#executable)

```ts
import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: {
    dts: true,
    format: ['esm', 'cjs'],
    sourcemap: true,
  },
});
```

## Standalone Executables

`vp pack` can also build standalone executables through tsdown's experimental [`exe` option](https://tsdown.dev/options/exe#executable).

Use this when you want to ship a CLI or other Node-based tool as a native executable that runs without requiring Node.js to be installed separately.

```ts
import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: {
    entry: ['src/cli.ts'],
    exe: true,
  },
});
```

See the official [tsdown executable docs](https://tsdown.dev/options/exe#executable) for details about configuring custom file names, embedded assets, and cross-platform targets.

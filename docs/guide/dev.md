# Dev

`vp dev` starts the Vite development server.

## Overview

`vp dev` runs the standard Vite development server through Vite+, so you keep the normal Vite dev experience while using the same CLI entry point as the rest of the toolchain. For more information about using and configuring the dev server, see the [Vite guide](https://vite.dev/guide/).

::: info
`vp dev` always runs the built-in Vite dev server. If your project also has a `dev` script in `package.json`, run `vp run dev` when you want to run that script instead. See [Built-in Commands vs Scripts](/guide/run#built-in-commands-vs-scripts).
:::

## Usage

```bash
vp dev
```

## Configuration

Use standard Vite config in `vite.config.ts`. For the full configuration reference, see the [Vite config docs](https://vite.dev/config/).

Use it for:

- [plugins](https://vite.dev/guide/using-plugins)
- [aliases](https://vite.dev/config/shared-options#resolve-alias)
- [`server`](https://vite.dev/config/server-options)
- [environment modes](https://vite.dev/guide/env-and-mode)

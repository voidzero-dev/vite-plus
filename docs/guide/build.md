# Build

`vp build` builds Vite applications for production.

## Overview

`vp build` runs the standard Vite production build through Vite+. Since it is directly based on Vite, the build pipeline and configuration model are the same as Vite. For more information about how Vite production builds work, see the [Vite guide](https://vite.dev/guide/build). Note that Vite+ uses Vite 8 and [Rolldown](https://rolldown.rs/) for builds.

::: info
`vp build` always runs the built-in Vite production build. If your project also has a `build` script in `package.json`, run `vp run build` when you want to run that script instead.
:::

## Usage

```bash
vp build
vp build --watch
vp build --sourcemap
```

## Configuration

Use standard Vite configuration in `vite.config.ts`. For the full configuration reference, see the [Vite config docs](https://vite.dev/config/).

Use it for:

- [plugins](https://vite.dev/guide/using-plugins)
- [aliases](https://vite.dev/config/shared-options#resolve-alias)
- [`build`](https://vite.dev/config/build-options)
- [`preview`](https://vite.dev/config/preview-options)
- [environment modes](https://vite.dev/guide/env-and-mode)

## Preview

Use `vp preview` to serve the production build locally after `vp build`.

```bash
vp build
vp preview
```

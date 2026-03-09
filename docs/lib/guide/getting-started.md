# Getting Started

## Overview

Vite+ is the unified toolchain and entry point for modern web application development. It manages your runtime, package manager, and frontend toolchain in one place by combining [Vite](https://vite.dev/), [Vitest](https://vitest.dev/), [Oxlint](https://oxc.rs/docs/guide/usage/linter.html), [Oxfmt](https://oxc.rs/docs/guide/usage/formatter.html), [Rolldown](https://rolldown.rs/), [tsdown](https://tsdown.dev/), and [Vite Task](https://github.com/voidzero-dev/vite-task):

- **Runtime Management:** Manage Node.js globally and per-project with `vp env`
- **Package Management:** Install and manage dependencies with `vp install` and related commands
- **Dev Server:** Run Vite's native ESM dev server with `vp dev`
- **Code Health:** Run linting, formatting, and type checks with Oxlint, Oxfmt, and `tsgo` via `vp check`
- **Testing:** Run tests with bundled Vitest via `vp test`
- **Build & Pack:** Build apps with `vp build` and build libraries or standalone app binaries with `vp pack`
- **Task Runner:** Execute monorepo tasks with `vp run` and automated caching/dependency resolution

All in a single, cohesive tool designed for scale, speed, and developer sanity.
Vite+ is fully open-source under the MIT license.

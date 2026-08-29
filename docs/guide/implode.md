# Removing Vite+

Use `vp implode` to remove the [global `vp` installation](/guide/global-cli) and all related Vite+ data from your machine. It does not remove `vite-plus` dependencies from projects.

## Overview

`vp implode` is the cleanup command for removing a Vite+ installation and its managed data. Use it if you no longer want Vite+ to manage your runtime, package manager, and related local tooling state.

::: info
If you decide Vite+ is not for you, please [share your feedback with us](https://discord.gg/cAnsqHh5PX).
:::

## Usage

```bash
vp implode
```

Skip the confirmation prompt with:

```bash
vp implode --yes
```

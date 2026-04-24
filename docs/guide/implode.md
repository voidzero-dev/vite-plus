# Removing Vite+

Use `vp implode` to remove `vp` and all related Vite+ data from your machine.

## Overview

`vp implode` is the cleanup command for removing a Vite+ installation and its managed data. Use it if you no longer want Vite+ to manage your runtime, package manager, and related local tooling state.

If you installed `vite-plus` globally with npm or pnpm, `vp implode` does not remove that package-manager installation. In that case, uninstall it with the same package manager you used to install it:

```bash
npm uninstall -g vite-plus
pnpm remove -g vite-plus
```

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

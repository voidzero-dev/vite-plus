# Removing Vite+

Use `vp implode` to remove the Vite+-managed [global `vp` installation](/guide/global-cli) and related user data from your machine. It does not remove `vite-plus` dependencies from projects or packages owned by Homebrew.

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

## Homebrew

For the [official Homebrew tap](/guide/homebrew), run `vp implode` first to remove Vite+-managed runtimes, global packages, configuration, shims, and shell entries. Then remove the Homebrew package:

```bash
vp implode
brew uninstall voidzero-dev/vite-plus/vp
```

The confirmation prompt explains that the Homebrew package will remain installed. After cleanup, `vp implode` shows the uninstall command for the owning formula.

Restart your terminal before you run `vp` again. In Bash, you can run `hash -r` instead to clear cached command paths. If the Homebrew package is still installed, the next `vp` command starts first-run setup again.

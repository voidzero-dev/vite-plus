# command_self_setup_homebrew_sources

## `node verify-refresh.mjs homebrew-sources`

```
core installation
    CLI source        Homebrew Core
    CLI formula       vite-plus
error: Upgrade error: Homebrew Core manages this installation. Run `brew upgrade vite-plus` to update it.

To switch to the official Vite+ tap while keeping your settings:

  brew uninstall vite-plus
  brew tap voidzero-dev/vite-plus https://github.com/voidzero-dev/vite-plus
  brew install voidzero-dev/vite-plus/vp
  "$(brew --prefix)/bin/vp" env setup --refresh
  hash -r
info: Homebrew Core manages this installation. Run `brew outdated vite-plus` to check for updates.

To switch to the official Vite+ tap while keeping your settings:

  brew uninstall vite-plus
  brew tap voidzero-dev/vite-plus https://github.com/voidzero-dev/vite-plus
  brew install voidzero-dev/vite-plus/vp
  "$(brew --prefix)/bin/vp" env setup --refresh
  hash -r
The Homebrew Core package remains installed. Run `brew uninstall vite-plus` to remove it.
official installation
    CLI source        Vite+ Homebrew tap
    CLI formula       voidzero-dev/vite-plus/vp
error: Upgrade error: Vite+ Homebrew tap manages this installation. Run `brew upgrade voidzero-dev/vite-plus/vp` to update it.
info: Vite+ Homebrew tap manages this installation. Run `brew outdated voidzero-dev/vite-plus/vp` to check for updates.
The Vite+ Homebrew tap package remains installed. Run `brew uninstall voidzero-dev/vite-plus/vp` to remove it.
other installation
    CLI source        Homebrew
    CLI formula       example/tools/vp
error: Upgrade error: Homebrew manages this installation. Run `brew upgrade example/tools/vp` to update it.
info: Homebrew manages this installation. Run `brew outdated example/tools/vp` to check for updates.
The Homebrew package remains installed. Run `brew uninstall example/tools/vp` to remove it.
unknown installation
    CLI source        Homebrew
    CLI formula       vite-plus
error: Upgrade error: Homebrew manages this installation. Run `brew upgrade vite-plus` to update it.
info: Homebrew manages this installation. Run `brew outdated vite-plus` to check for updates.
The Homebrew package remains installed. Run `brew uninstall vite-plus` to remove it.
```

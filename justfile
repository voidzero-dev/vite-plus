#!/usr/bin/env -S just --justfile

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]
set shell := ["bash", "-cu"]

_default:
  @just --list -u

alias r := ready

[unix]
_clean_dist:
  rm -rf packages/*/dist

[windows]
_clean_dist:
  Remove-Item -Path 'packages/*/dist' -Recurse -Force -ErrorAction SilentlyContinue

init: _clean_dist _fix_symlinks
  cargo binstall watchexec-cli cargo-insta typos-cli cargo-shear dprint taplo-cli -y
  node packages/tools/src/index.ts sync-remote
  pnpm install
  pnpm -C docs install

[unix]
_fix_symlinks:
  #!/usr/bin/env bash
  if [ "$(git config --get core.symlinks)" != "true" ]; then \
    echo "Enabling core.symlinks and re-checking out symlinks..."; \
    git config core.symlinks true; \
    git ls-files -s | grep '^120000' | cut -f2 | while read -r f; do git checkout -- "$f"; done; \
  fi

[windows]
_fix_symlinks:
  $symlinks = git config --get core.symlinks; \
  if ($symlinks -ne 'true') { \
    Write-Host 'Enabling core.symlinks and re-checking out symlinks...'; \
    git config core.symlinks true; \
    git ls-files -s | Where-Object { $_ -match '^120000' } | ForEach-Object { ($_ -split "`t", 2)[1] } | ForEach-Object { git checkout -- $_ }; \
  }

build:
  pnpm install
  pnpm build

ready:
  git diff --exit-code --quiet
  typos
  just fmt
  just check
  just test
  just lint
  just doc

watch *args='':
  watchexec --no-vcs-ignore {{args}}

fmt:
  cargo shear --fix
  cargo fmt --all
  cargo fmt --manifest-path crates/vp_trampoline/Cargo.toml
  pnpm fmt

check:
  cargo check --workspace --all-features --all-targets --locked

watch-check:
  just watch "'cargo check; cargo clippy'"

# Test all crates/* packages (new crates are automatically included) plus
# vite-plus-cli (lives outside crates/) to catch type sync issues.
# vp_cli_snapshots is excluded: its suite needs a built global binary and
# node, and runs via `just snapshot-test` instead.
# vp_trampoline is excluded from the workspace and tests from its own
# directory (build-std). Its only test module is unix-only, so the Windows
# recipe skips it instead of paying for a no-test build-std compile.
# Single source of truth for cargo test, used by CI too.
[unix]
test:
  RUST_MIN_STACK=8388608 cargo test $(for d in crates/*/; do n=$(basename $d); [ "$n" = "vp_cli_snapshots" ] || [ "$n" = "vp_trampoline" ] || echo -n "-p $n "; done) -p vite-plus-cli
  cd crates/vp_trampoline && cargo test

[windows]
test:
  $packages = Get-ChildItem -Path crates -Directory | Where-Object { $_.Name -ne 'vp_cli_snapshots' -and $_.Name -ne 'vp_trampoline' } | ForEach-Object { '-p'; $_.Name }; $Env:RUST_MIN_STACK='8388608'; $Env:__COMPAT_LAYER='RunAsInvoker'; cargo test @packages -p vite-plus-cli

# PTY-based CLI snapshot tests (crates/vp_cli_snapshots). Builds the global
# binary and shim template first so the runner never tests a stale build, and
# installs Playwright Chromium for the browser-mode cases (idempotent).
# Filter by trial name substring: `just snapshot-test create`. Accept snapshot changes with
# `UPDATE_SNAPSHOTS=1 just snapshot-test`. Local-flavor cases additionally
# need a built packages/cli (`pnpm build`); the runner fails fast when dist
# is missing or stale. Use snapshot-test-global on checkouts without one.
snapshot-test *args='': _install_chromium _build-trampoline
  cargo build -p vp_global_cli
  cargo test -p vp_cli_snapshots -- {{args}}

# The trampoline is excluded from the workspace; build it from its own
# directory so its .cargo/config.toml (build-std) applies. Artifacts still
# land in the repo-root target/ directory.
[unix]
_build-trampoline:
  cd crates/vp_trampoline && cargo build

[windows]
_build-trampoline:
  Set-Location crates/vp_trampoline; cargo build

# Browser-mode snapshot cases run with PLAYWRIGHT_BROWSERS_PATH=0, so the
# browser must be installed into node_modules with the same setting.
[unix]
_install_chromium:
  PLAYWRIGHT_BROWSERS_PATH=0 pnpm exec playwright install chromium

[windows]
_install_chromium:
  $Env:PLAYWRIGHT_BROWSERS_PATH='0'; pnpm exec playwright install chromium

# Global flavor + vpt cases only: needs no JS build, for Rust-side work on
# a checkout that never ran `pnpm build`.
[unix]
snapshot-test-global *args='':
  VP_SNAP_SKIP_FLAVORS=local just snapshot-test {{args}}

[windows]
snapshot-test-global *args='':
  $Env:VP_SNAP_SKIP_FLAVORS='local'; just snapshot-test {{args}}

# Single source of truth for clippy, used by CI too. The `-A` flags allow
# new toolchain lints that fire in upstream rolldown crates without a `[lints]` table.
lint:
  cargo clippy --workspace --all-targets --all-features -- --deny warnings \
    -A clippy::byte_char_slices \
    -A clippy::manual_assert_eq \
    -A clippy::needless_return_with_question_mark \
    -A clippy::redundant_else \
    -A clippy::unused_async_trait_impl \
    -A clippy::useless_borrows_in_formatting
  cargo clippy --manifest-path crates/vp_trampoline/Cargo.toml --all-targets -- --deny warnings

[unix]
doc:
  RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --document-private-items

[windows]
doc:
  $Env:RUSTDOCFLAGS='-D warnings'; cargo doc --no-deps --document-private-items

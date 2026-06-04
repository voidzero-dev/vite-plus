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
  pnpm fmt

check:
  cargo check --workspace --all-features --all-targets --locked

watch-check:
  just watch "'cargo check; cargo clippy'"

[unix]
test:
  RUST_MIN_STACK=8388608 cargo test $(for d in crates/*/; do echo -n "-p $(basename $d) "; done) -p vite-plus-cli

[windows]
test:
  $packages = Get-ChildItem -Path crates -Directory | ForEach-Object { '-p'; $_.Name }; $Env:__COMPAT_LAYER='RunAsInvoker'; cargo test @packages -p vite-plus-cli

lint:
  cargo clippy --workspace --all-targets --all-features -- --deny warnings

[unix]
doc:
  RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --document-private-items

[windows]
doc:
  $Env:RUSTDOCFLAGS='-D warnings'; cargo doc --no-deps --document-private-items

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

init: _clean_dist
  cargo binstall watchexec-cli cargo-insta typos-cli cargo-shear dprint taplo-cli -y
  node packages/tools/src/index.ts sync-remote --clone-only
  pnpm install
  pnpm -C docs install

build:
  pnpm install
  pnpm --filter @rolldown/pluginutils build
  pnpm --filter rolldown build-binding:release
  pnpm --filter rolldown build-node
  pnpm --filter vite build-types
  pnpm --filter=@voidzero-dev/vite-plus-core build
  pnpm --filter=@voidzero-dev/vite-plus-test build
  pnpm --filter=@voidzero-dev/vite-plus-prompts build
  node packages/tools/src/with-curl-http1.ts pnpm --filter=vite-plus build

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

test:
  cargo test

lint:
  cargo clippy --workspace --all-targets --all-features -- --deny warnings

[unix]
doc:
  RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --document-private-items

[windows]
doc:
  $Env:RUSTDOCFLAGS='-D warnings'; cargo doc --no-deps --document-private-items

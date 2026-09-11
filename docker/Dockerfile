# syntax=docker/dockerfile:1
#
# Official Vite+ toolchain image.
#
# Bundles the `vp` CLI for the build, CI, and development phases. This is NOT a
# production runtime image: it ships the full toolchain (vite, rolldown, vitest,
# oxlint, ...) and is meant for use as a build stage, CI image, or devcontainer.
#
# For production, use the documented multi-stage pattern (see docs/guide/docker.md)
# where this image builds the app and the exact Node.js resolved from
# `.node-version` is copied into a small, vp-free runtime stage.

FROM debian:bookworm-slim

LABEL org.opencontainers.image.source="https://github.com/voidzero-dev/vite-plus" \
      org.opencontainers.image.description="Vite+ toolchain image (vp CLI) for build, CI, and development" \
      org.opencontainers.image.licenses="MIT"

# Version of vp to install. Override at build time:
#   docker build --build-arg VP_VERSION=1.4.2 .
ARG VP_VERSION=latest

# Optional: build a preview image from a registry bridge build instead of npm.
# Set to a PR number or commit SHA; when set it overrides VP_VERSION.
#   docker build --build-arg VP_PR_VERSION=1569 .
ARG VP_PR_VERSION=

# Toolchain image: include git (+ openssh-client for git+ssh dependencies) and a
# C/C++ build toolchain so native addons (for example better-sqlite3 or sharp)
# can compile during `vp install`. `sudo` provides the non-root vp user an
# escape hatch to root (see the sudoers entry below).
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates \
      curl \
      git \
      openssh-client \
      build-essential \
      python3 \
      pkg-config \
      sudo \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --create-home --shell /bin/bash vp \
 # Own /app for the vp user so downstream `WORKDIR /app` + `vp install` can write.
 && mkdir -p /app \
 && chown vp:vp /app \
 # Grant the non-root vp user passwordless sudo. The image defaults to non-root
 # (bind-mounted files stay owned by uid 1000, and Chromium keeps its sandbox),
 # but build/CI/dev work still needs occasional root: installing extra apt
 # packages, and `playwright install --with-deps` for Vitest browser mode. When
 # not already root, Playwright runs the apt-get step via `sudo` if present and
 # otherwise falls back to `su root`, which fails here (the vp user has no
 # password). Providing NOPASSWD sudo lets that path succeed. This mirrors the
 # devcontainer base images (non-root user + NOPASSWD sudo).
 && echo 'vp ALL=(root) NOPASSWD:ALL' > /etc/sudoers.d/vp \
 && chmod 0440 /etc/sudoers.d/vp

# Run as a non-root user by default. Unlike a production runtime image, this is a
# build/CI/dev image, so the vp user also has passwordless sudo (above) for the
# root work those phases occasionally need.
USER vp

ENV VP_HOME=/home/vp/.vite-plus \
    PATH=/home/vp/.vite-plus/bin:$PATH

# Install the vp global CLI. The installer downloads the platform package from
# npm (or from the registry bridge when VP_PR_VERSION is set). Node.js itself is
# provisioned per-project by vp at build time, honoring `.node-version` /
# `engines.node` / `devEngines.runtime`.
#
# The installer pre-provisions a default Node.js (~190 MB). Drop it: each project
# downloads its own pinned Node.js at build time, so the default is dead weight in a
# builder image. The node/npm/npx shims remain and fetch the right version on
# first use.
RUN curl -fsSL https://vite.plus | VP_VERSION="${VP_VERSION}" VP_PR_VERSION="${VP_PR_VERSION}" bash \
 && vp --version \
 && rm -rf "$VP_HOME/js_runtime"

WORKDIR /app

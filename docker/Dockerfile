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

# Toolchain image: include git and a C/C++ build toolchain so native addons
# (for example better-sqlite3 or sharp) can compile during `vp install`.
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates \
      curl \
      git \
      xz-utils \
      build-essential \
      python3 \
      pkg-config \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --create-home --shell /bin/bash vp \
 && mkdir -p /app \
 && chown vp:vp /app

# Run as a non-root user by default (mirrors oven/bun's `bun` and Deno's `deno`).
USER vp

ENV VP_HOME=/home/vp/.vite-plus
ENV PATH=/home/vp/.vite-plus/bin:$PATH

# Install the vp global CLI. The installer downloads the platform package from
# npm. Node.js itself is provisioned per-project by vp at build time, honoring
# `.node-version` / `engines.node` / `devEngines.runtime`.
RUN curl -fsSL https://vite.plus | VP_VERSION="${VP_VERSION}" bash \
 && vp --version

WORKDIR /app

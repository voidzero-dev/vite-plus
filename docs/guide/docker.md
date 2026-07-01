# Docker

Vite+ publishes an official Docker image that bundles the `vp` CLI for the
**build, CI, and development** phases.

```
ghcr.io/voidzero-dev/vite-plus
```

The image is a toolchain image, not a production runtime image. Because `vp`
already reads your project's pinned Node.js version (`.node-version`,
`engines.node`, or `devEngines.runtime`) and downloads that exact version, you do
not need a base image pinned to a specific Node.js version: one image builds any
project against its own Node.js.

For production, you do not ship this image. Instead you use a multi-stage build
where this image builds the app, and the exact Node.js it resolved is copied into
a small, vp-free runtime image. That keeps the deployed image small while
matching your project's Node.js version exactly.

## Image tags

Tags track the `vp` version:

| Tag                                                      | Meaning        |
| -------------------------------------------------------- | -------------- |
| `ghcr.io/voidzero-dev/vite-plus:latest`                  | Latest release |
| `ghcr.io/voidzero-dev/vite-plus:<major>`                 | Latest major   |
| `ghcr.io/voidzero-dev/vite-plus:<major>.<minor>`         | Latest minor   |
| `ghcr.io/voidzero-dev/vite-plus:<major>.<minor>.<patch>` | Exact version  |

The examples use `:latest` to track the newest release; pin an exact tag or a
digest if you need reproducible builds. The image is published for `linux/amd64`
and `linux/arm64` and runs as a non-root user by default.

Browse all published versions and digests on the [GitHub package page](https://github.com/voidzero-dev/vite-plus/pkgs/container/vite-plus).

## Production: SSR / Node.js server app

For apps that run Node.js in production (SvelteKit, Nuxt, a custom Vite SSR
server, and so on), build with the toolchain image and copy the resolved Node.js
and the built app into a slim runtime stage:

```dockerfile [Dockerfile]
# syntax=docker/dockerfile:1

# --- build stage: the official Vite+ toolchain image ---
FROM ghcr.io/voidzero-dev/vite-plus:latest AS build
WORKDIR /app

# Install dependencies first so this layer is cached across source changes.
COPY --chown=vp:vp package.json pnpm-lock.yaml .node-version* ./
RUN vp install --frozen-lockfile

# Build. vp reads .node-version and provisions that exact Node.js automatically.
COPY --chown=vp:vp . .
RUN vp build

# Export the exact resolved Node.js binary for the runtime stage.
RUN cp "$(vp env which node | head -1)" /tmp/node

# --- deps stage: production-only dependencies ---
# A separate, fresh `--prod` install so devDependencies (including the vite-plus
# toolchain) are excluded. Running `--prod` over the full install above would not
# prune the already-installed devDependencies.
FROM ghcr.io/voidzero-dev/vite-plus:latest AS deps
WORKDIR /app
COPY --chown=vp:vp package.json pnpm-lock.yaml .node-version* ./
RUN vp install --frozen-lockfile --prod

# --- runtime stage: small, glibc, no vp ---
FROM debian:bookworm-slim AS runtime
WORKDIR /app
ENV NODE_ENV=production

# The exact Node.js from .node-version (official, signature-verified build).
COPY --from=build /tmp/node /usr/local/bin/node

COPY --from=build /app/dist ./dist
COPY --from=deps /app/node_modules ./node_modules
COPY --from=build /app/package.json ./

USER nobody
EXPOSE 3000
CMD ["node", "dist/server.js"]
```

The deployed image contains only Node.js plus your app and production
dependencies, and matches `.node-version` exactly. It is much smaller than the
default `node:*` image; see the distroless tip below for the smallest result.

::: warning Prune production dependencies in a separate stage
Install production dependencies in their own `deps` stage as shown. Running
`vp install --prod` after a full `vp install` in the same stage does not remove
the already-installed devDependencies, so the `vite-plus` toolchain would be
copied into the runtime image. If your server bundle is fully self-contained (no
un-bundled runtime dependencies), you can skip copying `node_modules` entirely.
:::

::: tip Smaller still
For a shell-less, minimal-CVE runtime, swap the runtime base for distroless
(`gcr.io/distroless/cc`) and keep an `ENTRYPOINT` in vector form. It is glibc
based, so the copied Node.js binary remains compatible.
:::

## Production: static SPA / SSG

A static site needs no Node.js at runtime; serve the build output with any static
server:

```dockerfile [Dockerfile]
FROM ghcr.io/voidzero-dev/vite-plus:latest AS build
WORKDIR /app
COPY --chown=vp:vp package.json pnpm-lock.yaml .node-version* ./
RUN vp install --frozen-lockfile
COPY --chown=vp:vp . .
RUN vp build

FROM nginx:alpine AS runtime
COPY --from=build /app/dist /usr/share/nginx/html
```

## Continuous integration

Use the image directly in container-based CI (GitLab CI, Buildkite, CircleCI,
Jenkins, and others):

```yaml [.gitlab-ci.yml]
build:
  image: ghcr.io/voidzero-dev/vite-plus:latest
  script:
    - vp install --frozen-lockfile
    - vp check
    - vp test
    - vp build
```

On GitHub Actions, prefer [`setup-vp`](./ci) instead of the image.

## Devcontainers

Use the image as a ready-to-go development container with the toolchain
preinstalled:

```jsonc [.devcontainer/devcontainer.json]
{
  "image": "ghcr.io/voidzero-dev/vite-plus:latest",
}
```

## Ad-hoc usage

Run any `vp` command against a project without installing vp on your machine:

```bash
docker run --rm -it -v "$PWD:/app" -w /app ghcr.io/voidzero-dev/vite-plus vp build
```

## Notes

- **Node.js version**: provisioned from `.node-version`, `engines.node`, or
  `devEngines.runtime` at build time, so there is no Node.js-specific image tag. The
  dependency `COPY` uses a `.node-version*` glob so the file is optional: projects
  that pin via `engines.node`/`devEngines.runtime` need no `.node-version`, and
  those that use one have it available in every stage.
- **Non-root user**: the image runs as the non-root `vp` user, so copy sources
  with `COPY --chown=vp:vp ...` as shown. Without it, `COPY` writes root-owned
  files that `vp install` cannot update (permission denied).
- **Native addons**: the image includes a C/C++ build toolchain (`build-essential`,
  `python3`), so native dependencies such as `better-sqlite3` compile during
  `vp install`.
- **glibc**: the image is glibc based so it uses the official, signature-verified
  Node.js builds.
- **Custom base image**: to add `vp` to your own base image instead, run the
  installer: `curl -fsSL https://vite.plus | bash` (set `VP_VERSION` to pin a
  version).

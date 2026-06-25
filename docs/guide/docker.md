# Docker

Vite+ publishes an official Docker image that bundles the `vp` CLI for the
**build, CI, and development** phases.

```
ghcr.io/voidzero-dev/vite-plus
```

The image is a toolchain image, not a production runtime image. Because `vp`
already reads your project's pinned Node.js version (`.node-version`,
`engines.node`, or `devEngines.runtime`) and downloads that exact version, you do
not need a Node-version-specific base image: one image builds any project against
its own Node.

For production, you do not ship this image. Instead you use a multi-stage build
where this image builds the app, and the exact Node.js it resolved is copied into
a small, vp-free runtime image. That keeps the deployed image small while
matching your project's Node version exactly.

## Image tags

Tags track the `vp` version:

| Tag                                     | Meaning        |
| --------------------------------------- | -------------- |
| `ghcr.io/voidzero-dev/vite-plus:latest` | Latest release |
| `ghcr.io/voidzero-dev/vite-plus:1`      | Latest 1.x     |
| `ghcr.io/voidzero-dev/vite-plus:1.4`    | Latest 1.4.x   |
| `ghcr.io/voidzero-dev/vite-plus:1.4.2`  | Exact version  |

Pin an exact tag (or a digest) for reproducible builds. The image is published
for `linux/amd64` and `linux/arm64` and runs as a non-root user by default.

## Production: SSR / Node-server app

For apps that run Node.js in production (SvelteKit, Nuxt, a custom Vite SSR
server, and so on), build with the toolchain image and copy the resolved Node.js
and the built app into a slim runtime stage:

```dockerfile [Dockerfile]
# syntax=docker/dockerfile:1

# --- build stage: the official Vite+ toolchain image ---
FROM ghcr.io/voidzero-dev/vite-plus:1 AS build
WORKDIR /app

# Install dependencies first so this layer is cached across source changes.
COPY package.json pnpm-lock.yaml .node-version ./
RUN vp install --frozen-lockfile

# Build. vp reads .node-version and provisions that exact Node.js automatically.
COPY . .
RUN vp build

# Stage production-only dependencies and the exact resolved Node.js binary.
RUN vp install --frozen-lockfile --prod \
 && cp "$(vp env which node | head -1)" /tmp/node

# --- runtime stage: small, glibc, no vp ---
FROM debian:bookworm-slim AS runtime
WORKDIR /app
ENV NODE_ENV=production

# The exact Node.js from .node-version (official, signature-verified build).
COPY --from=build /tmp/node /usr/local/bin/node

COPY --from=build /app/dist ./dist
COPY --from=build /app/node_modules ./node_modules
COPY --from=build /app/package.json ./

USER nobody
EXPOSE 3000
CMD ["node", "dist/server.js"]
```

The deployed image contains only Node.js plus your app, matches `.node-version`
exactly, and is smaller than a full `node:*` base image.

::: tip Smaller still
For a shell-less, minimal-CVE runtime, swap the runtime base for distroless
(`gcr.io/distroless/cc`) and keep an `ENTRYPOINT` in vector form. It is glibc
based, so the copied Node.js binary remains compatible.
:::

## Production: static SPA / SSG

A static site needs no Node.js at runtime; serve the build output with any static
server:

```dockerfile [Dockerfile]
FROM ghcr.io/voidzero-dev/vite-plus:1 AS build
WORKDIR /app
COPY package.json pnpm-lock.yaml .node-version ./
RUN vp install --frozen-lockfile
COPY . .
RUN vp build

FROM nginx:alpine AS runtime
COPY --from=build /app/dist /usr/share/nginx/html
```

## Continuous integration

Use the image directly in container-based CI (GitLab CI, Buildkite, CircleCI,
Jenkins, and others):

```yaml [.gitlab-ci.yml]
build:
  image: ghcr.io/voidzero-dev/vite-plus:1
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
  "image": "ghcr.io/voidzero-dev/vite-plus:1",
}
```

## Ad-hoc usage

Run any `vp` command against a project without installing vp on your machine:

```bash
docker run --rm -it -v "$PWD:/app" -w /app ghcr.io/voidzero-dev/vite-plus vp build
```

## Notes

- **Node.js version**: the image provisions the version from `.node-version` /
  `engines.node` / `devEngines.runtime` at build time. There is no need to pick a
  Node-specific image tag.
- **Native addons**: the image includes a C/C++ build toolchain (`build-essential`,
  `python3`), so native dependencies such as `better-sqlite3` compile during
  `vp install`.
- **glibc**: the image is glibc based so it can use the official,
  signature-verified Node.js builds. An Alpine/musl variant is not currently
  provided.
- **Custom base image**: to add `vp` to your own base image instead, run the
  installer: `curl -fsSL https://vite.plus | bash` (set `VP_VERSION` to pin a
  version).

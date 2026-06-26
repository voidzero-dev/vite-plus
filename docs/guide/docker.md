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

| Tag                                                      | Meaning        |
| -------------------------------------------------------- | -------------- |
| `ghcr.io/voidzero-dev/vite-plus:latest`                  | Latest release |
| `ghcr.io/voidzero-dev/vite-plus:<major>`                 | Latest major   |
| `ghcr.io/voidzero-dev/vite-plus:<major>.<minor>`         | Latest minor   |
| `ghcr.io/voidzero-dev/vite-plus:<major>.<minor>.<patch>` | Exact version  |

Pin an exact tag (or a digest) for reproducible builds. The image is published
for `linux/amd64` and `linux/arm64` and runs as a non-root user by default.

Browse all published versions and digests on the [GitHub package page](https://github.com/voidzero-dev/vite-plus/pkgs/container/vite-plus).

The default image is Debian (glibc). An Alpine (musl) variant is published under
the same versions with an `-alpine` suffix (`:latest-alpine`,
`:<major>-alpine`, and so on). See [Alpine variant](#alpine-musl-variant) for
when to use it and its tradeoffs.

## Production: SSR / Node-server app

For apps that run Node.js in production (SvelteKit, Nuxt, a custom Vite SSR
server, and so on), build with the toolchain image and copy the resolved Node.js
and the built app into a slim runtime stage:

```dockerfile [Dockerfile]
# syntax=docker/dockerfile:1

# --- build stage: the official Vite+ toolchain image ---
FROM ghcr.io/voidzero-dev/vite-plus:latest AS build
WORKDIR /app

# Install dependencies first so this layer is cached across source changes.
COPY --chown=vp:vp package.json pnpm-lock.yaml ./
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
COPY --chown=vp:vp package.json pnpm-lock.yaml ./
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
COPY --chown=vp:vp package.json pnpm-lock.yaml ./
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

## Alpine (musl) variant

The `-alpine` tags are a musl build for teams that standardize on Alpine. They
produce the smallest runtime image, but come with tradeoffs:

- Vite+ installs Node.js from the **unofficial musl builds**
  (`unofficial-builds.nodejs.org`), which are **not PGP-signed** (the Debian
  image uses the official, signature-verified builds).
- Some native addons need musl prebuilds or source compilation.
- A musl Node.js binary only runs on a musl base, so the runtime stage must also
  be Alpine (not `debian:bookworm-slim` or distroless).

Prefer the default Debian image unless you specifically need Alpine. The SSR
pattern with an Alpine runtime:

```dockerfile [Dockerfile]
# syntax=docker/dockerfile:1

FROM ghcr.io/voidzero-dev/vite-plus:latest-alpine AS build
WORKDIR /app
COPY --chown=vp:vp package.json pnpm-lock.yaml ./
RUN vp install --frozen-lockfile
COPY --chown=vp:vp . .
RUN vp build
RUN cp "$(vp env which node | head -1)" /tmp/node

FROM ghcr.io/voidzero-dev/vite-plus:latest-alpine AS deps
WORKDIR /app
COPY --chown=vp:vp package.json pnpm-lock.yaml ./
RUN vp install --frozen-lockfile --prod

# Runtime must be a musl base so the musl Node.js binary runs.
FROM alpine:3.21 AS runtime
WORKDIR /app
ENV NODE_ENV=production
RUN apk add --no-cache libstdc++
COPY --from=build /tmp/node /usr/local/bin/node
COPY --from=build /app/dist ./dist
COPY --from=deps /app/node_modules ./node_modules
COPY --from=build /app/package.json ./
USER nobody
EXPOSE 3000
CMD ["node", "dist/server.js"]
```

For a static SPA there is no Node.js at runtime, so only the builder changes:
swap the build stage to `ghcr.io/voidzero-dev/vite-plus:latest-alpine`; the
`nginx:alpine` runtime and its output are unchanged.

## Notes

- **Node.js version**: provisioned from `.node-version`, `engines.node`, or
  `devEngines.runtime` at build time, so there is no Node-specific image tag. The
  dependency layer copies only `package.json` + the lockfile (always present);
  `.node-version`, if your project uses it, is picked up from the full
  `COPY . .` before `vp build`.
- **Non-root user**: the image runs as the non-root `vp` user, so copy sources
  with `COPY --chown=vp:vp ...` as shown. Without it, `COPY` writes root-owned
  files that `vp install` cannot update (permission denied).
- **Native addons**: the image includes a C/C++ build toolchain (`build-essential`,
  `python3`), so native dependencies such as `better-sqlite3` compile during
  `vp install`.
- **glibc by default**: the default image is glibc based so it uses the official,
  signature-verified Node.js builds. An [Alpine/musl variant](#alpine-musl-variant)
  is also published (`-alpine` tags) with the tradeoffs noted above.
- **Custom base image**: to add `vp` to your own base image instead, run the
  installer: `curl -fsSL https://vite.plus | bash` (set `VP_VERSION` to pin a
  version).

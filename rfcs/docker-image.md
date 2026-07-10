# RFC: Official Vite+ Docker Image

- Issue: [#1490](https://github.com/voidzero-dev/vite-plus/issues/1490)
- Plan: [#1324](https://github.com/voidzero-dev/vite-plus/issues/1324) ("Distribute `vp` across Homebrew, Windows installer, Docker image, apt etc.")

## Summary

Publish an official Vite+ Docker image to GHCR that bundles the `vp` global CLI
for the **build, CI, and development** phases. The image is a toolchain image,
not a production runtime image. Because `vp` already reads `.node-version` /
`engines.node` / `devEngines.runtime` and downloads that exact Node.js version, the
image needs no Node.js-version-specific tags: one image builds any project against
its pinned Node.js.

For production, this RFC does not ship a runtime image. Instead it documents a
multi-stage pattern where the `vp` builder resolves and downloads the exact
official (glibc, signature-verified) Node.js, and a slim final stage copies just
that Node.js binary plus the built app and production dependencies into a small
glibc base (no `vp`). This keeps deployed images small while honoring the
project's pinned Node.js version, which is what [#1490](https://github.com/voidzero-dev/vite-plus/issues/1490)
asks for.

## Motivation

### The problem (#1490)

When containerizing a Vite+ project, users need the Node.js version to match the
project's `.node-version` exactly. The reporter's project pins `24.15.0`:

```text
Environment:
  Package manager  pnpm v10.33.2
  Node.js          v24.15.0 (.node-version)
```

Their options today both have downsides:

- `node:24-alpine` matches the major version and is reasonably small, but it is
  musl-based and roughly doubles the image size in their case, and the tag does
  not pin the exact patch version.
- `alpine:3.23` + `apk add nodejs` is much smaller, but Alpine currently ships
  `24.14.1`, which does not match the pinned `24.15.0`.

There is no Vite+ Docker image or documented Docker pattern that keeps the
container Node.js aligned with `.node-version`. This RFC provides both.

### Why Vite+ is well positioned

Every comparable tool delegates the Node.js version to the base `node:*` image tag
and only manages the _package manager_ (via Corepack). Vite+ already manages the
Node.js runtime itself: it reads the project's config and downloads the exact Node.js,
verifying the official `SHASUMS256.txt.asc` PGP signature (see
[`js-runtime.md`](./js-runtime.md) and
[`verify-node-shasums-signature.md`](./verify-node-shasums-signature.md)). The
Docker story can build directly on that machinery instead of reinventing
version pinning with image tags.

## Prior art

Researched against current official docs (2026-06-25). Summary of how
comparable tools handle Node.js version + Docker:

| Tool              | Official image                        | How the Node.js version is set                   | Default base                | musl/Alpine stance                        |
| ----------------- | ------------------------------------- | ------------------------------------------------ | --------------------------- | ----------------------------------------- |
| Volta             | no (community only)                   | `volta` field in package.json, auto-fetch        | glibc only                  | unsupported (libc dependency)             |
| mise              | exists but "do not use"               | `mise install` from `.tool-versions`/`mise.toml` | debian-slim                 | discouraged; needs `MISE_LIBC=musl`       |
| proto / moon      | no (moon docs only)                   | layered on top of `node:*` base                  | `node:latest`               | needs `MOON_TOOLCHAIN_FORCE_GLOBALS=true` |
| asdf              | no (community only)                   | `asdf install` from `.tool-versions`             | community                   | per-plugin; glibc Node.js by default      |
| pnpm              | yes (`ghcr.io/pnpm/pnpm`, no Node.js) | base `node:*` tag + Corepack                     | debian-slim                 | not addressed                             |
| Yarn              | no                                    | base `node:*` tag + Corepack (`packageManager`)  | `node:*`                    | n/a                                       |
| Turborepo         | no                                    | base `node:*` tag; `turbo prune --docker`        | `node:*-alpine`             | adds `libc6-compat`                       |
| Nx                | no                                    | base `node:*` tag; `prune-lockfile`              | `node:lts-alpine`           | not addressed                             |
| Bun               | yes (`oven/bun`)                      | own runtime                                      | debian; offers distroless   | not discussed                             |
| Deno              | yes (Hub + GHCR)                      | own runtime; ships a `:bin` image to copy in     | debian; offers distroless   | non-root default                          |
| Node.js official  | yes                                   | the tag is the version                           | debian (`-slim`, `-alpine`) | warns musl breaks glibc apps              |
| distroless nodejs | yes (`gcr.io/distroless/nodejsNN`)    | copy artifacts in                                | debian/glibc, ~45MB         | glibc only                                |

Key takeaways that shape this RFC:

1. **No one else manages Node.js from a config file in a usable published image.**
   The version managers (Volta, mise, proto, asdf) either ship no official image
   or one flagged unusable, and they all hit the musl wall because managed Node.js
   means official glibc builds. The package-manager and monorepo tools pin Node.js
   only via the base `node:*` tag. Vite+ collapsing both axes (Node.js + toolchain)
   into one deterministic, project-driven build step is a genuine differentiator.

2. **The closest analog (mise) and the runtimes (Deno) validate the chosen
   pattern.** mise's documented best practice is to copy the small static binary
   into a slim glibc base and install the pinned tool at build time, not to ship
   a fat all-in-one image. Deno ships a `:bin` image precisely so users can
   `COPY --from=denoland/deno:bin /deno ...` into any base, and its distroless
   variant copies just the binary onto `gcr.io/distroless/cc`. This is exactly
   the multi-stage "copy the resolved Node.js in" pattern below.

3. **glibc is the consensus default.** Every Node.js-managing tool warns about or
   breaks on musl. Defaulting to glibc keeps official signature-verified Node.js
   (the unofficial musl builds publish no PGP signature) and avoids native-addon
   surprises.

4. **Monorepo pruning is the one capability plain package managers lack.**
   Turborepo `turbo prune --docker` and Nx `prune-lockfile` exist only because a
   shared lockfile makes one package's change rebuild every container. Vite+
   owns the workspace graph, so a future `vp prune --docker` is a natural
   follow-up (see Future Work).

Sources: pnpm <https://pnpm.io/docker>; Turborepo <https://turborepo.dev/docs/guides/tools/docker>;
Nx <https://nx.dev/docs/technologies/build-tools/docker/introduction>; mise
<https://mise.jdx.dev/mise-cookbook/docker.html>; moon <https://moonrepo.dev/docs/guides/docker>;
Volta <https://github.com/volta-cli/volta/issues/1162>; Bun <https://bun.com/docs/guides/ecosystem/docker>;
Deno <https://github.com/denoland/deno_docker>; Node.js <https://hub.docker.com/_/node/>;
distroless <https://github.com/GoogleContainerTools/distroless/blob/main/nodejs/README.md>.

## User scenarios

The official image is a toolchain image. The scenarios it serves, in priority
order:

1. **Build stage for app deployment (primary).** Used as `FROM ... AS build` in a
   multi-stage Dockerfile. `vp install` + `vp build` produce the app; the exact
   Node.js from `.node-version` is copied into a slim final stage. This is the
   #1490 anchor.
2. **Container-native CI (primary).** GitLab CI, Buildkite, CircleCI, Jenkins
   agents, Tekton, etc. set `image: ghcr.io/voidzero-dev/vite-plus:<tag>` and run
   `vp install`, `vp check`, `vp test`, `vp build`. (GitHub Actions users are
   already served by `setup-vp`, so this targets the rest of the ecosystem.)
3. **Reproducible dev environments (secondary).** Devcontainers, Codespaces, and
   onboarding: a single image pins Node.js + package managers + vp so the toolchain
   matches the repo with zero host setup.
4. **Ad-hoc / evaluation (secondary).** `docker run --rm -v $PWD:/app -w /app
ghcr.io/voidzero-dev/vite-plus vp <cmd>` to try vp or reproduce a bug report
   on a clean toolchain.
5. **Platform / monorepo builders (secondary).** Internal PaaS and buildpack-style
   systems standardizing on a canonical vp builder; monorepo single-app builds
   (which motivate the future `vp prune --docker`).

What it is explicitly **not**: the production runtime image. Shipping the full
toolchain (vite, rolldown, vitest, oxlint, ...) into a deployed container is the
bloat #1490 is complaining about. Production images are produced from the builder
via the documented multi-stage pattern.

## Goals

1. Publish a maintained, multi-arch (`linux/amd64`, `linux/arm64`) Vite+
   toolchain image on GHCR.
2. Honor `.node-version` automatically at build time via vp's existing managed
   runtime, with no Node.js-version-specific image tags.
3. Document a recommended multi-stage pattern that produces a small production
   image with the exact pinned Node.js and no vp.
4. Keep official, signature-verified glibc Node.js end to end (builder downloads it,
   runtime copies it).
5. Provide patterns for the secondary scenarios (CI, devcontainer, static SPA,
   ad-hoc).

## Non-Goals

1. A production runtime image (documented pattern instead, see Future Work for a
   possible thin runtime base).
2. Node.js-version-keyed image tags (the tag sprawl this design avoids).
3. An Alpine/musl image variant. glibc is the default (official,
   signature-verified Node.js, no native-addon breakage), and Alpine is deferred
   (see Future Work) rather than shipped in the first version.
4. `vp prune --docker` monorepo pruning (Future Work).
5. Docker Hub publishing (GHCR only for now).

## Design

### Image role and version-alignment mechanism

The image bundles `vp` and provisions Node.js at build time:

1. In the build stage, `vp install` / `vp build` cause vp to read `.node-version`
   (or `engines.node` / `devEngines.runtime`), download that exact official Node.js,
   verify its PGP signature, and cache it under
   `$VP_HOME/js_runtime/node/<version>/`.
2. The documented multi-stage pattern copies the resolved Node.js binary plus the
   built app and production dependencies into a slim glibc final stage that does
   not contain vp.

This makes one image version-agnostic across every project's pinned Node.js,
eliminates the Corepack-in-Docker class of problems other tools hit, and keeps
deployed images small.

### Base image, contents, and variants

- **Base:** `debian:bookworm-slim` (glibc). Glibc is required so vp downloads the
  official signature-verified Node.js and so native addons behave; debian-slim is
  the consensus small glibc base (pnpm's choice) and provides the shell, `apt`,
  and `git` that build/CI/dev scenarios need.
- **Preinstalled:** `vp` (on `PATH`), `ca-certificates`, `curl`, `git`, and a
  build toolchain (`build-essential`, `python3`, `pkg-config`) for native addon
  compilation (for example `better-sqlite3`). Package managers are handled by
  vp's managed corepack/runtime, so they are provisioned per-project rather than
  baked to a fixed version.
- **No baked default Node.js:** the installer pre-provisions a default Node.js
  (~190 MB); the image drops it (`rm -rf $VP_HOME/js_runtime`) because each
  project provisions its own pinned Node.js at build time, so a default is dead
  weight in a builder. The `node`/`npm`/`npx` shims remain and fetch the right
  version on first use. This keeps the toolchain image ~190 MB smaller, more than
  a switch to Alpine/musl would save (and without the musl tradeoffs).
- **User:** create a non-root `vp` user (mirroring Bun's `USER bun` and Deno's
  `USER deno`); document switching to root for steps that need `apt`. Because the
  image runs as non-root, the documented multi-stage examples copy sources with
  `COPY --chown=vp:vp ...`; without it `COPY` writes root-owned files that
  `vp install` cannot update (permission denied). Verified end to end against the
  published preview image.
- **Possible later variants:** an Alpine/musl toolchain image (deferred, see
  Future Work) and a `-slim` image without the native build toolchain for
  projects with no native deps.

### How `vp` gets into the image

The image installs `vp` with the official install script, pinned to the release
version:

```dockerfile
RUN curl -fsSL https://vite.plus | VP_VERSION="${VP_VERSION}" bash
```

The publish job runs after the npm release, so the pinned version is already on
the registry. This reuses the install script's battle-tested platform detection
(including correct gnu/musl and amd64/arm64 selection under buildx), so the same
Dockerfile produces every architecture without a per-arch artifact copy. The
image version maps 1:1 to a `vp` release via the `VP_VERSION` build arg, and the
same one-liner is the documented way to add `vp` to a custom base image.

A fully hermetic build that copies the per-arch `vp` binary from the release
artifacts (no network at image-build time) is a possible later hardening; it is
not required for v1.

### Tagging

Tags track the `vp` version, not Node.js:

- `ghcr.io/voidzero-dev/vite-plus:latest`
- `ghcr.io/voidzero-dev/vite-plus:<major>` (for example `:1`)
- `ghcr.io/voidzero-dev/vite-plus:<major>.<minor>` (for example `:1.4`)
- `ghcr.io/voidzero-dev/vite-plus:<major>.<minor>.<patch>` (for example `:1.4.2`)

Users pin by exact tag or digest for reproducibility. No `node-<version>` tags.

### Security and reproducibility

- Official, signature-verified glibc Node.js throughout (no unofficial musl builds).
- Non-root default user.
- Multi-arch manifest (`linux/amd64`, `linux/arm64`); vp already ships
  `{x86_64,aarch64}-unknown-linux-gnu` binaries.
- Pinnable by digest.

### Locating the resolved Node.js for the runtime stage

No new CLI surface is required: `vp env which node` prints the resolved Node.js
binary path as its first (uncolored, pipe-friendly) line, and the runtime lives
at `$VP_HOME/js_runtime/node/<version>/bin/node`. The runtime stage copies that
file directly.

### Publishing pipeline

Add an image build/publish job to the release flow (`release.yml` /
`reusable-release-build.yml`) that builds the multi-arch image from the release
binaries and pushes to GHCR with the tag set above, gated on a successful
release. (Exact wiring is an implementation detail for the PR.)

### Pre-release validation (preview image)

To verify the image before a real release, the preview publish workflow
(`publish-preview.yml`, triggered by the `preview-build` label) also builds the
multi-arch image, but from the PR's registry bridge build (`VP_PR_VERSION`),
and pushes it as `ghcr.io/voidzero-dev/vite-plus:pr-<number>`
(never `latest`). This reuses the exact same `docker/Dockerfile` as the release,
so labeling a PR with `preview-build` produces a pullable preview image that
exercises the real build path.

### Docs example verification

The Dockerfile patterns documented below (and in `docs/guide/docker.md`) are kept
honest by a reproduction repo whose GitHub Actions build and smoke-test each
example end to end (build the image, run the container, assert `HTTP 200`, and
assert the SSR runtime Node.js matches the pinned `.node-version`):

- <https://github.com/why-reproductions-are-required/vite-plus-docker-example>

## Recommended Dockerfile patterns (documented for users)

### 1. SSR / Node.js-server app, slim runtime (the #1490 case)

```dockerfile
# syntax=docker/dockerfile:1

# --- build stage: official Vite+ toolchain image ---
FROM ghcr.io/voidzero-dev/vite-plus:1 AS build
WORKDIR /app

# Dependency layer first for cache reuse. --chown is required: the image runs as
# the non-root vp user, and COPY would otherwise write root-owned files that
# vp install cannot update.
COPY --chown=vp:vp package.json pnpm-lock.yaml .node-version* ./
RUN vp install --frozen-lockfile

# Build. vp reads .node-version and provisions that exact Node.js automatically.
COPY --chown=vp:vp . .
RUN vp build

# Export the exact resolved Node.js binary for the runtime stage.
RUN cp "$(vp env which node | head -1)" /tmp/node

# --- deps stage: production-only dependencies (fresh --prod, so devDeps are
# excluded; running --prod over the full install above would not prune them) ---
FROM ghcr.io/voidzero-dev/vite-plus:1 AS deps
WORKDIR /app
COPY --chown=vp:vp package.json pnpm-lock.yaml .node-version* ./
RUN vp install --frozen-lockfile --prod

# --- runtime stage: small, glibc, no vp ---
FROM debian:bookworm-slim AS runtime
WORKDIR /app
ENV NODE_ENV=production

# Exact Node.js from .node-version (official, signature-verified glibc build).
COPY --from=build /tmp/node /usr/local/bin/node

COPY --from=build /app/dist ./dist
COPY --from=deps /app/node_modules ./node_modules
COPY --from=build /app/package.json ./

USER nobody
EXPOSE 3000
CMD ["node", "dist/server.js"]
```

The deployed image carries only Node.js + app + production deps, matches
`.node-version` exactly, and is much smaller than the default `node:*` image.
Production dependencies must be installed in a separate `deps` stage: running
`vp install --prod` over the full install in the build stage does not prune the
already-installed devDependencies (the large `vite-plus` toolchain), so it would
otherwise be copied into the runtime. A distroless final base
(`gcr.io/distroless/cc`) is a documented size/security upgrade for users who do
not need a shell at runtime (see Future Work).

### 2. Static SPA / SSG

```dockerfile
FROM ghcr.io/voidzero-dev/vite-plus:1 AS build
WORKDIR /app
COPY --chown=vp:vp package.json pnpm-lock.yaml .node-version* ./
RUN vp install --frozen-lockfile
COPY --chown=vp:vp . .
RUN vp build

FROM nginx:alpine AS runtime
COPY --from=build /app/dist /usr/share/nginx/html
```

No Node.js at runtime; the vp image is only the builder.

### 3. Container-native CI

```yaml
# e.g. GitLab CI
build:
  image: ghcr.io/voidzero-dev/vite-plus:1
  script:
    - vp install --frozen-lockfile
    - vp check
    - vp test
    - vp build
```

### 4. Devcontainer

```jsonc
// .devcontainer/devcontainer.json
{
  "image": "ghcr.io/voidzero-dev/vite-plus:1",
}
```

### 5. Ad-hoc / evaluation

```bash
docker run --rm -it -v "$PWD:/app" -w /app ghcr.io/voidzero-dev/vite-plus vp build
```

## Open questions

1. **Default app Node.js in the image.** The toolchain image bakes no specific app
   Node.js (vp downloads the pinned version at build, needing network). Should we
   offer a variant with an LTS Node.js prebaked for faster/offline builds, or rely
   on caching and `VP_NODE_DIST_MIRROR`? (Leaning: no prebaked Node.js by default;
   revisit with a prebaked or offline variant if demand appears.)
2. **Native build toolchain by default.** Include `build-essential`/`python3` in
   the default image (larger, but native addons just work), or keep the default
   lean and add them in a `-full` variant? (Leaning: include by default since
   this is a builder image; offer a `-slim` later.)
3. **`vp install --prod` semantics for the runtime copy.** Confirm the exact flag
   set vp exposes for a production-only install and whether a dedicated deps stage
   improves layer caching in the documented pattern.
4. **Image naming.** `ghcr.io/voidzero-dev/vite-plus` vs a `-toolchain` suffix to
   leave room for other images later.

## Future Work

1. **`vp prune <target> --docker`** for monorepos: emit a target-scoped subset
   (package.json files, pruned lockfile, source) so the dependency-install layer
   caches across unrelated workspace edits, matching Turborepo `turbo prune` and
   Nx `prune-lockfile`. This is the one capability plain package managers cannot
   offer and the main reason monorepo Docker guides cite those tools. Likely its
   own RFC.
2. **Distroless runtime guidance/variant.** Document (or provide) a
   `gcr.io/distroless/cc` final stage and the `tini` PID-1 pattern for a smaller,
   shell-less, better-CVE-posture runtime.
3. **Thin runtime base image.** Reconsider only if the documented copy-Node.js-in
   pattern proves insufficient; would reintroduce Node.js-version coupling, so not
   planned.
4. **Alpine/musl variant.** Deferred, not part of the first version; add later
   only on real demand. It would serve teams that mandate Alpine and yields the
   smallest runtime (an Alpine SSR runtime measured ~136 MB), but the musl
   tradeoffs are why it is not shipped now:

   - On musl, vp downloads Node.js from `unofficial-builds.nodejs.org`, which
     publishes no PGP signature (see `crates/vite_js_runtime/src/providers/node.rs`),
     so an Alpine variant would not get the "official, signature-verified Node.js"
     guarantee the Debian image has.
   - musl is the classic native-addon sharp edge (prebuilt addons are usually
     glibc; on musl they need musl prebuilds or source compilation plus
     `gcompat`/`libc6-compat`), which Vite+ projects hit regularly (better-sqlite3,
     sharp). The wider field treats musl as a hazard for the same reasons (Volta
     unsupported on musl, mise needs `MISE_LIBC=musl`, moon needs
     `MOON_TOOLCHAIN_FORCE_GLOBALS=true`, Turborepo `apk add libc6-compat`).
   - A musl Node.js binary only runs on a musl base, so an Alpine builder would
     need an Alpine runtime stage (not debian-slim/distroless).

   If added, ship it as an opt-in `-alpine` variant with loud caveats and a
   documented libc autodetect/override.

5. **Docker Hub publishing** for discoverability, in addition to GHCR.
6. **Offline / airgapped builds**: a prebaked-Node.js variant and `VP_NODE_DIST_MIRROR`
   guidance.

## References

- Issue: [#1490](https://github.com/voidzero-dev/vite-plus/issues/1490)
- Q2 plan: [#1324](https://github.com/voidzero-dev/vite-plus/issues/1324)
- JS runtime management: [`js-runtime.md`](./js-runtime.md)
- Node.js signature verification: [`verify-node-shasums-signature.md`](./verify-node-shasums-signature.md)
- CI guide: `docs/guide/ci.md`
- Distribution prior art: pnpm <https://pnpm.io/docker>, Deno <https://github.com/denoland/deno_docker>,
  mise <https://mise.jdx.dev/mise-cookbook/docker.html>, Turborepo
  <https://turborepo.dev/docs/guides/tools/docker>, distroless
  <https://github.com/GoogleContainerTools/distroless/blob/main/nodejs/README.md>.

# node_container_scope

Check the runtime image and feature option, not source properties or Dev Container feature tags.

## `vpt write-file worker.js 'export const options = { node:2 };
'`


## `vpt write-file .devcontainer/devcontainer.json '{"features":{"ghcr.io/devcontainers/features/node:1":{"version":"lts"}}}
'`


## `vpt write-file Dockerfile '# FROM node:20
FROM node:20-alpine AS build
RUN echo node:2
'`


## `vp migrate --no-interactive`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 2 review items (1 block dependency updates)

.devcontainer/devcontainer.json
  1:1 REVIEW [node-runtime] Resolve Dev Container Node feature (lts) and select Node ^22.18.0 || ^24.11.0 || >=26.0.0.

Dockerfile
  2:1 BLOCK [node-runtime] Node container image (20) cannot run Vite+ with Vitest v5. Select Node ^22.18.0 || ^24.11.0 || >=26.0.0; use vp env pin 22 --force for a runtime pin. Do not widen a library's engines.node contract automatically.
Resolve the blocking Vitest v5 findings, then re-run `vp migrate`. No project files were changed.
```

## `vpt print-file package.json`

```
{
  "name": "migration-vitest-v5",
  "private": true,
  "type": "module",
  "packageManager": "pnpm@11.24.0",
  "scripts": {
    "test": "vitest list"
  },
  "devDependencies": {
    "vite": "^8.0.0",
    "vitest": "<version>"
  }
}
```

## `vpt write-file Dockerfile '# FROM node:20
FROM node:24.11.0-alpine AS build
RUN echo node:2
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

.devcontainer/devcontainer.json
  1:1 REVIEW [node-runtime] Resolve Dev Container Node feature (lts) and select Node ^22.18.0 || ^24.11.0 || >=26.0.0.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
! Warnings:
  - Vitest v5: 1 review item

.devcontainer/devcontainer.json
  1:1 REVIEW [node-runtime] Resolve Dev Container Node feature (lts) and select Node ^22.18.0 || ^24.11.0 || >=26.0.0.
```

## `vpt print-file worker.js`

```
export const options = { node:2 };
```

## `vpt print-file .devcontainer/devcontainer.json`

```
{"features":{"ghcr.io/devcontainers/features/node:1":{"version":"lts"}}}
```

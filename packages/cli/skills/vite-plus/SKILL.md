---
name: vite-plus
description: Vite+ skill for development workflow and CLI operations. Use this skill to route user requests to the appropriate bundled Vite+ docs.
---

# Vite+ Skill

This skill is a router. Open the minimum relevant docs files under `docs/` and execute.

Docs in this skill are bundled from `docs` during `vite-plus` package build and live at:

- `skills/vite-plus/docs/**/*.md`

## Command Naming

Use `vp` in examples and commands in this skill.

## No-Args Behavior

If invoked without a concrete task, do a brief project status check and report:

1. Dev server configuration (`vite.config.ts` or `vite.config.js`).
2. Tool usage (`vp dev`, `vp build`, `vp test`, `vp lint`, `vp fmt`).
3. Monorepo structure (workspace detection, package manager).
4. Build configuration (library mode, SSR, custom config).

Then ask what to do next.

## Task Routing

| User intent                       | Docs file(s)                                          |
| --------------------------------- | ----------------------------------------------------- |
| Initial setup, getting started    | `docs/guide/index.md`                                 |
| Dev server, development workflow  | `docs/guide/dev.md`, `docs/guide/index.md`            |
| Build configuration, optimization | `docs/guide/build.md`, `docs/config/build.md`         |
| Testing with Vitest               | `docs/guide/test.md`, `docs/config/test.md`           |
| Linting with Oxlint               | `docs/guide/lint.md`, `docs/config/lint.md`           |
| Formatting with Oxfmt             | `docs/guide/fmt.md`, `docs/config/fmt.md`             |
| Check (format, lint, types)       | `docs/guide/check.md`                                 |
| Monorepo tasks                    | `docs/guide/run.md`, `docs/config/run.md`             |
| Migration from existing tools     | `docs/guide/migrate.md`                               |
| Caching and performance           | `docs/guide/cache.md`                                 |
| Library mode (pack)               | `docs/guide/pack.md`, `docs/config/pack.md`           |
| Troubleshooting                   | `docs/guide/troubleshooting.md`                       |
| Configuration overview            | `docs/config/index.md`                                |
| Staged files / pre-commit         | `docs/guide/commit-hooks.md`, `docs/config/staged.md` |
| Install dependencies              | `docs/guide/install.md`                               |
| Node.js version management        | `docs/guide/env.md`                                   |
| Create a new project              | `docs/guide/create.md`                                |
| CI setup                          | `docs/guide/ci.md`                                    |
| IDE integration                   | `docs/guide/ide-integration.md`                       |
| Upgrade Vite+                     | `docs/guide/upgrade.md`                               |
| Execute one-off binaries          | `docs/guide/vpx.md`                                   |

## Working Rules

- For multi-topic tasks, combine only the needed doc files.
- If docs and memory differ, follow docs.

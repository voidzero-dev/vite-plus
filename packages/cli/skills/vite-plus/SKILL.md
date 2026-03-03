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

| User intent                       | Docs file(s)                                                                     |
| --------------------------------- | -------------------------------------------------------------------------------- |
| CLI command syntax, flags         | `docs/vite/guide/cli.md`                                                         |
| Initial setup, getting started    | `docs/index.md`, `docs/vite/guide/index.md`, `docs/lib/guide/getting-started.md` |
| Dev server, development workflow  | `docs/vite/guide/index.md`, `docs/vite/guide/cli.md`                             |
| Build configuration, optimization | `docs/config/index.md`, `docs/config/shared-options.md`                          |
| Testing with Vitest               | `docs/vite/guide/tasks.md`, `docs/vite/guide/task/getting-started.md`            |
| Linting with Oxlint               | `docs/vite/guide/cli.md`                                                         |
| Formatting with Oxfmt             | `docs/vite/guide/cli.md`                                                         |
| Monorepo setup and management     | `docs/vite/guide/monorepo.md`                                                    |
| Migration from existing tools     | `docs/vite/guide/migration.md`                                                   |
| Caching and performance           | `docs/vite/guide/caching.md`                                                     |
| Library mode                      | `docs/lib/guide/getting-started.md`                                              |
| Troubleshooting                   | `docs/vite/guide/troubleshooting.md`                                             |
| Configuration and shared options  | `docs/config/shared-options.md`                                                  |
| API reference                     | `docs/apis/index.md`                                                             |

## Working Rules

- For CLI-heavy tasks, open `docs/vite/guide/cli.md` first.
- For multi-topic tasks, combine only the needed doc files.
- If docs and memory differ, follow docs.

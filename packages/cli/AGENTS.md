<!--VITE PLUS START-->

# Using Vite+, the Unified Toolchain for the Web

This project is using Vite+, a unified toolchain built on top of Vite, Rolldown, Vitest, tsdown, Oxlint, Oxfmt, and Vite Task. Vite+ wraps runtime management, package management, and frontend tooling in a single global CLI called `vp`. Vite+ is distinct from Vite, and it invokes Vite through `vp dev` and `vp build`. Run `vp help` to print a list of commands and `vp <command> --help` for information about a specific command.

## Common Pitfalls

- **Running scripts:** Vite+ built-in commands (`vp lint`, `vp dev`, `vp build`, `vp test`, etc.) always run the Vite+ built-in tool. Use `vp run <script>` to run `package.json` scripts or tasks defined in `vite.config.ts`
- **Do not install Vitest, Oxlint, Oxfmt, or tsdown directly:** Vite+ wraps these tools. Do not install or upgrade them, use Vite+ directly.
- **Import JavaScript modules from `vite-plus`:** Import modules from the `vite-plus` dependency, not from `vite` or `vitest`. For example, `import { defineConfig } from 'vite-plus';` or `import { expect, test, vi } from 'vite-plus/test';`. You must not install `vitest` to import test utilities.

## Review Checklist

- [ ] Run `vp install` after pulling remote changes and before getting started.
- [ ] Run `vp check` and `vp test` to format, lint, type check and test changes.
- [ ] Check if there are tasks or `package.json` scripts necessary for validation, run via `vp run <script>`.

* Docs: https://viteplus.dev/guide/
<!--VITE PLUS END-->

# Why Vite+?

Working in the JavaScript ecosystem today, developers need a runtime such as Node.js, a package manager like pnpm, a dev server, a linter, a formatter, a test runner, a bundler, a task runner, and a growing number of config files.

Vite showed that frontend tooling could become dramatically faster by rethinking the architecture instead of accepting the status quo. Vite+ applies that same idea to the rest of the local development workflow, and unifies them all into a single package that speeds up and simplifies development.

## The Problem Vite+ is Solving

The JavaScript tooling ecosystem has seen its fair share of fragmentation and churn. Web apps keep getting larger, and as a result tooling performance, complexity, and inconsistencies have become real bottlenecks as projects grow.

These bottlenecks are amplified in organizations with multiple teams, each using a different tooling stack. Dependency management, build infrastructure, and code quality become fragmented responsibilities, handled team by team and often not owned as a priority by anyone. As a result, dependencies drift out of sync, builds get slower, and code quality declines. Fixing those problems later requires significantly more effort, slows everyone down, and pulls teams away from shipping product.

## What's Included in Vite+

Vite+ brings the tools needed for modern web development together into a single, integrated toolchain. Instead of assembling and maintaining a custom toolchain, Vite+ provides a consistent entry point that manages the runtime, dependencies, development server, code quality checks, testing, and builds in one place.

- **[Vite](https://vite.dev/)** and **[Rolldown](https://rolldown.rs/)** for development and application builds
- **[Vitest](https://vitest.dev/)** for testing
- **[Oxlint](https://oxc.rs/docs/guide/usage/linter.html)** and **[Oxfmt](https://oxc.rs/docs/guide/usage/formatter.html)** for linting and formatting
- **[tsdown](https://tsdown.dev/)** for library builds or standalone executables
- **[Vite Task](https://github.com/voidzero-dev/vite-task)** for task orchestration

In practice, this means developers interact with one consistent workflow: `vp dev`, `vp check`, `vp test`, and `vp build`.

This unified toolchain reduces configuration overhead, improves performance, and makes it easier for teams to maintain consistent tooling across projects.

## Fast and Scalable by Default

Vite+ is built on top of modern tooling such as Vite, Rolldown, Oxc, Vitest, and Vite Task to keep your projects fast and scalable as your codebase grows. By using Rust, we can speed up common tasks by [10× or sometimes even by 100×](https://voidzero.dev/posts/announcing-vite-plus-alpha#performance-scale). However, many Rust-based toolchains are incompatible with existing tools, or aren't extensible using JavaScript.

Vite+ bridges Rust to JavaScript via [NAPI-RS](https://napi.rs/) which allows it to provide a familiar, easy-to-configure, and extensible interface in JavaScript with a great ecosystem-compatible developer experience.

Unifying the toolchain has performance benefits beyond just using faster tools on their own. For example, many developers set up their linter with "type aware" tools, requiring a full-typecheck to be run during the linting stage. With `vp check` you can format, lint, and type-check your code all in a single pass, speeding up static checks by 2× compared to running type-aware lint rules and type-checks separately.

## Fully Open Source

Vite+ is fully open source and not a new framework or locked-down platform. Vite+ integrates with the existing Vite ecosystem and the frameworks built on top of it, including React, Vue, Svelte, and others. It can use pnpm, npm, yarn, or Bun as package manager, and manages the Node.js runtime for you.

We always welcome contributions from the community. See our [Contributing Guidelines](https://github.com/voidzero-dev/vite-plus/blob/main/CONTRIBUTING.md) to get involved.

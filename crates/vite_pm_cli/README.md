# vite_pm_cli

Owns Vite+'s package-manager lifecycle:

- detects the project package manager and requested version;
- downloads and caches managed package-manager binaries;
- exposes the shared clap command surface used by global and local CLIs;
- resolves typed arguments for pnpm, npm, Yarn, or Bun;
- executes the resolved command and its pre-run actions.

Managed Node.js runtimes and Vite+'s managed global-package store remain in the
global CLI.

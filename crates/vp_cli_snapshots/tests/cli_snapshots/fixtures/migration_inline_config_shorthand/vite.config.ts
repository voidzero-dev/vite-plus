import { defineConfig } from 'vite-plus';

// Mirrors a custom template that keeps tooling config in separate modules and
// wires them in with shorthand properties (`fmt,` / `lint,`). See #1836.
const fmt = { ignorePatterns: [] };
const lint = { rules: {} };

export default defineConfig(({ mode }) => {
  return {
    server: { port: 3000 },
    fmt,
    lint,
  };
});

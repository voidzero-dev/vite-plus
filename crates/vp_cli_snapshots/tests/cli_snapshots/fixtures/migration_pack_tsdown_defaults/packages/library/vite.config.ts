import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: [
    {
      entry: ['src/index.ts'],
      attw: true,
    },
    {
      entry: ['src/index.ts'],
      deps: {},
      attw: { enabled: true },
    },
    {
      entry: ['src/index.ts'],
      deps: { resolveDepSubpath: false },
      attw: { profile: 'esm-only' },
    },
    {
      entry: ['src/index.ts'],
      deps: { resolveDepSubpath: true },
      attw: { profile: 'strict' },
    },
  ],
});

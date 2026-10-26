import { defineConfig } from 'tsdown';

export default defineConfig({
  deps: {
    onlyBundle: false,
  },
  dts: {
    // Generate declarations with Oxc so the build does not depend on the
    // resolved TypeScript version: rolldown-plugin-dts requires the classic
    // tsc API (TypeScript <7) or the tsgo API (TypeScript 7.1+).
    generator: 'oxc',
  },
});

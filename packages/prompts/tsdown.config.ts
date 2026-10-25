import { defineConfig } from 'tsdown';

export default defineConfig({
  deps: {
    onlyBundle: false,
  },
  dts: {
    // TypeScript 7 no longer ships the classic tsc API and its tsgo API
    // requires 7.1+, so rolldown-plugin-dts cannot use either here. Generate
    // declarations with Oxc instead.
    generator: 'oxc',
  },
});

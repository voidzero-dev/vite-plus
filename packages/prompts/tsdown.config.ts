import { defineConfig } from 'tsdown';

export default defineConfig({
  deps: {
    onlyBundle: false,
  },
  dts: {
    // The repo uses typescript@7 (native), which rolldown-plugin-dts's
    // default `tsc` generator cannot load. Emit declarations with Oxc,
    // matching packages/core/build.ts.
    generator: 'oxc',
  },
});

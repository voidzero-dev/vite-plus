import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: {
    entry: ['src/index.ts'],
    dts: process.env.PACK_DTS_GENERATOR === 'tsgo' ? { generator: 'tsgo' } : true,
  },
});

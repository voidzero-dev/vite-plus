import { defineConfig } from 'vite-plus';

export default defineConfig(() => ({
  // This is Vite's public directory, which must stay unchanged.
  publicDir: 'vite-public',
  pack: [
    {
      entry: 'src/index.ts',
      bundle: false,
      outExtension: () => ({ js: '.mjs' }),
      publicDir: 'public',
      removeNodeProtocol: true,
      injectStyle: true,
      inlineOnly: [/^allowed/],
      skipNodeModulesBundle: true,
      dts: { oxc: true, cjsReexport: false },
      attw: true,
    },
    {
      entry: 'src/index.ts',
      deps: { onlyAllowBundle: false, skipNodeModulesBundle: true, resolveDepSubpath: false },
      dts: { tsgo: { path: './tsgo' }, cjsReexport: true },
      attw: { profile: 'node16' },
    },
  ],
}));

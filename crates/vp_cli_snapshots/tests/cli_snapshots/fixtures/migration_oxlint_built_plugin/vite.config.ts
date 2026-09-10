import { defineConfig } from 'vite';

export default defineConfig({
  lint: {
    jsPlugins: [{ name: 'built', specifier: './dist/plugin.cjs' }],
    rules: { 'built/no-foo': 'warn' },
    options: { typeAware: false, typeCheck: false },
  },
});

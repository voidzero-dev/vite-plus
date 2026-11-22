import { defineConfig } from 'vite-plus';

// A command held in a constant makes the run config non-static, so it is
// loaded through the JS resolver.
const command = 'vpt print hello';

export default defineConfig({
  run: {
    tasks: {
      hello: { command },
    },
  },
});

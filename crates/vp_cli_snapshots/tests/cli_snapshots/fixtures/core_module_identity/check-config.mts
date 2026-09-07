import { defineConfig, type UserConfig } from 'vite-plus';

const config: UserConfig = {
  test: { globals: true },
  pack: { entry: ['entry.js'] },
  lint: {},
  fmt: {},
  run: { tasks: { hello: { command: 'echo hello' } } },
};
defineConfig(config);
defineConfig({ test: { globals: true } });

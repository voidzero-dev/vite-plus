import { defineConfig, type UserConfig } from 'tsdown';

const config: UserConfig = {
  deps: {
    onlyBundle: false,
  },
};

export default defineConfig(config);

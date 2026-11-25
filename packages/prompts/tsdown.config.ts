import type { UserConfig } from 'tsdown';
import { defineConfig } from 'tsdown';

const config: UserConfig = {
  deps: {
    onlyBundle: false,
  },
};

export default defineConfig(config);

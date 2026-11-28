import type { UserConfig } from 'tsdown';
import { defineConfig } from 'tsdown';

const config: UserConfig = defineConfig({
  deps: {
    onlyBundle: false,
  },
});

export default config;

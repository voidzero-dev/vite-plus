import type { UserConfig as TsdownUserConfig } from '@voidzero-dev/vite-plus-core/lib';

export { defineConfig, build, globalLogger } from '@voidzero-dev/vite-plus-core/lib';
export type * from '@voidzero-dev/vite-plus-core/lib';

export interface LibUserConfig extends TsdownUserConfig {
  /**
  * When loading env variables from `envFile`, only include variables with these prefixes.
  * @default ['VITE_LIB_', 'TSDOWN_']
  */
  envPrefix?: string | string[];
}

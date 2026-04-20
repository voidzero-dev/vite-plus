import type { UserConfig as TsdownUserConfig } from '@voidzero-dev/vite-plus-core/pack';

export * from '@voidzero-dev/vite-plus-core/pack';

export interface PackUserConfig extends TsdownUserConfig {
  /**
   * When loading env variables from `envFile`, only include variables with these prefixes.
   * @default ['VITE_PACK_', 'TSDOWN_']
   */
  envPrefix?: string | string[];
}

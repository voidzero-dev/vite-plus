import type { RuntimeEnvConfig } from './runtime.env.config.js';
import type { RuntimeHtmlConfig } from './runtime.html.config.js';

declare module 'vite' {
  interface UserConfig {
    /**
     * Options for vite-plugin-runtime-env
     */
    runtimeEnv?: RuntimeEnvConfig;
    /**
     * Options for vite-plugin-runtime-html
     */
    runtimeHtml?: RuntimeHtmlConfig;
  }
}

declare module 'vitest' {
  export const describe: any;
  export const it: any;
  export const expect: any;
  export const beforeAll: any;
  export const afterAll: any;
}

declare module 'vitest/config' {
  export function defineConfig(config: any): any;
  const _default: typeof defineConfig;
  export default _default;
}

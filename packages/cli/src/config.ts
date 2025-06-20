import { isAbsolute, join } from "node:path";
import type { UserConfig } from "vite";
import type { UserConfig as VitestUserConfig } from "vitest/node";
import { deepMerge } from "@vitest/utils";

interface VitePlusConfig extends UserConfig {
  extends?: string;
  test?: VitestUserConfig;
}

export const defineConfig = async (config: VitePlusConfig): Promise<UserConfig> => {
  if (config.extends) {
    const extendsPath = isAbsolute(config.extends) ? config.extends : join(process.cwd(), config.extends);
    const { default: baseConfig } = await import(extendsPath);
    return deepMerge(await baseConfig, config);
  }
  return config;
};

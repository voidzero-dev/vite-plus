import fs from 'node:fs';
import path from 'node:path';

export interface ConfigFiles {
  viteConfig?: string;
  vitestConfig?: string;
  tsdownConfig?: string;
  oxlintConfig?: string;
  oxfmtConfig?: string;
  eslintConfig?: string;
  eslintLegacyConfig?: string;
}

export function detectConfigs(projectPath: string): ConfigFiles {
  const configs: ConfigFiles = {};

  // Check for vite.config.*
  // https://vite.dev/config/
  const viteConfigs = [
    'vite.config.ts',
    'vite.config.mts',
    'vite.config.cts',
    'vite.config.js',
    'vite.config.mjs',
    'vite.config.cjs',
  ];
  for (const config of viteConfigs) {
    if (fs.existsSync(path.join(projectPath, config))) {
      configs.viteConfig = config;
      break;
    }
  }

  // Check for vitest.config.*
  // https://vitest.dev/config/
  const vitestConfigs = [
    'vitest.config.ts',
    'vitest.config.mts',
    'vitest.config.cts',
    'vitest.config.js',
    'vitest.config.mjs',
    'vitest.config.cjs',
  ];
  for (const config of vitestConfigs) {
    if (fs.existsSync(path.join(projectPath, config))) {
      configs.vitestConfig = config;
      break;
    }
  }

  // Check for tsdown.config.*
  // https://tsdown.dev/options/config-file
  const tsdownConfigs = [
    'tsdown.config.ts',
    'tsdown.config.mts',
    'tsdown.config.cts',
    'tsdown.config.js',
    'tsdown.config.mjs',
    'tsdown.config.cjs',
    'tsdown.config.json',
    'tsdown.config',
  ];
  // Additionally, you can define your configuration directly in the `tsdown` field of your package.json file
  for (const config of tsdownConfigs) {
    if (fs.existsSync(path.join(projectPath, config))) {
      configs.tsdownConfig = config;
      break;
    }
  }

  // Check for oxlint configs
  // https://oxc.rs/docs/guide/usage/linter/config.html#configuration-file-format
  const oxlintConfigs = ['.oxlintrc.json'];
  for (const config of oxlintConfigs) {
    if (fs.existsSync(path.join(projectPath, config))) {
      configs.oxlintConfig = config;
      break;
    }
  }

  // Check for oxfmt configs
  // https://oxc.rs/docs/guide/usage/formatter.html#configuration-file
  const oxfmtConfigs = ['.oxfmtrc.json', '.oxfmtrc.jsonc'];
  for (const config of oxfmtConfigs) {
    if (fs.existsSync(path.join(projectPath, config))) {
      configs.oxfmtConfig = config;
      break;
    }
  }

  // Check for eslint configs (flat config only)
  // https://eslint.org/docs/latest/use/configure/configuration-files
  const eslintConfigs = [
    'eslint.config.js',
    'eslint.config.mjs',
    'eslint.config.cjs',
    'eslint.config.ts',
    'eslint.config.mts',
    'eslint.config.cts',
  ];
  for (const config of eslintConfigs) {
    if (fs.existsSync(path.join(projectPath, config))) {
      configs.eslintConfig = config;
      break;
    }
  }

  // Check for legacy eslint configs (.eslintrc*)
  // https://eslint.org/docs/latest/use/configure/configuration-files-deprecated
  const eslintLegacyConfigs = [
    '.eslintrc',
    '.eslintrc.json',
    '.eslintrc.js',
    '.eslintrc.cjs',
    '.eslintrc.yaml',
    '.eslintrc.yml',
  ];
  for (const config of eslintLegacyConfigs) {
    if (fs.existsSync(path.join(projectPath, config))) {
      configs.eslintLegacyConfig = config;
      break;
    }
  }

  return configs;
}

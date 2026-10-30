import fs from 'node:fs';
import path from 'node:path';

import type { WorkspacePackage } from '../types/index.ts';
import { VITE_CONFIG_FILES } from '../utils/constants.ts';

export interface ConfigFiles {
  viteConfig?: string;
  vitestConfig?: string;
  tsdownConfig?: string;
  tsupConfig?: string;
  oxlintConfig?: string;
  oxfmtConfig?: string;
  eslintConfig?: string;
  eslintLegacyConfig?: string;
  prettierConfig?: string; // e.g. '.prettierrc.json', 'prettier.config.js', PRETTIER_PACKAGE_JSON_CONFIG
  prettierIgnore?: boolean;
  nvmrcFile?: boolean;
  voltaNode?: string;
}

// Sentinel value indicating Prettier config lives inside package.json "prettier" key.
export const PRETTIER_PACKAGE_JSON_CONFIG = 'package.json#prettier';

// Sentinel value indicating tsup config lives inside package.json "tsup" key.
export const TSUP_PACKAGE_JSON_CONFIG = 'package.json#tsup';

// All known tsup config file names (standalone files only).
// https://tsup.egoist.dev/#using-a-config-file
export const TSUP_CONFIG_FILES = [
  'tsup.config.ts',
  'tsup.config.mts',
  'tsup.config.cts',
  'tsup.config.js',
  'tsup.config.mjs',
  'tsup.config.cjs',
  'tsup.config.json',
] as const;

// All known Prettier config file names (standalone files only).
// https://prettier.io/docs/configuration
export const PRETTIER_CONFIG_FILES = [
  '.prettierrc',
  '.prettierrc.json',
  '.prettierrc.jsonc',
  '.prettierrc.yaml',
  '.prettierrc.yml',
  '.prettierrc.toml',
  '.prettierrc.js',
  '.prettierrc.cjs',
  '.prettierrc.mjs',
  '.prettierrc.ts',
  '.prettierrc.cts',
  '.prettierrc.mts',
  'prettier.config.js',
  'prettier.config.cjs',
  'prettier.config.mjs',
  'prettier.config.ts',
  'prettier.config.cts',
  'prettier.config.mts',
] as const;

const CONFIG_SCRIPT_EXTENSIONS = ['js', 'mjs', 'cjs', 'ts', 'mts', 'cts'] as const;

const MIGRATION_WARNING_DETECTIONS = [
  {
    packageName: 'rollup',
    configFiles: CONFIG_SCRIPT_EXTENSIONS.map((extension) => `rollup.config.${extension}`),
    warning:
      'Rollup configuration detected. Rollup is API-compatible with Rolldown and can be migrated first before migrating further to tsdown or Vite. Once migrated to tsdown or Vite, re-run vp migrate.',
  },
  {
    packageName: 'rolldown',
    configFiles: CONFIG_SCRIPT_EXTENSIONS.map((extension) => `rolldown.config.${extension}`),
    warning:
      'Rolldown configuration detected. Rolldown can be migrated to tsdown or Vite, and it is recommended to do so. Once migrated, re-run vp migrate.',
  },
  {
    packageName: 'jest',
    configFiles: [
      ...CONFIG_SCRIPT_EXTENSIONS.map((extension) => `jest.config.${extension}`),
      'jest.config.json',
    ],
    packageJsonField: 'jest',
    warning:
      'Jest configuration detected. Vitest is highly compatible with Jest, and so we recommend migrating. See the Vitest Jest migration guide: https://vitest.dev/guide/migration/jest, then re-run vp migrate.',
  },
] as const;

function hasDependency(packageJson: Record<string, unknown>, packageName: string): boolean {
  return ['dependencies', 'devDependencies', 'optionalDependencies'].some((field) => {
    const dependencies = packageJson[field];
    return typeof dependencies === 'object' && dependencies !== null && packageName in dependencies;
  });
}

/**
 * Detect tools that have a direct migration path worth showing before the
 * regular Vite+ migration steps run.
 */
export function detectMigrationWarnings(
  projectPath: string,
  packages: readonly WorkspacePackage[] = [],
): string[] {
  const projectPaths = [
    projectPath,
    ...packages.map((workspacePackage) => path.join(projectPath, workspacePackage.path)),
  ];
  const detected = new Set<string>();
  let rootPackageJson: Record<string, unknown> | undefined;

  for (const currentPath of projectPaths) {
    const packageJsonPath = path.join(currentPath, 'package.json');
    if (!fs.existsSync(packageJsonPath)) {
      continue;
    }

    let packageJson: Record<string, unknown>;
    try {
      packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8')) as Record<string, unknown>;
    } catch {
      continue;
    }
    if (currentPath === projectPath) {
      rootPackageJson = packageJson;
    }

    for (const detection of MIGRATION_WARNING_DETECTIONS) {
      if (
        (hasDependency(packageJson, detection.packageName) ||
          (currentPath !== projectPath &&
            rootPackageJson !== undefined &&
            hasDependency(rootPackageJson, detection.packageName))) &&
        (detection.configFiles.some((configFile) =>
          fs.existsSync(path.join(currentPath, configFile)),
        ) ||
          ('packageJsonField' in detection &&
            Object.hasOwn(packageJson, detection.packageJsonField)))
      ) {
        detected.add(detection.warning);
      }
    }
  }

  return MIGRATION_WARNING_DETECTIONS.map(({ warning }) => warning).filter((warning) =>
    detected.has(warning),
  );
}

export function detectConfigs(projectPath: string): ConfigFiles {
  const configs: ConfigFiles = {};

  for (const config of VITE_CONFIG_FILES) {
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

  // Check for tsup.config.* (still detected even though tsdown supersedes it —
  // `promptTsupMigration` / `detectTsupProject` use this to offer the tsup →
  // tsdown migration).
  // https://tsup.egoist.dev/#using-a-config-file
  for (const config of TSUP_CONFIG_FILES) {
    if (fs.existsSync(path.join(projectPath, config))) {
      configs.tsupConfig = config;
      break;
    }
  }

  // Check for oxlint configs
  // https://oxc.rs/docs/guide/usage/linter/config.html#configuration-file-format
  const oxlintConfigs = ['.oxlintrc.json', '.oxlintrc.jsonc'];
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

  // Check for prettier configs
  for (const config of PRETTIER_CONFIG_FILES) {
    if (fs.existsSync(path.join(projectPath, config))) {
      configs.prettierConfig = config;
      break;
    }
  }
  // Check for .prettierignore
  if (fs.existsSync(path.join(projectPath, '.prettierignore'))) {
    configs.prettierIgnore = true;
  }

  // Check for .nvmrc (nvm)
  if (fs.existsSync(path.join(projectPath, '.nvmrc'))) {
    configs.nvmrcFile = true;
  }

  // Check package.json for "prettier" key and Volta node version
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (fs.existsSync(packageJsonPath)) {
    try {
      const content = fs.readFileSync(packageJsonPath, 'utf8');
      const pkg = JSON.parse(content);

      if (!configs.prettierConfig && pkg.prettier) {
        configs.prettierConfig = PRETTIER_PACKAGE_JSON_CONFIG;
      }

      if (!configs.tsupConfig && pkg.tsup) {
        configs.tsupConfig = TSUP_PACKAGE_JSON_CONFIG;
      }

      const voltaNode = pkg.volta?.node;
      if (typeof voltaNode === 'string') {
        configs.voltaNode = voltaNode;
      }
    } catch {
      // ignore parse errors
    }
  }

  return configs;
}

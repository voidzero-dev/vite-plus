import fs from 'node:fs';
import path from 'node:path';

import { VITE_CONFIG_FILES } from '../utils/constants.ts';

export interface ConfigFiles {
  viteConfig?: string;
  vitestConfig?: string;
  tsdownConfig?: string;
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

// Oxlint and Oxfmt each accept a static JSON config or a dynamic TypeScript one.
// The JSON forms are inlined into `vite.config.*` during migration and deleted;
// the dynamic forms are preserved and imported instead. Detection takes the
// first match in each list, so the two forms are ordered JSON-first only to keep
// the historical precedence — `detectOxcConfigConflicts` rejects any directory
// holding more than one of these before that precedence can matter.
// https://oxc.rs/docs/guide/usage/linter/config.html#configuration-file-format
export const OXLINT_JSON_CONFIG_FILES = ['.oxlintrc.json', '.oxlintrc.jsonc'] as const;
export const OXLINT_DYNAMIC_CONFIG_FILES = ['oxlint.config.ts', 'oxlint.config.mts'] as const;
export const OXLINT_CONFIG_FILES = [
  ...OXLINT_JSON_CONFIG_FILES,
  ...OXLINT_DYNAMIC_CONFIG_FILES,
] as const;

// https://oxc.rs/docs/guide/usage/formatter.html#configuration-file
export const OXFMT_JSON_CONFIG_FILES = ['.oxfmtrc.json', '.oxfmtrc.jsonc'] as const;
export const OXFMT_DYNAMIC_CONFIG_FILES = ['oxfmt.config.ts', 'oxfmt.config.mts'] as const;
export const OXFMT_CONFIG_FILES = [
  ...OXFMT_JSON_CONFIG_FILES,
  ...OXFMT_DYNAMIC_CONFIG_FILES,
] as const;

export interface OxcConfigConflict {
  /** `oxlint` or `oxfmt` — the tool whose config is ambiguous. */
  tool: 'oxlint' | 'oxfmt';
  /** Directory holding the competing configs, relative to the workspace root ('.' for the root). */
  dir: string;
  /** Every config for `tool` present in `dir`, in the tool's own candidate order. */
  configs: string[];
}

/**
 * Detect directories that hold more than one config for the same Oxc tool.
 *
 * Both tools refuse to run in that state — `oxlint` and `oxfmt` each fail with
 * "Both '<a>' and '<b>' found in <dir>" — so such a project is already broken
 * before migration sees it. The rule is one config per directory, not one config
 * *form*: two JSON forms (`.oxlintrc.json` + `.oxlintrc.jsonc`) and two dynamic
 * forms (`oxlint.config.ts` + `oxlint.config.mts`) are rejected exactly like the
 * mixed pair. Migration cannot repair any of them either: first-match detection
 * would consume one config and leave the rest on disk unreferenced, where they
 * then silently shadow the freshly inlined `lint` block for direct `oxlint`
 * invocations. Erroring out and letting the user pick a single config first is
 * the only outcome that does not quietly lose settings.
 *
 * `dir` is `'.'` for the workspace root; other values are workspace-relative
 * package paths with forward slashes.
 */
export function detectOxcConfigConflicts(
  projectPath: string,
  relativeDir = '.',
): OxcConfigConflict[] {
  const conflicts: OxcConfigConflict[] = [];

  const tools = [
    { tool: 'oxlint', configFiles: OXLINT_CONFIG_FILES },
    { tool: 'oxfmt', configFiles: OXFMT_CONFIG_FILES },
  ] as const;

  for (const { tool, configFiles } of tools) {
    const configs = configFiles.filter((config) => fs.existsSync(path.join(projectPath, config)));

    if (configs.length > 1) {
      conflicts.push({ tool, dir: relativeDir, configs });
    }
  }

  return conflicts;
}

/**
 * Collect Oxc config conflicts across a workspace: the root directory plus every
 * workspace package. `packageDirs` holds workspace-relative paths with forward
 * slashes, matching `WorkspacePackage['path']`; pass an empty array for a
 * single-package project.
 */
export function collectOxcConfigConflicts(
  rootDir: string,
  packageDirs: readonly string[] = [],
): OxcConfigConflict[] {
  return [
    ...detectOxcConfigConflicts(rootDir),
    ...packageDirs.flatMap((packageDir) =>
      detectOxcConfigConflicts(path.join(rootDir, packageDir), packageDir),
    ),
  ];
}

/** Render one conflict as a user-facing line for the migration abort message. */
export function formatOxcConfigConflict(conflict: OxcConfigConflict): string {
  const location = conflict.dir === '.' ? 'the project root' : conflict.dir;
  const quoted = conflict.configs.map((file) => `\`${file}\``);
  // `a and b` for the common pair, `a, b and c` once a directory holds more.
  const files = [quoted.slice(0, -1).join(', '), quoted.at(-1)].filter(Boolean).join(' and ');
  return `${location} has ${files} — ${conflict.tool} allows only one config per directory.`;
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

  // Check for oxlint configs
  // https://oxc.rs/docs/guide/usage/linter/config.html#configuration-file-format
  for (const config of OXLINT_CONFIG_FILES) {
    if (fs.existsSync(path.join(projectPath, config))) {
      configs.oxlintConfig = config;
      break;
    }
  }

  // Check for oxfmt configs
  // https://oxc.rs/docs/guide/usage/formatter.html#configuration-file
  for (const config of OXFMT_CONFIG_FILES) {
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

import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import { type OxlintConfig } from 'oxlint';

import {
  hasConfigKey,
  mergeJsonConfig,
  mergeTsdownConfig,
  rewriteImportsInDirectory,
  rewriteScripts,
  wrapLazyPlugins,
} from '../../../binding/index.js';
import {
  createDefaultVitePlusLintConfig,
  ensureVitePlusImportRuleDefaults,
} from '../../oxlint-plugin-config.ts';
import { type WorkspacePackage } from '../../types/index.ts';
import { BASEURL_TSCONFIG_WARNING, VITE_PLUS_NAME } from '../../utils/constants.ts';
import { editJsonFile, isJsonFile, readJsonFile, writeJsonFile } from '../../utils/json.ts';
import { displayRelative } from '../../utils/path.ts';
import { hasBaseUrlInTsconfig } from '../../utils/tsconfig.ts';
import { detectConfigs, type ConfigFiles } from '../detector.ts';
import {
  collectInstalledPackageNames,
  readRulesYaml,
  sanitizeMigratedOxlintConfig,
} from '../migrator.ts';
import { type MigrationReport } from '../report.ts';
import {
  LINT_STAGED_JSON_CONFIG_FILES,
  LINT_STAGED_OTHER_CONFIG_FILES,
  infoMigration,
  warnMigration,
} from './shared.ts';

// Remove the "lint-staged" key from package.json after config has been
// successfully merged into vite.config.ts.
export function removeLintStagedFromPackageJson(packageJsonPath: string): void {
  editJsonFile<{ 'lint-staged'?: Record<string, string | string[]> }>(packageJsonPath, (pkg) => {
    if (pkg['lint-staged']) {
      delete pkg['lint-staged'];
      return pkg;
    }
    return undefined;
  });
}

// Migrate standalone lint-staged config files into staged in vite.config.ts.
// JSON-parseable files are inlined automatically; non-JSON files get a warning.
export function rewriteLintStagedConfigFile(projectPath: string, report?: MigrationReport): void {
  let hasUnsupported = false;

  for (const filename of LINT_STAGED_JSON_CONFIG_FILES) {
    const configPath = path.join(projectPath, filename);
    if (!fs.existsSync(configPath)) {
      continue;
    }
    if (filename === '.lintstagedrc' && !isJsonFile(configPath)) {
      warnMigration(
        `${displayRelative(configPath)} is not JSON format — please migrate to "staged" in vite.config.ts manually`,
        report,
      );
      hasUnsupported = true;
      continue;
    }
    // Merge the JSON config into vite.config.ts as "staged" and delete the file.
    // Skip if staged already exists in vite.config.ts (already migrated by rewritePackageJson).
    if (!hasStagedConfigInViteConfig(projectPath)) {
      const config = readJsonFile(configPath);
      const updated = rewriteScripts(JSON.stringify(config), readRulesYaml());
      const finalConfig = updated ? JSON.parse(updated) : config;
      if (!mergeStagedConfigToViteConfig(projectPath, finalConfig, true, report)) {
        // Merge failed — preserve the original config file so the user doesn't lose their rules
        continue;
      }
      fs.unlinkSync(configPath);
      if (report) {
        report.inlinedLintStagedConfigCount++;
      }
    } else {
      warnMigration(
        `${displayRelative(configPath)} found but "staged" already exists in vite.config.ts — please merge manually`,
        report,
      );
    }
  }
  // Non-JSON standalone files — warn
  for (const filename of LINT_STAGED_OTHER_CONFIG_FILES) {
    const configPath = path.join(projectPath, filename);
    if (!fs.existsSync(configPath)) {
      continue;
    }
    warnMigration(
      `${displayRelative(configPath)} — please migrate to "staged" in vite.config.ts manually`,
      report,
    );
    hasUnsupported = true;
  }
  if (hasUnsupported) {
    infoMigration(
      'Only "staged" in vite.config.ts is supported. See https://viteplus.dev/guide/migrate#lint-staged',
      report,
    );
  }
}

/**
 * Ensure vite.config.ts exists, create it if not
 * @returns The vite config filename
 */
function ensureViteConfig(
  projectPath: string,
  configs: ConfigFiles,
  silent = false,
  report?: MigrationReport,
): string {
  if (!configs.viteConfig) {
    configs.viteConfig = 'vite.config.ts';
    const viteConfigPath = path.join(projectPath, 'vite.config.ts');
    fs.writeFileSync(
      viteConfigPath,
      `import { defineConfig } from '${VITE_PLUS_NAME}';

export default defineConfig({});
`,
    );
    if (report) {
      report.createdViteConfigCount++;
    }
    if (!silent) {
      prompts.log.success(`✔ Created vite.config.ts in ${displayRelative(viteConfigPath)}`);
    }
  }
  return configs.viteConfig;
}

/**
 * Merge tsdown.config.* into vite.config.ts
 * - For JSON files: merge content directly into `pack` field and delete the JSON file
 * - For TS/JS files: import the config file
 */
export function mergeTsdownConfigFile(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
): void {
  const configs = detectConfigs(projectPath);
  if (!configs.tsdownConfig) {
    return;
  }
  const viteConfig = ensureViteConfig(projectPath, configs, silent, report);

  const fullViteConfigPath = path.join(projectPath, viteConfig);
  const fullTsdownConfigPath = path.join(projectPath, configs.tsdownConfig);

  // For JSON files, merge content directly and delete the file
  if (configs.tsdownConfig.endsWith('.json')) {
    mergeAndRemoveJsonConfig(projectPath, viteConfig, configs.tsdownConfig, 'pack', silent, report);
    return;
  }

  // For TS/JS files, import the config file
  const tsdownRelativePath = `./${configs.tsdownConfig}`;
  const result = mergeTsdownConfig(fullViteConfigPath, tsdownRelativePath);
  if (result.updated) {
    fs.writeFileSync(fullViteConfigPath, result.content);
    if (report) {
      report.tsdownImportCount++;
    }
    if (!silent) {
      prompts.log.success(
        `✔ Added import for ${displayRelative(fullTsdownConfigPath)} in ${displayRelative(fullViteConfigPath)}`,
      );
    }
  }
  // Show documentation link for manual merging since we only added the import
  infoMigration(
    `Please manually merge ${displayRelative(fullTsdownConfigPath)} into ${displayRelative(fullViteConfigPath)}, see https://viteplus.dev/guide/migrate#tsdown`,
    report,
  );
}

/**
 * Merge oxlint and oxfmt config into vite.config.ts
 */
export function mergeViteConfigFiles(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
  packages?: WorkspacePackage[],
  // For per-sub-package callers: the workspace root that `packages[].path`
  // is relative to. When undefined we resolve relative to `projectPath`
  // (correct for the top-level standalone/monorepo callers, where
  // projectPath IS the workspace root).
  workspaceRoot?: string,
): void {
  const configs = detectConfigs(projectPath);
  if (!configs.oxfmtConfig && !configs.oxlintConfig) {
    return;
  }
  const viteConfig = ensureViteConfig(projectPath, configs, silent, report);
  if (configs.oxlintConfig) {
    // Inject options.typeAware and options.typeCheck defaults before merging
    const fullOxlintPath = path.join(projectPath, configs.oxlintConfig);
    const oxlintJson = readJsonFile(fullOxlintPath, true) as OxlintConfig;
    if (!oxlintJson.options) {
      oxlintJson.options = {};
    }
    // Skip typeAware/typeCheck when tsconfig.json has baseUrl (unsupported by tsgolint)
    if (!hasBaseUrlInTsconfig(projectPath)) {
      if (oxlintJson.options.typeAware === undefined) {
        oxlintJson.options.typeAware = true;
      }
      if (oxlintJson.options.typeCheck === undefined) {
        oxlintJson.options.typeCheck = true;
      }
    } else {
      warnMigration(BASEURL_TSCONFIG_WARNING, report);
    }
    // Drop references to plugins / jsPlugins / rules that won't resolve
    // at lint time (e.g. `@oxlint/migrate` translating `@unocss/eslint-config`
    // → `eslint-plugin-unocss` even when that package isn't installed).
    // Resolve workspace package paths against `workspaceRoot` when the
    // caller is processing a sub-package — otherwise the sanitizer would
    // mistakenly look for `subPath/<sibling-pkg-path>` and miss the
    // hoisted deps it's supposed to see.
    sanitizeMigratedOxlintConfig(
      oxlintJson,
      collectInstalledPackageNames(workspaceRoot ?? projectPath, packages),
      report,
    );
    const normalizedOxlintConfig = ensureVitePlusImportRuleDefaults(oxlintJson);
    // writeJsonFile preserves the user file's existing indent/newline (and adds a
    // trailing newline) instead of forcing 2-space + no EOL.
    writeJsonFile(fullOxlintPath, normalizedOxlintConfig as Record<string, unknown>);
    // merge oxlint config into vite.config.ts
    mergeAndRemoveJsonConfig(projectPath, viteConfig, configs.oxlintConfig, 'lint', silent, report);
  }
  if (configs.oxfmtConfig) {
    // merge oxfmt config into vite.config.ts
    mergeAndRemoveJsonConfig(projectPath, viteConfig, configs.oxfmtConfig, 'fmt', silent, report);
  }
}

/**
 * Inject typeAware and typeCheck defaults into vite.config.ts lint config.
 * Called after mergeViteConfigFiles() to handle the case where no .oxlintrc.json exists
 * (e.g., newly created projects from create-vite templates).
 */
export function injectLintTypeCheckDefaults(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
): void {
  if (hasBaseUrlInTsconfig(projectPath)) {
    warnMigration(BASEURL_TSCONFIG_WARNING, report);
    return;
  }
  injectConfigDefaults(
    projectPath,
    'lint',
    '.vite-plus-lint-init.oxlintrc.json',
    JSON.stringify(
      createDefaultVitePlusLintConfig({
        includeTypeAwareDefaults: true,
      }),
    ),
    silent,
    report,
  );
}

export function injectFmtDefaults(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
): void {
  injectConfigDefaults(
    projectPath,
    'fmt',
    '.vite-plus-fmt-init.oxfmtrc.json',
    JSON.stringify({}),
    silent,
    report,
  );
}

/**
 * Wire `create.defaultTemplate: '<scope>'` into the new monorepo's
 * `vite.config.ts`. The caller is `bin.ts`, only when scaffolding a
 * monorepo from a bundled `@org` manifest entry — that's the case where
 * the user just picked a template from a specific org and naturally
 * wants subsequent `vp create` invocations from the workspace to default
 * to that same org's picker.
 */
export function injectCreateDefaultTemplate(
  projectPath: string,
  scope: string,
  silent = false,
  report?: MigrationReport,
): void {
  if (!scope) {
    return;
  }
  injectConfigDefaults(
    projectPath,
    'create',
    '.vite-plus-create-init.json',
    JSON.stringify({ defaultTemplate: scope }),
    silent,
    report,
  );
}

function injectConfigDefaults(
  projectPath: string,
  configKey: string,
  tempFileName: string,
  tempFileContent: string,
  silent: boolean,
  report?: MigrationReport,
): void {
  const configs = detectConfigs(projectPath);
  if (configs.viteConfig && hasConfigKey(path.join(projectPath, configs.viteConfig), configKey)) {
    return;
  }

  const viteConfig = ensureViteConfig(projectPath, configs, silent, report);
  const tempConfigPath = path.join(projectPath, tempFileName);
  fs.writeFileSync(tempConfigPath, tempFileContent);
  const fullViteConfigPath = path.join(projectPath, viteConfig);
  let result;
  try {
    result = mergeJsonConfig(fullViteConfigPath, tempConfigPath, configKey);
  } finally {
    fs.rmSync(tempConfigPath, { force: true });
  }
  if (result.updated) {
    fs.writeFileSync(fullViteConfigPath, result.content);
  }
}

function mergeAndRemoveJsonConfig(
  projectPath: string,
  viteConfigPath: string,
  jsonConfigPath: string,
  configKey: string,
  silent = false,
  report?: MigrationReport,
): void {
  const fullViteConfigPath = path.join(projectPath, viteConfigPath);
  const fullJsonConfigPath = path.join(projectPath, jsonConfigPath);
  // Skip merge when the key is already present in vite.config.ts — the Rust
  // merge step always prepends, so without this guard a template that ships
  // both an inline `${configKey}:` block and a standalone JSON file (e.g.
  // create-fate's vite.config.ts + .oxfmtrc.jsonc) ends up with two of them.
  // AST-based check ignores comments, string-literal occurrences, and nested
  // keys (e.g. `plugins: [{ fmt: ... }]`).
  if (hasConfigKey(fullViteConfigPath, configKey)) {
    fs.unlinkSync(fullJsonConfigPath);
    if (!silent) {
      prompts.log.info(
        `${configKey} config already present in ${displayRelative(fullViteConfigPath)} — removed redundant ${displayRelative(fullJsonConfigPath)}`,
      );
    }
    return;
  }
  const result = mergeJsonConfig(fullViteConfigPath, fullJsonConfigPath, configKey);
  if (result.updated) {
    fs.writeFileSync(fullViteConfigPath, result.content);
    fs.unlinkSync(fullJsonConfigPath);
    if (report) {
      report.mergedConfigCount++;
    }
    if (!silent) {
      prompts.log.success(
        `✔ Merged ${displayRelative(fullJsonConfigPath)} into ${displayRelative(fullViteConfigPath)}`,
      );
    }
  } else {
    warnMigration(
      `Failed to merge ${displayRelative(fullJsonConfigPath)} into ${displayRelative(fullViteConfigPath)}`,
      report,
    );
    infoMigration(
      'Please complete the merge manually and follow the instructions in the documentation: https://viteplus.dev/config/',
      report,
    );
  }
}

/**
 * Merge a staged config object into vite.config.ts as `staged: { ... }`.
 * Writes the config to a temp JSON file, calls mergeJsonConfig NAPI, then cleans up.
 */
export function mergeStagedConfigToViteConfig(
  projectPath: string,
  stagedConfig: Record<string, string | string[]>,
  silent = false,
  report?: MigrationReport,
): boolean {
  const configs = detectConfigs(projectPath);
  const viteConfig = ensureViteConfig(projectPath, configs, silent, report);
  const fullViteConfigPath = path.join(projectPath, viteConfig);

  // Write staged config to a temp JSON file for mergeJsonConfig NAPI
  const tempJsonPath = path.join(projectPath, '.staged-config-temp.json');
  fs.writeFileSync(tempJsonPath, JSON.stringify(stagedConfig, null, 2));

  let result;
  try {
    result = mergeJsonConfig(fullViteConfigPath, tempJsonPath, 'staged');
  } finally {
    fs.unlinkSync(tempJsonPath);
  }

  if (result.updated) {
    fs.writeFileSync(fullViteConfigPath, result.content);
    if (report) {
      report.mergedStagedConfigCount++;
    }
    if (!silent) {
      prompts.log.success(`✔ Merged staged config into ${displayRelative(fullViteConfigPath)}`);
    }
    return true;
  } else {
    warnMigration(
      `Failed to merge staged config into ${displayRelative(fullViteConfigPath)}`,
      report,
    );
    infoMigration(
      `Please add staged config to ${displayRelative(fullViteConfigPath)} manually, see https://viteplus.dev/guide/migrate#lint-staged`,
      report,
    );
    return false;
  }
}

/**
 * Check if vite.config.ts already has a `staged` config key.
 */
export function hasStagedConfigInViteConfig(projectPath: string): boolean {
  const configs = detectConfigs(projectPath);
  if (!configs.viteConfig) {
    return false;
  }
  const viteConfigPath = path.join(projectPath, configs.viteConfig);
  const content = fs.readFileSync(viteConfigPath, 'utf8');
  return /\bstaged\s*:/.test(content);
}

/**
 * Wrap safe inline Vite plugin arrays with lazyPlugins so check/lint/fmt do not
 * eagerly execute plugin factories while loading vite.config.ts.
 */
export function wrapLazyPluginsInViteConfig(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
): void {
  const configs = detectConfigs(projectPath);
  if (!configs.viteConfig) {
    return;
  }

  const viteConfigPath = path.join(projectPath, configs.viteConfig);
  const result = wrapLazyPlugins(viteConfigPath);
  if (!result.updated) {
    return;
  }

  fs.writeFileSync(viteConfigPath, result.content);
  if (report) {
    report.wrappedPluginConfigCount++;
  }
  if (!silent) {
    prompts.log.success(
      `✔ Wrapped inline Vite plugins with lazyPlugins in ${displayRelative(viteConfigPath)}`,
    );
  }
}

/**
 * Rewrite imports in all TypeScript/JavaScript files under a directory
 * This rewrites vite/vitest imports to @voidzero-dev/vite-plus
 * @param projectPath - The root directory to search for files
 */
export function rewriteAllImports(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
  preserveNuxtVitestImports = true,
): boolean {
  // Issue #2004: the Rust rewriter rewrites `vite` imports only in config entry
  // files. Standard basenames are recognized in-crate; also pass any config the
  // migrate resolved here so a custom-named config is honored.
  const configs = detectConfigs(projectPath);
  const extraConfigFiles = [configs.viteConfig, configs.vitestConfig]
    .filter((name): name is string => Boolean(name))
    .map((name) => path.join(projectPath, name));
  const result = rewriteImportsInDirectory(
    projectPath,
    preserveNuxtVitestImports,
    extraConfigFiles,
  );
  const modified = result.modifiedFiles.length;
  const preserved = result.preservedVitestFiles.length;
  const errors = result.errors.length;

  if (report) {
    report.rewrittenImportFileCount += modified;
    report.preservedUpstreamVitestImportFileCount += preserved;
    report.rewrittenImportErrors.push(
      ...result.errors.map((error) => ({
        path: displayRelative(error.path),
        message: error.message,
      })),
    );
  }

  if (!silent && modified > 0) {
    prompts.log.success(`Rewrote imports in ${modified === 1 ? 'one file' : `${modified} files`}`);
    prompts.log.info(result.modifiedFiles.map((file) => `  ${displayRelative(file)}`).join('\n'));
  }

  if (errors > 0) {
    if (report) {
      warnMigration(
        `${errors === 1 ? 'one file had an error' : `${errors} files had errors`} while rewriting imports`,
        report,
      );
    } else {
      prompts.log.warn(
        `⚠ ${errors === 1 ? 'one file had an error' : `${errors} files had errors`}:`,
      );
      for (const error of result.errors) {
        prompts.log.error(`  ${displayRelative(error.path)}: ${error.message}`);
      }
    }
  }
  return modified > 0;
}

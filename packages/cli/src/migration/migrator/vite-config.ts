import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import { braceExpand } from 'minimatch';
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

const SVELTE_RUNE_GLOBALS = {
  $state: 'readonly',
  $derived: 'readonly',
  $effect: 'readonly',
  $props: 'readonly',
  $bindable: 'readonly',
  $inspect: 'readonly',
  $host: 'readonly',
} satisfies Record<string, 'readonly'>;

// Add Svelte's built-in rune globals to migrated Svelte overrides.
// https://github.com/oxc-project/oxc/issues/20191
export function ensureSvelteRuneGlobals(config: OxlintConfig): void {
  for (const override of config.overrides ?? []) {
    const targetsSvelte = override.files.some(
      (file: string) =>
        !file.startsWith('!') && braceExpand(file).some((pattern) => pattern.includes('.svelte')),
    );
    if (!targetsSvelte) {
      continue;
    }
    override.globals = {
      ...SVELTE_RUNE_GLOBALS,
      ...override.globals,
    };
  }
}

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
): boolean {
  const configs = detectConfigs(projectPath);
  if (!configs.tsdownConfig) {
    return false;
  }
  const createdViteConfig = !configs.viteConfig;
  const viteConfig = ensureViteConfig(projectPath, configs, silent, report);

  const fullViteConfigPath = path.join(projectPath, viteConfig);
  const fullTsdownConfigPath = path.join(projectPath, configs.tsdownConfig);

  // For JSON files, merge content directly and delete the file
  if (configs.tsdownConfig.endsWith('.json')) {
    mergeAndRemoveJsonConfig(projectPath, viteConfig, configs.tsdownConfig, 'pack', silent, report);
    return createdViteConfig || !fs.existsSync(fullTsdownConfigPath);
  }

  // For TS/JS files, import the config file
  const tsdownRelativePath = `./${configs.tsdownConfig}`;
  // Do not prepend a second `pack` key when the config is already wired under
  // a different local import name. If a pack config exists but does not import
  // this tsdown config, leave both files untouched and keep the manual follow-up.
  if (hasConfigKey(fullViteConfigPath, 'pack')) {
    const viteConfigContent = fs.readFileSync(fullViteConfigPath, 'utf8');
    const runtimeImportPath = tsdownRelativePath
      .replace(/\.mts$/, '.mjs')
      .replace(/\.cts$/, '.cjs')
      .replace(/\.ts$/, '.js');
    const importsTsdownConfig = [tsdownRelativePath, runtimeImportPath].some(
      (importPath) =>
        viteConfigContent.includes(`from '${importPath}'`) ||
        viteConfigContent.includes(`from "${importPath}"`),
    );
    if (!importsTsdownConfig) {
      infoMigration(
        `Please manually merge ${displayRelative(fullTsdownConfigPath)} into ${displayRelative(fullViteConfigPath)}, see https://viteplus.dev/guide/migrate#tsdown`,
        report,
      );
    }
    return false;
  }

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
  return createdViteConfig || result.updated;
}

const CONFIG_MODULE_EXTENSIONS = ['.ts', '.mts', '.cts', '.tsx', '.js', '.mjs', '.cjs', '.jsx'];
const CONFIG_SOURCE_EXTENSIONS: Record<string, string[]> = {
  '.js': ['.ts', '.tsx'],
  '.mjs': ['.mts'],
  '.cjs': ['.cts'],
  '.jsx': ['.tsx'],
};

function resolveConfigReferences(directory: string, reference: string): string[] {
  if (!reference) {
    return [];
  }
  const filename = path.resolve(directory, reference);
  const extension = path.extname(filename);
  const candidates = [filename];
  // Vite resolves .js imports to TypeScript sources, as well as extensionless
  // imports and directory indexes. Inspect those sources before deleting JSON.
  for (const suffix of CONFIG_SOURCE_EXTENSIONS[extension] ?? []) {
    candidates.push(filename.slice(0, -extension.length) + suffix);
  }
  for (const suffix of CONFIG_MODULE_EXTENSIONS) {
    candidates.push(filename + suffix, path.join(filename, `index${suffix}`));
  }
  // Preserve references from all matching sources instead of relying on one
  // config loader's extension precedence when multiple candidates exist.
  const resolved = new Set<string>();
  for (const candidate of candidates) {
    if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) {
      resolved.add(fs.realpathSync(candidate));
    }
  }
  return [...resolved];
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
  const fullViteConfigPath = configs.viteConfig && path.join(projectPath, configs.viteConfig);
  const rootDir = workspaceRoot ?? projectPath;
  const projectPaths = new Set([
    rootDir,
    projectPath,
    ...(packages ?? []).map((pkg) => path.join(rootDir, pkg.path)),
  ]);
  const configReferences: string[] = [];
  const lintConfigPaths = new Set<string>();
  const preservedConfigs = new Set<string>();
  const modulePaths = new Set<string>();
  let unresolvedScriptConfig = false;
  for (const projectDir of projectPaths) {
    const packageJsonPath = path.join(projectDir, 'package.json');
    if (fs.existsSync(packageJsonPath)) {
      const pkg = readJsonFile(packageJsonPath) as { scripts?: Record<string, string> };
      for (const script of Object.values(pkg.scripts ?? {})) {
        configReferences.push(script);
        // Inspect config arguments without executing shell commands. An unknown
        // path (for example an environment variable or a preceding `cd`) means
        // we cannot safely decide that a standalone config is unused.
        const words = (script.match(/(?:[^\s"';&|]+|"[^"]*"|'[^']*')+/g) ?? []).map((word) =>
          word.replace(/(["'])(.*?)\1/g, '$2'),
        );
        for (let index = 0; index < words.length; index++) {
          const word = words[index];
          let reference: string;
          if (word === '--config' || word === '-c') {
            reference = words[++index] ?? '';
          } else if (word.startsWith('--config=')) {
            reference = word.slice('--config='.length);
          } else if (word.startsWith('-c')) {
            reference = word.slice(2).replace(/^=/, '');
          } else {
            continue;
          }
          const filenames = resolveConfigReferences(projectDir, reference);
          if (filenames.length === 0 || words.includes('cd')) {
            unresolvedScriptConfig = true;
          }
          for (const filename of filenames) {
            if (/\.jsonc?$/.test(filename)) {
              preservedConfigs.add(filename);
              lintConfigPaths.add(filename);
            } else if (CONFIG_MODULE_EXTENSIONS.includes(path.extname(filename))) {
              modulePaths.add(filename);
            } else {
              unresolvedScriptConfig = true;
            }
          }
        }
      }
    }
    const projectConfigs = detectConfigs(projectDir);
    if (projectConfigs.viteConfig) {
      modulePaths.add(fs.realpathSync(path.join(projectDir, projectConfigs.viteConfig)));
    }
    if (projectConfigs.oxlintConfig) {
      lintConfigPaths.add(fs.realpathSync(path.join(projectDir, projectConfigs.oxlintConfig)));
    }
  }
  for (const modulePath of modulePaths) {
    const content = fs.readFileSync(modulePath, 'utf8');
    configReferences.push(content);
    // Quoted local paths cover imports, re-exports, require(), dynamic import(),
    // and readFileSync/new URL calls. Extra matches only preserve more configs.
    // Canonical paths keep cycles, including symlink cycles, finite.
    for (const match of content.matchAll(/(["'`])((?:\.{1,2}\/|\/)[^"'`\r\n]+)\1/g)) {
      for (const filename of resolveConfigReferences(path.dirname(modulePath), match[2])) {
        if (/\.jsonc?$/.test(filename)) {
          preservedConfigs.add(filename);
          lintConfigPaths.add(filename);
        } else if (CONFIG_MODULE_EXTENSIONS.includes(path.extname(filename))) {
          modulePaths.add(filename);
        }
      }
    }
  }
  // Set iteration also visits newly discovered targets, once each, including
  // custom filenames. This follows transitive extends without looping on cycles.
  for (const lintConfigPath of lintConfigPaths) {
    if (!fs.existsSync(lintConfigPath)) {
      continue;
    }
    const json = readJsonFile(lintConfigPath, true) as { extends?: unknown } | null | undefined;
    if (Array.isArray(json?.extends) && json.extends.length > 0) {
      // JSON extends uses file paths; inline lint.extends requires config objects.
      // Keep both the extending config and every config in its inheritance chain.
      preservedConfigs.add(lintConfigPath);
      for (const extendedConfig of json.extends) {
        if (typeof extendedConfig === 'string') {
          const resolvedPath = path.resolve(path.dirname(lintConfigPath), extendedConfig);
          const extendedConfigPath = fs.existsSync(resolvedPath)
            ? fs.realpathSync(resolvedPath)
            : resolvedPath;
          preservedConfigs.add(extendedConfigPath);
          lintConfigPaths.add(extendedConfigPath);
        }
      }
    }
  }
  const canMergeConfig = (filename: string, configKey: string): boolean => {
    // An existing tool config can load the JSON file indirectly. Direct imports,
    // readFileSync calls, and package scripts can also use it without a tool key.
    // Keep these files intact, including their lint options. A filename match is
    // deliberately conservative because script paths can depend on shell state.
    return (
      !unresolvedScriptConfig &&
      (!fullViteConfigPath || !hasConfigKey(fullViteConfigPath, configKey)) &&
      !preservedConfigs.has(fs.realpathSync(path.join(projectPath, filename))) &&
      !configReferences.some((content) => content.includes(filename))
    );
  };
  if (configs.oxlintConfig && !canMergeConfig(configs.oxlintConfig, 'lint')) {
    configs.oxlintConfig = undefined;
  }
  if (configs.oxfmtConfig && !canMergeConfig(configs.oxfmtConfig, 'fmt')) {
    configs.oxfmtConfig = undefined;
  }
  if (!configs.oxlintConfig && !configs.oxfmtConfig) {
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
    ensureSvelteRuneGlobals(oxlintJson);
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
  // A config retained for imports, scripts, or extends must keep taking effect.
  if (
    (configKey === 'lint' && configs.oxlintConfig) ||
    (configKey === 'fmt' && configs.oxfmtConfig)
  ) {
    return;
  }
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
  // Directories of packages that own the Oxlint plugin API, captured before
  // `rewritePackageJson` stripped `oxlint` from their manifests. See
  // `collectOxlintOwnerDirs`.
  oxlintOwnerDirs: string[] = [],
): boolean {
  const result = rewriteImportsInDirectory(projectPath, preserveNuxtVitestImports, oxlintOwnerDirs);
  const modified = result.modifiedFiles.length;
  const preserved = result.preservedVitestFiles.length;
  const errors = result.errors.length;

  for (const warning of result.warnings) {
    warnMigration(`${displayRelative(warning.path)}: ${warning.message}`, report);
  }

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

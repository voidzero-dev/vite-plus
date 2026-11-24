import { randomUUID } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import type { ObjectExpression } from '@oxc-project/types';
import { log as promptLog } from '@voidzero-dev/vite-plus-prompts';
import { braceExpand } from 'minimatch';
import type { OxlintConfig } from 'oxlint';

import {
  hasConfigKey,
  mergeJsonConfig,
  mergeTsdownConfig,
  migrateTaskCacheConfig,
  readStaticLintConfig,
  rewriteImportsInDirectory,
  rewriteScripts,
  upsertJsonConfig,
  wrapLazyPlugins,
} from '../../../binding/index.js';
import {
  createDefaultVitePlusLintConfig,
  ensureVitePlusImportRuleDefaults,
  PREFER_VITE_PLUS_IMPORTS_RULE,
} from '../../oxlint-plugin-config.ts';
import type { WorkspacePackage } from '../../types/index.ts';
import { BASEURL_TSCONFIG_WARNING, VITE_PLUS_NAME } from '../../utils/constants.ts';
import { documentationUrl } from '../../utils/documentation.ts';
import { parseJsonValue, isObjectValue } from '../../utils/json-value.ts';
import type { JsonObject } from '../../utils/json.ts';
import { editJsonFile, isJsonFile, readJsonFile } from '../../utils/json.ts';
import { displayRelative } from '../../utils/path.ts';
import { hasBaseUrlInTsconfig } from '../../utils/tsconfig.ts';
import { parseSource, sameStaticValue } from '../ast-utils.ts';
import { readLintConfigInWorker } from '../compat/runner.ts';
import type { ConfigFiles } from '../detector.ts';
import { detectConfigs } from '../detector.ts';
import {
  collectInstalledPackageNames,
  oxlintMigrationMarker,
  prepareOxlintConfigs,
  readRulesYaml,
  removeMigratedOxlintConfigs,
  resolveOxlintConfigExtends,
  prepareOxlintConfigModules,
  sanitizeMigratedOxlintConfig,
  sanitizeInheritedJsPlugins,
} from '../migrator.ts';
import type { MigrationReport } from '../report.ts';
import type { OxlintConfigMigration, OxlintModulePlan, PreparedOxlintConfigs } from './shared.ts';
import { rewriteScriptMap } from './shared.ts';
import {
  DISABLED_IMPORT_RULE_CONFIG,
  OXLINT_OWNER_CONFIG_KEYS,
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

function oxlintConfigLayers(config: OxlintConfig): Array<OxlintConfig> {
  return [...(config.extends ?? []).flatMap(oxlintConfigLayers), config];
}

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
  editJsonFile<{ 'lint-staged'?: Record<string, string | Array<string>> }>(
    packageJsonPath,
    (pkg) => {
      if (pkg['lint-staged']) {
        delete pkg['lint-staged'];
        return pkg;
      }
      return undefined;
    },
  );
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
      const updated = rewriteScriptMap(config, (serialized) =>
        rewriteScripts(serialized, readRulesYaml()),
      );
      const finalConfig = updated ?? config;
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
      `Only "staged" in vite.config.ts is supported. See ${documentationUrl('/guide/migrate#lint-staged')}`,
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
      promptLog.success(`✔ Created vite.config.ts in ${displayRelative(viteConfigPath)}`);
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
        `Please manually merge ${displayRelative(fullTsdownConfigPath)} into ${displayRelative(fullViteConfigPath)}, see ${documentationUrl('/guide/migrate#tsdown')}`,
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
      promptLog.success(
        `✔ Added import for ${displayRelative(fullTsdownConfigPath)} in ${displayRelative(fullViteConfigPath)}`,
      );
    }
  }
  // Show documentation link for manual merging since we only added the import
  infoMigration(
    `Please manually merge ${displayRelative(fullTsdownConfigPath)} into ${displayRelative(fullViteConfigPath)}, see ${documentationUrl('/guide/migrate#tsdown')}`,
    report,
  );
  return createdViteConfig || result.updated;
}

/**
 * Merge oxlint and oxfmt config into vite.config.ts
 */
export function mergeViteConfigFiles(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
  packages?: Array<WorkspacePackage>,
  // For per-sub-package callers: the workspace root that `packages[].path`
  // is relative to. When undefined we resolve relative to `projectPath`
  // (correct for the top-level standalone/monorepo callers, where
  // projectPath IS the workspace root).
  workspaceRoot?: string,
  preparedOxlintConfigs?: PreparedOxlintConfigs,
): void {
  const configs = detectConfigs(projectPath);
  if (!configs.oxfmtConfig && !configs.oxlintConfig) {
    return;
  }
  const viteConfig = ensureViteConfig(projectPath, configs, silent, report);
  if (configs.oxlintConfig) {
    mergeOxlintConfigFile(
      projectPath,
      viteConfig,
      configs.oxlintConfig,
      silent,
      report,
      packages,
      workspaceRoot,
      preparedOxlintConfigs,
    );
  }
  if (configs.oxfmtConfig) {
    // merge oxfmt config into vite.config.ts
    mergeAndRemoveJsonConfig(projectPath, viteConfig, configs.oxfmtConfig, 'fmt', silent, report);
  }
}

function pendingOxlintMigrationMarker(filename: string): string {
  return `// vp migrate: pending ${encodeURIComponent(path.basename(filename))}.`;
}

/** Resume partial migrations without applying lint setup to other existing Vite+ projects. */
export function retryPreservedOxlintConfigs(
  workspaceRoot: string,
  silent = false,
  report?: MigrationReport,
  packages?: Array<WorkspacePackage>,
): boolean {
  const prepared = prepareOxlintConfigs([
    workspaceRoot,
    ...(packages ?? []).map((pkg) => path.join(workspaceRoot, pkg.path)),
  ]);
  let updated = false;
  for (const [filename, migration] of prepared) {
    const projectPath = path.dirname(filename);
    const configs = detectConfigs(projectPath);
    const existingViteConfig = configs.viteConfig && path.join(projectPath, configs.viteConfig);
    const source = existingViteConfig ? fs.readFileSync(existingViteConfig, 'utf8') : '';
    const lines = source.split(/\r?\n/);
    if (existingViteConfig && hasConfigKey(existingViteConfig, 'lint')) {
      if (!(migration instanceof Error)) {
        const marker = oxlintMigrationMarker(filename, migration.file, workspaceRoot);
        migration.migrated = lines.includes(marker);
      }
      continue;
    }
    if (!lines.includes(pendingOxlintMigrationMarker(filename))) {
      continue;
    }
    const viteConfig = ensureViteConfig(projectPath, configs, silent, report);
    mergeOxlintConfigFile(
      projectPath,
      viteConfig,
      path.basename(filename),
      silent,
      report,
      packages,
      workspaceRoot,
      prepared,
    );
    updated ||= !(migration instanceof Error) && migration.migrated === true;
  }
  return removeMigratedOxlintConfigs(prepared) || updated;
}

function mergeOxlintConfigFile(
  projectPath: string,
  viteConfig: string,
  oxlintConfig: string,
  silent: boolean,
  report?: MigrationReport,
  packages?: Array<WorkspacePackage>,
  workspaceRoot?: string,
  preparedOxlintConfigs?: PreparedOxlintConfigs,
): void {
  const fullOxlintPath = path.join(projectPath, oxlintConfig);
  function recordPendingMigration(): void {
    const filename = path.join(projectPath, viteConfig);
    const marker = pendingOxlintMigrationMarker(oxlintConfig);
    try {
      const source = fs.readFileSync(filename, 'utf8');
      if (!source.split(/\r?\n/).includes(marker)) {
        fs.appendFileSync(filename, `${source.endsWith('\n') ? '' : '\n'}${marker}\n`);
      }
    } catch (error) {
      warnMigration(
        `Could not record pending migration in ${displayRelative(filename)}: ${String(error)}`,
        report,
      );
    }
  }
  function warnPreservedConfig(error: unknown): void {
    recordPendingMigration();
    warnMigration(
      `Could not migrate ${displayRelative(fullOxlintPath)}: ${String(error)}. The original config was preserved; use vp lint -c ${displayRelative(fullOxlintPath)} until it is migrated manually.`,
      report,
    );
  }
  let migration: OxlintConfigMigration;
  try {
    const prepared = preparedOxlintConfigs?.get(fullOxlintPath);
    if (prepared instanceof Error) {
      throw prepared;
    }
    migration = prepared ?? { file: resolveOxlintConfigExtends(fullOxlintPath) };
  } catch (error) {
    warnPreservedConfig(error);
    return;
  }
  if (migration.migrated) {
    return;
  }
  const { file } = migration;
  // Keep shared rules in one module, including for the workspace that owns it.
  // Fields that Oxlint does not inherit must also remain in the owner's local block.
  const sourceConfig: OxlintConfig = structuredClone(file.config);
  let oxlintJson = sourceConfig;
  if (file.shared) {
    oxlintJson = {
      extends: [oxlintJson],
      ...Object.fromEntries(
        Object.entries(file.config).filter(
          ([key, value]) => OXLINT_OWNER_CONFIG_KEYS.has(key) && value !== undefined,
        ),
      ),
    };
  }
  // Never discard an inheritance tree when a lint block already exists.
  if (oxlintJson.extends?.length && hasConfigKey(path.join(projectPath, viteConfig), 'lint')) {
    recordPendingMigration();
    warnMigration(
      `Preserved ${displayRelative(fullOxlintPath)} because ${viteConfig} already has a lint config. Merge its inherited rules manually.`,
      report,
    );
    return;
  }
  const layers = oxlintConfigLayers(oxlintJson);
  const inheritedOptions: NonNullable<OxlintConfig['options']> = {};
  for (const layer of layers) {
    Object.assign(inheritedOptions, layer.options);
  }
  oxlintJson.options ||= {};
  // Skip typeAware/typeCheck when tsconfig.json has baseUrl (unsupported by TypeScript 7)
  if (!hasBaseUrlInTsconfig(projectPath)) {
    if (inheritedOptions.typeAware === undefined) {
      oxlintJson.options.typeAware = true;
    }
    if (inheritedOptions.typeCheck === undefined) {
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
  // A pre-existing inheritance tree can declare JS plugins in another layer,
  // including local plugins whose namespace is only known at runtime. The
  // ESLint migration sanitizer cannot validate these rules independently.
  if (!oxlintJson.extends?.length) {
    sanitizeMigratedOxlintConfig(
      oxlintJson,
      collectInstalledPackageNames(workspaceRoot ?? projectPath, packages),
      report,
    );
  } else {
    sanitizeInheritedJsPlugins(
      sourceConfig,
      collectInstalledPackageNames(workspaceRoot ?? projectPath, packages),
      path.dirname(file.filename),
      report,
    );
  }
  for (const layer of layers) {
    ensureSvelteRuneGlobals(layer);
  }
  const hasInheritedImportRule = layers.some(
    (layer) => layer.rules?.[PREFER_VITE_PLUS_IMPORTS_RULE] !== undefined,
  );
  const normalizedOxlintConfig = ensureVitePlusImportRuleDefaults(oxlintJson);
  if (hasInheritedImportRule && oxlintJson.rules?.[PREFER_VITE_PLUS_IMPORTS_RULE] === undefined) {
    // Keep the shared rule live: copying its value into the root would mask later edits.
    delete normalizedOxlintConfig.rules?.[PREFER_VITE_PLUS_IMPORTS_RULE];
  }
  try {
    const plan: OxlintModulePlan =
      file.shared || file.extends.length > 0
        ? prepareOxlintConfigModules(
            file,
            normalizedOxlintConfig,
            path.join(projectPath, viteConfig),
            workspaceRoot ?? projectPath,
          )
        : {
            configContent: JSON.stringify(normalizedOxlintConfig, null, 2),
            imports: [],
            modules: new Map(),
          };
    if (preparedOxlintConfigs) {
      plan.completionMarker = oxlintMigrationMarker(
        fullOxlintPath,
        file,
        workspaceRoot ?? projectPath,
      );
    }
    migration.migrated = mergeAndRemoveJsonConfig(
      projectPath,
      viteConfig,
      oxlintConfig,
      'lint',
      silent,
      report,
      plan,
      preparedOxlintConfigs !== undefined,
    );
    if (!migration.migrated) {
      recordPendingMigration();
    }
  } catch (error) {
    warnPreservedConfig(error);
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
  // An unresolved JSON inheritance tree needs manual attention. Do not mask it
  // with a new lint block, which would also prevent a later migration retry.
  if (detectConfigs(projectPath).oxlintConfig) {
    return;
  }
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

function configImportOffset(filename: string, source: string): number {
  const { program, comments } = parseSource(filename, source);
  let offset = program.hashbang?.end ?? 0;
  for (const statement of program.body) {
    if (statement.type !== 'ExpressionStatement' || typeof statement.directive !== 'string') {
      break;
    }
    offset = statement.end;
  }
  const firstStatement = program.body[0]?.start ?? source.length;
  for (const comment of comments) {
    if (comment.end > firstStatement) {
      break;
    }
    const text = source.slice(comment.start, comment.end);
    // File pragmas must precede imports; next-statement directives must stay in place.
    if (
      /^\/\/\/\s*</.test(text) ||
      /@(?:ts-(?:no)?check|jsx(?:Frag|Runtime|ImportSource)?)\b/.test(text)
    ) {
      offset = Math.max(offset, comment.end);
    }
  }
  return offset;
}

// The native parser supplies AST nodes tagged with their ESTree type.
function isAstObjectExpression(node: unknown): node is ObjectExpression {
  return (
    typeof node === 'object' &&
    node !== null &&
    'type' in node &&
    node.type === 'ObjectExpression' &&
    'properties' in node &&
    Array.isArray(node.properties)
  );
}

function staticValue(source: string) {
  const statement = parseSource('value.ts', `const value = ${source};`).program.body[0];
  return statement?.type === 'VariableDeclaration' ? statement.declarations[0]?.init : undefined;
}

/**
 * Whether the property whose value starts at `offset` follows a spread in its
 * object. Returns `undefined` when no object literal holds such a property.
 */
function followsSpread(node: unknown, offset: number): boolean | undefined {
  if (typeof node !== 'object' || node === null) {
    return undefined;
  }
  if (isAstObjectExpression(node)) {
    const { properties } = node;
    const index = properties.findIndex(
      (property) => property.type === 'Property' && property.value.start === offset,
    );
    if (index !== -1) {
      return properties.slice(0, index).some((property) => property.type === 'SpreadElement');
    }
  }
  for (const child of Object.values(node)) {
    const found = followsSpread(child, offset);
    if (found !== undefined) {
      return found;
    }
  }
  return undefined;
}

/**
 * Whether the existing `configKey` block holds only the defaults that Vite+
 * writes on its own. A tool config merged later already carries these defaults,
 * so it can replace such a block without losing anything the user wrote.
 */
export function hasOnlyInjectedDefaults(viteConfigPath: string, configKey: string): boolean {
  // The blocks `injectFmtDefaults` and `injectLintTypeCheckDefaults` write.
  const injected =
    configKey === 'fmt'
      ? {}
      : configKey === 'lint'
        ? createDefaultVitePlusLintConfig({ includeTypeAwareDefaults: true })
        : undefined;
  if (!injected) {
    return false;
  }
  // shortcut: a placeholder upsert marks the range of the existing value, so
  // this uses the same key lookup as the replacement. Add a binding that
  // returns the value when another caller needs to read one.
  const placeholder = `"${randomUUID()}"`;
  const placeholderPath = path.join(os.tmpdir(), `vite-plus-config-${randomUUID()}.json`);
  fs.writeFileSync(placeholderPath, placeholder);
  let marked: Array<string>;
  try {
    marked = upsertJsonConfig(viteConfigPath, placeholderPath, configKey).content.split(
      placeholder,
    );
  } finally {
    fs.rmSync(placeholderPath, { force: true });
  }
  const source = fs.readFileSync(viteConfigPath, 'utf8');
  const [before, after] = marked;
  // Several config objects, an inserted key, or a shorthand property do not
  // yield one replaced value.
  if (
    marked.length !== 2 ||
    before.length + after.length > source.length ||
    !source.startsWith(before) ||
    !source.endsWith(after)
  ) {
    return false;
  }
  try {
    // Vite+ prepends its blocks, so one that follows a spread was written by
    // the user to override it. Replacing that block could move it before the
    // spread, where the spread wins over the converted settings.
    if (followsSpread(parseSource(viteConfigPath, source).program, before.length) !== false) {
      return false;
    }
    const existing = staticValue(source.slice(before.length, source.length - after.length));
    const expected = staticValue(JSON.stringify(injected));
    return !!existing && !!expected && sameStaticValue(existing, expected);
  } catch {
    return false;
  }
}

function mergeAndRemoveJsonConfig(
  projectPath: string,
  viteConfigPath: string,
  jsonConfigPath: string,
  configKey: string,
  silent = false,
  report?: MigrationReport,
  modulePlan?: OxlintModulePlan,
  deferRemoval = false,
): boolean {
  const fullViteConfigPath = path.join(projectPath, viteConfigPath);
  const fullJsonConfigPath = path.join(projectPath, jsonConfigPath);
  // Skip merge when the key is already present in vite.config.ts — the Rust
  // merge step always prepends, so without this guard a template that ships
  // both an inline `${configKey}:` block and a standalone JSON file (e.g.
  // create-fate's vite.config.ts + .oxfmtrc.jsonc) ends up with two of them.
  // AST-based check ignores comments, string-literal occurrences, and nested
  // keys (e.g. `plugins: [{ fmt: ... }]`).
  const keyExists = hasConfigKey(fullViteConfigPath, configKey);
  // A block of injected defaults is not a second copy of the JSON config. The
  // first `vp migrate` writes one even when ESLint stays, so a later `--full`
  // run must replace it with the converted config instead. Prettier takes the
  // same decision before its conversion, see `migratePrettierToOxfmt`.
  const replaceDefaults = keyExists && hasOnlyInjectedDefaults(fullViteConfigPath, configKey);
  if (keyExists && !replaceDefaults) {
    // This run converted the JSON from ESLint and already removed the ESLint
    // config, so the JSON is the only copy of those rules. The pending marker
    // keeps that true for later runs, until the user merges the rules.
    if (
      configKey === 'lint' &&
      (report?.eslintMigrated ||
        fs
          .readFileSync(fullViteConfigPath, 'utf8')
          .split(/\r?\n/)
          .includes(pendingOxlintMigrationMarker(jsonConfigPath)))
    ) {
      warnMigration(
        `Preserved ${displayRelative(fullJsonConfigPath)} because ${viteConfigPath} already has a ${configKey} config. Merge it manually.`,
        report,
      );
      return false;
    }
    if (!deferRemoval) {
      fs.unlinkSync(fullJsonConfigPath);
    }
    if (!silent) {
      promptLog.info(
        `${configKey} config already present in ${displayRelative(fullViteConfigPath)} — ${deferRemoval ? 'deferred removal of' : 'removed redundant'} ${displayRelative(fullJsonConfigPath)}`,
      );
    }
    return true;
  }
  // Only delete the original JSON after a successful merge. Resolved lint
  // configs contain object extends, which JSON Oxlint configs do not accept.
  const mergeConfig = replaceDefaults ? upsertJsonConfig : mergeJsonConfig;
  let result;
  if (!modulePlan) {
    result = mergeConfig(fullViteConfigPath, fullJsonConfigPath, configKey);
  } else {
    const tempDir = fs.mkdtempSync(path.join(projectPath, '.vite-plus-lint-'));
    try {
      const tempConfig = path.join(tempDir, 'config.json');
      fs.writeFileSync(tempConfig, modulePlan.configContent);
      result = mergeConfig(fullViteConfigPath, tempConfig, configKey);
    } finally {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  }
  if (!result.updated) {
    warnMigration(
      `Failed to merge ${displayRelative(fullJsonConfigPath)} into ${displayRelative(fullViteConfigPath)}`,
      report,
    );
    infoMigration(
      `Please complete the merge manually and follow the instructions in the documentation: ${documentationUrl('/config/')}`,
      report,
    );
    return false;
  }

  const originalViteConfig = fs.readFileSync(fullViteConfigPath, 'utf8');
  const writtenModules: Array<string> = [];
  try {
    for (const [filename, module] of modulePlan?.modules ?? []) {
      const descriptor = fs.openSync(filename, 'wx');
      writtenModules.push(filename);
      try {
        fs.writeFileSync(descriptor, module.content);
      } finally {
        fs.closeSync(descriptor);
      }
    }
    let content = result.content;
    if (configKey === 'lint') {
      const pendingMarker = pendingOxlintMigrationMarker(jsonConfigPath);
      content = content
        .split(/\r?\n/)
        .filter((line) => line !== pendingMarker)
        .join('\n');
    }
    if (modulePlan && (modulePlan.imports.length || modulePlan.completionMarker)) {
      const offset = configImportOffset(fullViteConfigPath, content);
      const prefix = [modulePlan.completionMarker, ...modulePlan.imports]
        .filter(Boolean)
        .join('\n');
      const header = content.slice(0, offset);
      content =
        header +
        (offset && !header.endsWith('\n') ? '\n' : '') +
        prefix +
        '\n' +
        content.slice(offset);
    }
    fs.writeFileSync(fullViteConfigPath, content);
    if (!deferRemoval) {
      fs.unlinkSync(fullJsonConfigPath);
    }
  } catch (error) {
    for (const filename of writtenModules) {
      fs.unlinkSync(filename);
    }
    fs.writeFileSync(fullViteConfigPath, originalViteConfig);
    throw error;
  }
  for (const [filename, module] of modulePlan?.modules ?? []) {
    module.file.modulePath = filename;
  }
  if (report) {
    report.mergedConfigCount++;
  }
  if (!silent) {
    promptLog.success(
      `✔ Merged ${displayRelative(fullJsonConfigPath)} into ${displayRelative(fullViteConfigPath)}`,
    );
  }
  return true;
}

/**
 * Merge a staged config object into vite.config.ts as `staged: { ... }`.
 * Writes the config to a temp JSON file, calls mergeJsonConfig NAPI, then cleans up.
 */
export function mergeStagedConfigToViteConfig(
  projectPath: string,
  stagedConfig: JsonObject,
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
      promptLog.success(`✔ Merged staged config into ${displayRelative(fullViteConfigPath)}`);
    }
    return true;
  } else {
    warnMigration(
      `Failed to merge staged config into ${displayRelative(fullViteConfigPath)}`,
      report,
    );
    infoMigration(
      `Please add staged config to ${displayRelative(fullViteConfigPath)} manually, see ${documentationUrl('/guide/migrate#lint-staged')}`,
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
    promptLog.success(
      `✔ Wrapped inline Vite plugins with lazyPlugins in ${displayRelative(viteConfigPath)}`,
    );
  }
}

/**
 * Move `env`, `untrackedEnv`, `input`, and `output` from the top level of
 * each `run.tasks` entry into its `cache` object, as Vite Task requires.
 * Tasks that need manual changes are warned about, or collected into
 * `manualWarnings` when the caller reports them later.
 */
export function migrateTaskCacheConfigInViteConfig(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
  manualWarnings?: Array<string>,
): boolean {
  const configs = detectConfigs(projectPath);
  if (!configs.viteConfig) {
    return false;
  }

  const viteConfigPath = path.join(projectPath, configs.viteConfig);
  const result = migrateTaskCacheConfig(viteConfigPath);
  if (result.manualTasks.length > 0) {
    const tasks = result.manualTasks.map((task) => `\`${task}\``).join(', ');
    const warning = `${displayRelative(viteConfigPath)}: Move \`env\`, \`untrackedEnv\`, \`input\`, and \`output\` under \`cache\` manually in ${
      result.manualTasks.length === 1 ? 'task' : 'tasks'
    } ${tasks}; ${result.manualTasks.length === 1 ? 'it was' : 'they were'} left unchanged. See ${documentationUrl('/config/run#cache')}`;
    if (manualWarnings) {
      manualWarnings.push(warning);
    } else {
      warnMigration(warning, report);
    }
  }
  if (!result.updated) {
    return false;
  }

  fs.writeFileSync(viteConfigPath, result.content);
  if (report) {
    report.migratedTaskCacheConfigCount++;
  }
  if (!silent) {
    promptLog.success(
      `✔ Moved task cache settings under \`cache\` in ${displayRelative(viteConfigPath)}`,
    );
  }
  return true;
}

function importRuleEnabled(rules: OxlintConfig['rules']): boolean | undefined {
  const rule = rules?.[PREFER_VITE_PLUS_IMPORTS_RULE];
  const severity = Array.isArray(rule) ? rule[0] : rule;
  switch (severity) {
    case undefined:
      return undefined;
    case 0:
    case 'off':
    case 'allow':
      return false;
    case 1:
    case 2:
    case 'warn':
    case 'error':
    case 'deny':
      return true;
    default:
      throw new Error(
        `Invalid severity for ${PREFER_VITE_PLUS_IMPORTS_RULE}: ${JSON.stringify(rule)}`,
      );
  }
}

export function warnUnresolvedImportRuleConfig(report?: MigrationReport): void {
  warnMigration(
    'Could not load lint configuration in an isolated process. Skipped Vite/Vitest import rewrites; fix the config and re-run `vp migrate`.',
    report,
  );
}

/**
 * Read the import rule before migration merges or removes lint configs.
 * Return null if the worker cannot load the project config.
 */
export async function tryResolveImportRuleConfig(projectPath: string): Promise<string | null> {
  const extracted = readStaticLintConfig(projectPath);
  let lint: OxlintConfig | undefined;
  if (typeof extracted === 'string') {
    const parsed = parseJsonValue(extracted);
    if (isObjectValue(parsed)) {
      lint = parsed;
    }
  } else {
    try {
      lint = await readLintConfigInWorker(projectPath);
    } catch {
      return null;
    }
  }
  if (!lint) {
    const filename = detectConfigs(projectPath).oxlintConfig;
    if (filename) {
      try {
        lint = resolveOxlintConfigExtends(path.join(projectPath, filename)).config;
      } catch {
        // The config migration reports and preserves unresolved JSON configs.
      }
    }
  }

  const layers = lint ? oxlintConfigLayers(lint) : [];
  const enabled = layers.reduce(
    (previous, layer) => importRuleEnabled(layer.rules) ?? previous,
    true,
  );
  const overrides = [];
  for (const layer of layers) {
    for (const override of layer.overrides ?? []) {
      const overrideEnabled = importRuleEnabled(override.rules);
      if (overrideEnabled !== undefined) {
        overrides.push({
          files: override.files,
          excludeFiles: override.excludeFiles ?? [],
          enabled: overrideEnabled,
        });
      }
    }
  }
  return JSON.stringify({ enabled, overrides });
}

/** Preserve imports if the last attempt to read the rule fails. */
export async function resolveImportRuleConfig(
  projectPath: string,
  report?: MigrationReport,
): Promise<string> {
  const config = await tryResolveImportRuleConfig(projectPath);
  if (config !== null) {
    return config;
  }
  warnUnresolvedImportRuleConfig(report);
  return DISABLED_IMPORT_RULE_CONFIG;
}

/** Rewrite toolchain imports throughout the project and record the results. */
export function rewriteAllImports(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
  options: {
    preserveNuxtVitestImports?: boolean;
    // Capture package owners before migration removes oxlint from their manifests.
    oxlintOwnerDirs?: Array<string>;
    importRuleConfig?: string;
  } = {},
): boolean {
  const result = rewriteImportsInDirectory(
    projectPath,
    options.preserveNuxtVitestImports ?? true,
    options.oxlintOwnerDirs ?? [],
    options.importRuleConfig,
  );
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
    promptLog.success(`Rewrote imports in ${modified === 1 ? 'one file' : `${modified} files`}`);
    promptLog.info(result.modifiedFiles.map((file) => `  ${displayRelative(file)}`).join('\n'));
  }

  if (errors > 0) {
    if (report) {
      warnMigration(
        `${errors === 1 ? 'one file had an error' : `${errors} files had errors`} while rewriting imports`,
        report,
      );
    } else {
      promptLog.warn(`⚠ ${errors === 1 ? 'one file had an error' : `${errors} files had errors`}:`);
      for (const error of result.errors) {
        promptLog.error(`  ${displayRelative(error.path)}: ${error.message}`);
      }
    }
  }
  return modified > 0;
}

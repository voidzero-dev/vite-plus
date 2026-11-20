import { randomUUID } from 'node:crypto';
import fs from 'node:fs';
import { isBuiltin } from 'node:module';
import os from 'node:os';
import path from 'node:path';

import type {
  ArrowFunctionExpression,
  Function as FunctionNode,
  Node,
  ObjectExpression,
  ObjectProperty,
  Program,
  Span,
} from '@oxc-project/types';
import * as prompts from '@voidzero-dev/vite-plus-prompts';
import { braceExpand } from 'minimatch';
import { type OxlintConfig } from 'oxlint';

import {
  analyzeMigrationSource,
  hasConfigKey,
  mergeJsonConfig,
  mergeTsdownConfig,
  mergeTsdownJsonConfig,
  migrateTaskCacheConfig,
  normalizeTsdownJsonPackConfigs,
  readStaticLintConfig,
  ResolverFactory,
  rewriteImportsInDirectory,
  rewriteScripts,
  rewriteTsdownConfigForPack,
  upsertJsonConfig,
  wrapLazyPlugins,
} from '../../../binding/index.js';
import {
  createDefaultVitePlusLintConfig,
  ensureVitePlusImportRuleDefaults,
  PREFER_VITE_PLUS_IMPORTS_RULE,
} from '../../oxlint-plugin-config.ts';
import { findViteConfig, findViteConfigUp } from '../../resolve-vite-config.ts';
import { type WorkspacePackage } from '../../types/index.ts';
import { BASEURL_TSCONFIG_WARNING, VITE_PLUS_NAME } from '../../utils/constants.ts';
import { documentationUrl } from '../../utils/documentation.ts';
import { editJsonFile, isJsonFile, readJsonFile } from '../../utils/json.ts';
import { displayRelative } from '../../utils/path.ts';
import { hasBaseUrlInTsconfig } from '../../utils/tsconfig.ts';
import {
  configReturnValues,
  defaultConfigValue,
  isString,
  memberName,
  objectProperty,
  parseSource,
  propertyName,
  sameConfigModule,
  sameStaticValue,
  staticObject,
  unwrapTypeExpression,
} from '../ast-utils.ts';
import { readLintConfigInWorker } from '../compat/runner.ts';
import { detectConfigs, type ConfigFiles } from '../detector.ts';
import {
  collectInstalledPackageNames,
  preparePackConfigTsconfigEdits,
  oxlintMigrationMarker,
  prepareOxlintConfigs,
  getScriptRulesYaml,
  removeMigratedOxlintConfigs,
  resolveOxlintConfigExtends,
  prepareOxlintConfigModules,
  sanitizeMigratedOxlintConfig,
  sanitizeInheritedJsPlugins,
} from '../migrator.ts';
import { type MigrationReport } from '../report.ts';
import {
  type OxlintConfigMigration,
  type OxlintModulePlan,
  type PreparedOxlintConfigs,
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

function oxlintConfigLayers(config: OxlintConfig): OxlintConfig[] {
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
export function rewriteLintStagedConfigFile(
  projectPath: string,
  report?: MigrationReport,
  keepTsdown = false,
): void {
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
      const updated = rewriteScripts(JSON.stringify(config), getScriptRulesYaml(false, keepTsdown));
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
      prompts.log.success(`✔ Created vite.config.ts in ${displayRelative(viteConfigPath)}`);
    }
  }
  return configs.viteConfig;
}

/**
 * Merge tsdown.config.* into vite.config.ts. Import TS/JS configs and inline JSON.
 * Keep the original config when standalone tsdown may still load it.
 * Convert direct pack imports of retained configs in other workspace packages.
 */
export function mergeTsdownConfigFile(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
  keepTsdownConfig = false,
  workspaceRoot = projectPath,
  tsdownPackages?: ReadonlySet<string>,
): boolean {
  const configs = detectConfigs(projectPath);
  const mergedLocal = mergeLocalTsdownConfigFile(
    projectPath,
    configs,
    workspaceRoot,
    keepTsdownConfig,
    silent,
    report,
  );
  const mergedImports =
    configs.viteConfig && tsdownPackages?.size
      ? mergeImportedTsdownConfigs(
          path.join(projectPath, configs.viteConfig),
          workspaceRoot,
          tsdownPackages,
          silent,
          report,
        )
      : false;
  return mergedLocal || mergedImports;
}

function mergeLocalTsdownConfigFile(
  projectPath: string,
  configs: ConfigFiles,
  workspaceRoot: string,
  keepTsdownConfig: boolean,
  silent: boolean,
  report: MigrationReport | undefined,
): boolean {
  if (!configs.tsdownConfig) {
    return false;
  }
  const createdViteConfig = !configs.viteConfig;
  const viteConfig = ensureViteConfig(projectPath, configs, silent, report);

  const fullViteConfigPath = path.join(projectPath, viteConfig);
  const fullTsdownConfigPath = path.join(projectPath, configs.tsdownConfig);

  if (configs.tsdownConfig.endsWith('.json')) {
    if (hasConfigKey(fullViteConfigPath, 'pack')) {
      const matching = matchingInlinePackConfig(
        fullViteConfigPath,
        fullTsdownConfigPath,
        workspaceRoot,
      );
      if (!matching) {
        infoMigration(
          `Please manually merge ${displayRelative(fullTsdownConfigPath)} into the existing pack config in ${displayRelative(fullViteConfigPath)}. The JSON file was preserved. See ${documentationUrl('/guide/migrate#tsdown')}`,
          report,
        );
        return false;
      }
      if (keepTsdownConfig) {
        return false;
      }
      if (matching.updated) {
        fs.writeFileSync(fullViteConfigPath, matching.content);
      }
    }
    const merged = mergeAndRemoveJsonConfig(
      projectPath,
      viteConfig,
      configs.tsdownConfig,
      'pack',
      silent,
      report,
      { deferRemoval: keepTsdownConfig, workspaceRoot },
    );
    return createdViteConfig || merged;
  }

  const converted = mergeRetainedTsdownConfig(
    fullViteConfigPath,
    fullTsdownConfigPath,
    workspaceRoot,
    keepTsdownConfig,
    silent,
    report,
  );
  if (converted !== undefined) {
    return createdViteConfig || converted;
  }

  // For TS/JS files, import the config file
  const tsdownRelativePath = `./${configs.tsdownConfig}`;
  // Do not prepend a second `pack` key when the config is already wired under
  // a different local import name. If a pack config exists but does not import
  // this tsdown config, leave both files untouched and keep the manual follow-up.
  if (hasConfigKey(fullViteConfigPath, 'pack')) {
    const viteConfigContent = fs.readFileSync(fullViteConfigPath, 'utf8');
    const importsTsdownConfig = tsdownConfigImportPaths(configs.tsdownConfig).some(
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
      prompts.log.success(
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

function mergeImportedTsdownConfigs(
  viteConfigPath: string,
  workspaceRoot: string,
  tsdownPackages: ReadonlySet<string>,
  silent: boolean,
  report: MigrationReport | undefined,
): boolean {
  const directory = path.resolve(path.dirname(viteConfigPath));
  const owners = new Set<string>();
  try {
    if (!hasConfigKey(viteConfigPath, 'pack')) {
      return false;
    }
    const { program } = parseSource(viteConfigPath, fs.readFileSync(viteConfigPath, 'utf8'));
    for (const { value } of configImports(program)) {
      if (!value.startsWith('./') && !value.startsWith('../')) {
        continue;
      }
      const owner = path.dirname(path.resolve(directory, value));
      if (owner !== directory && tsdownPackages.has(owner)) {
        owners.add(owner);
      }
    }
  } catch {
    return false;
  }
  let changed = false;
  for (const owner of owners) {
    const { tsdownConfig } = detectConfigs(owner);
    if (tsdownConfig && !tsdownConfig.endsWith('.json')) {
      changed =
        mergeRetainedTsdownConfig(
          viteConfigPath,
          path.join(owner, tsdownConfig),
          workspaceRoot,
          true,
          silent,
          report,
        ) || changed;
    }
  }
  return changed;
}

function matchingInlinePackConfig(
  viteConfigPath: string,
  tsdownConfigPath: string,
  workspaceRoot: string,
) {
  try {
    const source = fs.readFileSync(viteConfigPath, 'utf8');
    const { existing, expected } = normalizeTsdownJsonPackConfigs(
      source,
      viteConfigPath,
      tsdownConfigPath,
      workspaceRoot,
    );
    const current = configReturnValues(
      defaultConfigValue(parseSource(viteConfigPath, existing).program),
    );
    if (current.length === 0) {
      return undefined;
    }
    const standalone = defaultConfigValue(parseSource(viteConfigPath, expected).program);
    const inlined =
      standalone?.type === 'ObjectExpression' && objectProperty(standalone, 'pack')?.value;
    const matches =
      inlined &&
      current.every((config) => {
        const pack = staticObject(config, true) && objectProperty(config, 'pack')?.value;
        return !!pack && sameStaticValue(pack, inlined);
      });
    return matches ? { content: existing, updated: source !== existing } : undefined;
  } catch {
    // Unknown or invalid config syntax is not enough to discard the source.
    return undefined;
  }
}

function tsdownConfigImportPath(filename: string): string {
  return `./${filename.replace(/\.(m|c)?ts$/, '.$1js')}`;
}

function tsdownConfigImportPaths(filename: string): string[] {
  return [`./${filename}`, `./${path.parse(filename).name}`, tsdownConfigImportPath(filename)];
}

function packConfigPaths(tsdownConfigPath: string) {
  const directory = path.dirname(tsdownConfigPath);
  const sourceIsLink = fs.lstatSync(tsdownConfigPath).isSymbolicLink();
  const sourceTarget = sourceIsLink
    ? path.resolve(directory, fs.readlinkSync(tsdownConfigPath))
    : undefined;
  const realSource = sourceIsLink ? fs.realpathSync(tsdownConfigPath) : undefined;
  const installedPreset =
    sourceTarget &&
    realSource &&
    [sourceTarget, realSource].some((file) => file.split(path.sep).includes('node_modules'))
      ? sourceTarget
      : undefined;
  const extension = path.extname(tsdownConfigPath);
  const originalConvertedName = `vite-pack.config${extension}`;
  let convertedName = originalConvertedName;
  if (extension === '.js' && installedPreset && realSource) {
    // The installed package's module scope does not follow a copied .js file.
    const { program } = parseSource(realSource, fs.readFileSync(realSource, 'utf8'));
    if (program.sourceType === 'module') {
      convertedName = 'vite-pack.config.mjs';
    }
  }
  return {
    convertedName,
    convertedPath: path.join(directory, convertedName),
    previousPath:
      convertedName !== originalConvertedName
        ? path.join(directory, originalConvertedName)
        : undefined,
    realSource,
    installedPreset,
  };
}

function convertedPackLinkTarget(source: string, workspaceRoot: string) {
  const relative = path.relative(fs.realpathSync(workspaceRoot), source);
  if (path.isAbsolute(relative) || relative.split(path.sep)[0] === '..') {
    return undefined;
  }
  const { dir, name, ext } = path.parse(source);
  return path.join(dir, `${name}.vite-pack.config${ext}`);
}

function writePackConfigOutput(filename: string, create: (file: string) => void, replace: boolean) {
  const directory = path.dirname(filename);
  if (!replace) {
    create(filename);
    return;
  }
  // Keep the old generated output if creating its replacement fails.
  const temporary = path.join(directory, `.${path.basename(filename)}.${randomUUID()}`);
  create(temporary);
  try {
    fs.renameSync(temporary, filename);
  } finally {
    fs.rmSync(temporary, { force: true });
  }
}

function applySourceEdits(source: string, edits: (Span & { text: string })[]): string {
  for (const { start, end, text } of edits.toSorted((left, right) => right.start - left.start)) {
    source = source.slice(0, start) + text + source.slice(end);
  }
  return source;
}

function* configNodes(program: Program): IterableIterator<Node> {
  const pending: unknown[] = [...program.body];
  while (pending.length > 0) {
    const item = pending.pop();
    if (Array.isArray(item)) {
      pending.push(...item);
    } else if (typeof item === 'object' && item !== null && 'type' in item) {
      const node = item as Node;
      yield node;
      pending.push(...Object.values(node));
    }
  }
}

/** Preserve static module imports when copying an installed preset into the project. */
function relocateInstalledPackConfig(
  content: string,
  preset: string,
  realPreset: string,
  output: string,
): string | undefined {
  const { program } = parseSource(preset, content);
  for (const node of configNodes(program)) {
    if (
      node.type === 'ImportExpression' ||
      (node.type === 'MetaProperty' && node.meta.name === 'import') ||
      (node.type === 'Identifier' && (node.name === '__dirname' || node.name === '__filename')) ||
      (node.type === 'CallExpression' &&
        node.callee.type === 'Identifier' &&
        node.callee.name === 'require')
    ) {
      return undefined;
    }
  }
  const presetDirectory = path.dirname(preset);
  const installedDirectory = path.dirname(realPreset);
  const outputDirectory = path.dirname(output);
  const edits: (Span & { text: string })[] = [];
  let resolver: ResolverFactory | undefined;
  for (const statement of program.body) {
    if (
      statement.type !== 'ImportDeclaration' &&
      statement.type !== 'ExportNamedDeclaration' &&
      statement.type !== 'ExportAllDeclaration'
    ) {
      continue;
    }
    const source = statement.source;
    if (!source) {
      continue;
    }
    const specifier = source.value;
    let target: string | undefined;
    if (specifier.startsWith('./') || specifier.startsWith('../')) {
      target = path.resolve(presetDirectory, specifier);
    } else if (
      !isBuiltin(specifier) &&
      specifier !== VITE_PLUS_NAME &&
      !specifier.startsWith(`${VITE_PLUS_NAME}/`) &&
      !path.isAbsolute(specifier)
    ) {
      resolver ??= new ResolverFactory({
        conditionNames: ['module-sync', 'node', 'import'],
        symlinks: false,
        nodePath: false,
      });
      const installed = resolver.sync(installedDirectory, specifier).path;
      if (!installed) {
        return undefined;
      }
      if (installed !== resolver.sync(outputDirectory, specifier).path) {
        // Keep the public path to the preset so a reinstall can replace its store entry.
        target = path.resolve(presetDirectory, path.relative(installedDirectory, installed));
        if (!fs.existsSync(target) || fs.realpathSync(target) !== fs.realpathSync(installed)) {
          return undefined;
        }
      }
    }
    if (!target) {
      continue;
    }
    if (target.split(path.sep).includes('.pnpm')) {
      return undefined;
    }
    const relative = path.relative(outputDirectory, target).split(path.sep).join('/');
    const relocated = relative.startsWith('../') ? relative : `./${relative}`;
    edits.push({ start: source.start, end: source.end, text: JSON.stringify(relocated) });
  }
  return applySourceEdits(content, edits);
}

function configImports(program: Program) {
  return program.body.flatMap((statement) =>
    statement.type === 'ImportDeclaration' ? [statement.source] : [],
  );
}

/** Check whether a generated or retained configuration needs Vite+ in focused installs. */
export function configNeedsVitePlus(
  projectPath: string,
  keepTsdownDependency: boolean,
  options?: { wrapPlugins?: boolean },
): boolean {
  const { viteConfig, tsdownConfig } = detectConfigs(projectPath);
  if (!viteConfig || (!tsdownConfig && !keepTsdownDependency)) {
    return false;
  }
  const vitePath = path.join(projectPath, viteConfig);
  try {
    const { program } = parseSource(vitePath, fs.readFileSync(vitePath, 'utf8'));
    const specifiers = configImports(program).map(({ value }) => value);
    for (const node of configNodes(program)) {
      // Import rewriting also handles literal CommonJS require calls.
      if (
        node.type === 'CallExpression' &&
        node.callee.type === 'Identifier' &&
        node.callee.name === 'require' &&
        node.arguments.length === 1 &&
        isString(node.arguments[0])
      ) {
        specifiers.push(node.arguments[0].value);
      }
    }
    const importsVitePlus = specifiers.some(
      (specifier) => specifier === VITE_PLUS_NAME || specifier.startsWith(`${VITE_PLUS_NAME}/`),
    );
    // Workspace migration wraps plugins after rewriting imports and manifests.
    if (
      keepTsdownDependency &&
      (importsVitePlus ||
        specifiers.some((specifier) => specifier === 'vite' || specifier.startsWith('vite/')) ||
        (options?.wrapPlugins && wrapLazyPlugins(vitePath).updated))
    ) {
      return true;
    }
    if (!tsdownConfig || !hasConfigKey(vitePath, 'pack')) {
      return false;
    }
    const { convertedName, previousPath } = packConfigPaths(path.join(projectPath, tsdownConfig));
    const convertedImports = tsdownConfigImportPaths(convertedName);
    if (previousPath) {
      convertedImports.push(...tsdownConfigImportPaths(path.basename(previousPath)));
    }
    return importsVitePlus || specifiers.some((specifier) => convertedImports.includes(specifier));
  } catch {
    return false;
  }
}

function packProperties(program: Program, requireStatic = false): (ObjectProperty | undefined)[] {
  return configReturnValues(defaultConfigValue(program)).map((config) =>
    config?.type === 'ObjectExpression' && (!requireStatic || staticObject(config, true))
      ? objectProperty(config, 'pack')
      : undefined,
  );
}

type PackIdentifier = Extract<Node, { type: 'Identifier' }>;
type PackMapping = 'static' | 'unverified';
type PackReference = {
  value: PackIdentifier;
  mapping?: PackMapping;
};
type PackUse = PackReference & {
  property?: ObjectProperty;
  copy?: { initializer: ObjectExpression; reference: PackIdentifier };
};

function objectAssignTarget(node: Node): Node | undefined {
  if (
    node.type === 'CallExpression' &&
    node.callee.type === 'MemberExpression' &&
    node.callee.object.type === 'Identifier' &&
    node.callee.object.name === 'Object' &&
    memberName(node.callee) === 'assign'
  ) {
    const target = node.arguments[0];
    return target?.type === 'SpreadElement' ? undefined : target;
  }
  return undefined;
}

const staticPackOverrides = new Set(['entry', 'format', 'minify', 'name', 'outDir', 'sourcemap']);

function isStaticPackMapping(callback: Node | undefined): boolean {
  if (
    (callback?.type !== 'ArrowFunctionExpression' && callback?.type !== 'FunctionExpression') ||
    callback.async ||
    callback.generator ||
    callback.params.length !== 1 ||
    callback.params[0].type !== 'Identifier'
  ) {
    return false;
  }
  const parameter = callback.params[0].name;
  let result = callback.body;
  if (result?.type === 'BlockStatement') {
    const [statement] = result.body;
    result =
      result.body.length === 1 && statement.type === 'ReturnStatement' ? statement.argument : null;
  }
  const value = result && unwrapTypeExpression(result);
  if (value?.type === 'Identifier') {
    return value.name === parameter;
  }
  // Read only the complete object and override options that require no migration.
  return (
    value?.type === 'ObjectExpression' &&
    sharedPackSpread(value)?.name === parameter &&
    value.properties.every((member) => {
      if (member.type === 'SpreadElement') {
        return true;
      }
      const name = propertyName(member.key);
      return name === 'dts'
        ? member.value.type === 'Literal' && member.value.value === false
        : name !== undefined && staticPackOverrides.has(name);
    })
  );
}

function packValueReferences(node: Node, mapping?: PackMapping): PackReference[] {
  const value = unwrapTypeExpression(node);
  switch (value.type) {
    case 'Identifier':
      return [{ value, mapping }];
    case 'ConditionalExpression':
      // The test still reads the original values used to choose a config.
      return [
        ...packValueReferences(value.consequent, mapping),
        ...packValueReferences(value.alternate, mapping),
      ];
    case 'ArrayExpression':
      return value.elements.flatMap((entry) => (entry ? packValueReferences(entry, mapping) : []));
    case 'MemberExpression': {
      if (!value.computed) {
        return [];
      }
      // Keep the index expression. For a dynamic index, check the source array
      // before redirecting the import and keep the existing pack toolchain defaults.
      const literalIndex =
        value.property.type === 'Literal' &&
        typeof value.property.value === 'number' &&
        Number.isInteger(value.property.value) &&
        value.property.value >= 0;
      return packValueReferences(value.object, literalIndex ? mapping : (mapping ?? 'static'));
    }
    case 'CallExpression': {
      const target = objectAssignTarget(value);
      if (target) {
        return packValueReferences(target, mapping);
      }
      const { callee } = value;
      if (callee.type !== 'MemberExpression' || memberName(callee) !== 'map') {
        return [];
      }
      const receiver = unwrapTypeExpression(callee.object);
      const staticMapping =
        receiver.type === 'Identifier' &&
        value.arguments.length === 1 &&
        isStaticPackMapping(value.arguments[0]);
      return packValueReferences(receiver, staticMapping ? (mapping ?? 'static') : 'unverified');
    }
    case 'ObjectExpression':
      return value.properties.flatMap((member) =>
        member.type === 'SpreadElement' ? packValueReferences(member.argument, mapping) : [],
      );
    case 'SpreadElement':
      return packValueReferences(value.argument, mapping);
    default:
      return [];
  }
}

function privateConstInitializers(program: Program) {
  const initializers = new Map<number, Node>();
  const pending: unknown[] = [...program.body];
  while (pending.length > 0) {
    const item = pending.pop();
    if (Array.isArray(item)) {
      pending.push(...item);
    } else if (typeof item === 'object' && item !== null && 'type' in item) {
      const node = item as Node;
      // Exported declarations can also be read by other modules.
      if (node.type === 'ExportNamedDeclaration') {
        continue;
      }
      if (node.type === 'VariableDeclaration' && node.kind === 'const') {
        for (const { id, init } of node.declarations) {
          if (id.type === 'Identifier' && init) {
            initializers.set(id.start, init);
          }
        }
      }
      pending.push(...Object.values(node));
    }
  }
  return initializers;
}

function viteConfigMemberReads(program: Program): Set<number> {
  const reads = new Set<number>();
  const objects = configReturnValues(defaultConfigValue(program)).filter(
    (value) => value?.type === 'ObjectExpression',
  );
  for (const object of objects) {
    for (const property of object.properties) {
      if (property.type !== 'Property' || property.computed || property.kind !== 'init') {
        continue;
      }
      const value = unwrapTypeExpression(property.value);
      if (value.type === 'ObjectExpression') {
        objects.push(value);
      } else if (
        value.type === 'MemberExpression' &&
        !value.computed &&
        value.object.type === 'Identifier'
      ) {
        reads.add(value.object.start);
      }
    }
  }
  return reads;
}

/** Copy a literal override without reevaluating the original constant. */
function sharedPackSpread(initializer: ObjectExpression): PackIdentifier | undefined {
  const spreads = initializer.properties.filter((member) => member.type === 'SpreadElement');
  const spread = spreads.length === 1 ? spreads[0].argument : undefined;
  if (
    spread?.type === 'Identifier' &&
    initializer.properties.every(
      (member) =>
        member.type === 'SpreadElement' ||
        (!member.computed &&
          !member.method &&
          member.kind === 'init' &&
          sameStaticValue(member.value, member.value)),
    )
  ) {
    return spread;
  }
  return undefined;
}

function packAssignments(
  { program, bindings }: ReturnType<typeof parseSource>,
  packs: readonly (ObjectProperty | undefined)[],
) {
  const uses: PackUse[] = packs.flatMap((property) =>
    property
      ? packValueReferences(property.value).map(({ value, mapping }) => ({
          property,
          value,
          mapping,
        }))
      : [],
  );
  if (uses.length === 0) {
    return uses;
  }
  const initializers = privateConstInitializers(program);
  const aliasesByReference = new Map<
    number,
    { initializer: Node; references: number[]; remaining: number; mapping?: PackMapping }
  >();
  for (const { start, constant, references } of bindings) {
    const initializer = initializers.get(start);
    if (!initializer || !constant) {
      continue;
    }
    const alias = { initializer, references, remaining: references.length };
    for (const reference of references) {
      aliasesByReference.set(reference, alias);
    }
  }

  // Follow a constant only when every reference feeds pack. Add its initializer
  // uses to the same work list so chains of private aliases are handled too.
  for (const { value, mapping } of uses) {
    const alias = aliasesByReference.get(value.start);
    if (!alias) {
      continue;
    }
    if (mapping === 'unverified' || alias.mapping === undefined) {
      alias.mapping = mapping;
    }
    if (--alias.remaining === 0) {
      uses.push(...packValueReferences(alias.initializer, alias.mapping));
    }
  }
  const packReferences = new Set(uses.map(({ value }) => value.start));
  const propertyReads = viteConfigMemberReads(program);
  const copies = uses.flatMap(({ property, value: reference, mapping }) => {
    const alias = aliasesByReference.get(reference.start);
    if (
      !alias?.remaining ||
      alias.initializer.type !== 'ObjectExpression' ||
      !alias.references.every((start) => packReferences.has(start) || propertyReads.has(start))
    ) {
      return [];
    }
    const value = sharedPackSpread(alias.initializer);
    return value
      ? [{ property, value, mapping, copy: { initializer: alias.initializer, reference } }]
      : [];
  });
  return [...uses, ...copies];
}

const arrayMutators = new Set([
  'copyWithin',
  'fill',
  'pop',
  'push',
  'reverse',
  'shift',
  'sort',
  'splice',
  'unshift',
]);

function packRootIdentifier(node: Node): PackIdentifier | undefined {
  let root = unwrapTypeExpression(node);
  while (root.type === 'MemberExpression') {
    root = unwrapTypeExpression(root.object);
  }
  return root.type === 'Identifier' ? root : undefined;
}

function aliasBindingStarts(binding: Node): number[] {
  switch (binding.type) {
    case 'Identifier':
      return [binding.start];
    case 'ObjectPattern':
      return binding.properties.flatMap((property) =>
        property.type === 'Property' ? aliasBindingStarts(property.value) : [],
      );
    case 'ArrayPattern':
      return binding.elements.flatMap((element) => (element ? aliasBindingStarts(element) : []));
    case 'AssignmentPattern':
      return aliasBindingStarts(binding.left);
    default:
      return [];
  }
}

function callbackExpressions(node: FunctionNode | ArrowFunctionExpression) {
  const body = node.body;
  return body?.type === 'BlockStatement'
    ? body.body.map((statement) =>
        statement.type === 'ExpressionStatement' ? statement.expression : undefined,
      )
    : [body];
}

function entryMutationHelper(node: FunctionNode | ArrowFunctionExpression) {
  const [parameter] = node.params;
  if (node.async || node.generator || node.params.length !== 1 || parameter.type !== 'Identifier') {
    return undefined;
  }
  const type = parameter.typeAnnotation?.typeAnnotation;
  const typeReference =
    type?.type === 'TSTypeQuery' && type.exprName.type === 'Identifier'
      ? type.exprName.start
      : undefined;
  if (type && typeReference === undefined) {
    return undefined;
  }
  const expressions = callbackExpressions(node);
  if (
    expressions.length > 0 &&
    expressions.every((expression) => {
      if (expression?.type !== 'CallExpression' || expression.callee.type !== 'MemberExpression') {
        return false;
      }
      const method = memberName(expression.callee);
      const receiver = expression.callee.object;
      return (
        !!method &&
        arrayMutators.has(method) &&
        receiver.type === 'MemberExpression' &&
        memberName(receiver) === 'entry' &&
        receiver.object.type === 'Identifier' &&
        receiver.object.name === parameter.name &&
        expression.arguments.every((argument) => sameStaticValue(argument, argument))
      );
    })
  ) {
    return { typeReference };
  }
  return undefined;
}

function setsPackArrayOutput(
  callback: FunctionNode | ArrowFunctionExpression,
  bindings: ReturnType<typeof parseSource>['bindings'],
): boolean {
  if (callback.async || callback.generator) {
    return false;
  }
  const [options] = callback.params;
  if (options?.type !== 'Identifier' || options.typeAnnotation || callback.params.length > 2) {
    return false;
  }
  const binding = bindings.find(({ start }) => start === options.start);
  const expressions = callbackExpressions(callback);
  // The callback can compute the directory normally. Only the assignment may
  // use the options object, so it cannot read a renamed option in the copy.
  const references = new Set(binding?.references);
  return (
    expressions.length > 0 &&
    expressions.every(
      (expression) =>
        expression?.type === 'AssignmentExpression' &&
        expression.operator === '=' &&
        expression.left.type === 'MemberExpression' &&
        memberName(expression.left) === 'outDir' &&
        expression.left.object.type === 'Identifier' &&
        references.delete(expression.left.object.start),
    ) &&
    references.size === 0
  );
}

function packMutationReferences({ program, bindings }: ReturnType<typeof parseSource>) {
  const writes = new Set<number>();
  const aliasInitializers = new Map<number, number>();
  const callbackOrigins = new Map<number, number>();
  const helpers = new Map<number, NonNullable<ReturnType<typeof entryMutationHelper>>>();
  const calls = new Map<number, number>();
  const rewritable = new Map<number, { typeReference?: number; array?: boolean }>();
  const recordHelper = (start: number, node: FunctionNode | ArrowFunctionExpression) => {
    const helper = entryMutationHelper(node);
    if (helper) {
      helpers.set(start, helper);
    }
  };
  for (const node of configNodes(program)) {
    let target: Node;
    switch (node.type) {
      case 'FunctionDeclaration':
        if (node.id) {
          recordHelper(node.id.start, node);
        }
        continue;
      case 'VariableDeclaration':
        if (node.kind === 'const') {
          for (const { id, init } of node.declarations) {
            const origin = init && packRootIdentifier(init);
            if (origin) {
              for (const start of aliasBindingStarts(id)) {
                aliasInitializers.set(start, origin.start);
              }
            }
            if (
              id.type === 'Identifier' &&
              (init?.type === 'FunctionExpression' || init?.type === 'ArrowFunctionExpression')
            ) {
              recordHelper(id.start, init);
            }
          }
        }
        continue;
      case 'AssignmentExpression':
        target = node.left;
        break;
      case 'UpdateExpression':
        target = node.argument;
        break;
      case 'UnaryExpression':
        if (node.operator !== 'delete') {
          continue;
        }
        target = node.argument;
        break;
      case 'CallExpression': {
        const { callee } = node;
        const method = memberName(callee);
        if (callee.type === 'MemberExpression' && method === 'forEach') {
          const object = unwrapTypeExpression(callee.object);
          if (object.type === 'Identifier') {
            const [callback] = node.arguments;
            if (
              node.arguments.length === 1 &&
              (callback.type === 'FunctionExpression' ||
                callback.type === 'ArrowFunctionExpression') &&
              !callback.params.some((parameter) => parameter.type === 'RestElement')
            ) {
              const [element, , array] = callback.params;
              // Writes to the element or the third callback argument affect
              // the original array. A callback that only reads can stay split.
              for (const parameter of [element, array]) {
                if (parameter) {
                  for (const start of aliasBindingStarts(parameter)) {
                    callbackOrigins.set(start, object.start);
                  }
                }
              }
              if (setsPackArrayOutput(callback, bindings)) {
                rewritable.set(object.start, { array: true });
              }
            } else {
              writes.add(object.start);
            }
          }
        }
        const receiver =
          callee.type === 'MemberExpression' && method && arrayMutators.has(method)
            ? callee.object
            : undefined;
        const assigned = objectAssignTarget(node) ?? receiver;
        if (!assigned) {
          const [argument] = node.arguments;
          if (
            callee.type === 'Identifier' &&
            node.arguments.length === 1 &&
            argument.type === 'Identifier'
          ) {
            calls.set(callee.start, argument.start);
          }
          // A function can change options passed by reference. Keep those
          // references together unless the local helper can be verified.
          for (const argument of node.arguments) {
            const root = packRootIdentifier(
              argument.type === 'SpreadElement' ? argument.argument : argument,
            );
            if (root) {
              writes.add(root.start);
            }
          }
          continue;
        }
        target = assigned;
        break;
      }
      default:
        continue;
    }
    const root = packRootIdentifier(target);
    if (root) {
      writes.add(root.start);
    }
  }
  const origins = new Map<number, number>();
  for (const { start, references, constant } of bindings) {
    const helper = helpers.get(start);
    const argument = references.length === 1 ? calls.get(references[0]) : undefined;
    if (constant && helper && argument !== undefined) {
      rewritable.set(argument, helper);
    }
    const origin =
      (constant ? aliasInitializers.get(start) : undefined) ?? callbackOrigins.get(start);
    if (origin !== undefined) {
      for (const reference of references) {
        origins.set(reference, origin);
      }
    }
  }
  // The initializer can itself reference an alias. Follow those links until
  // the write set contains every original binding reached by a mutation.
  for (const write of writes) {
    const origin = origins.get(write);
    if (origin !== undefined) {
      writes.add(origin);
    }
  }
  return { writes, rewritable };
}

function importedPackBindings(
  parsed: ReturnType<typeof parseSource>,
  assignments: ReturnType<typeof packAssignments>,
  vitePath: string,
  packPath: string,
) {
  if (assignments.length === 0) {
    return [];
  }
  const { program, bindings } = parsed;
  const directory = path.dirname(vitePath);
  const withoutExtension = path.join(path.dirname(packPath), path.parse(packPath).name);
  const jsPath = packPath.replace(/\.(m|c)?ts$/, '.$1js');
  const { writes, rewritable } = packMutationReferences(parsed);
  return program.body.flatMap((statement) => {
    if (statement.type !== 'ImportDeclaration' || statement.importKind === 'type') {
      return [];
    }
    const specifier = statement.source.value;
    if (!specifier.startsWith('./') && !specifier.startsWith('../')) {
      return [];
    }
    const imported = path.resolve(directory, specifier);
    // Treat another file at the written path as an explicit import of that file.
    if (
      imported !== packPath &&
      ((imported !== jsPath && imported !== withoutExtension) || fs.existsSync(imported))
    ) {
      return [];
    }
    const binding = statement.specifiers.find(
      (spec) =>
        spec.type === 'ImportDefaultSpecifier' ||
        (spec.type === 'ImportSpecifier' &&
          spec.importKind !== 'type' &&
          propertyName(spec.imported) === 'default'),
    );
    const references = bindings.find(({ start }) => start === binding?.local.start)?.references;
    if (!binding || !references) {
      return [];
    }
    const uses = assignments.filter(({ value }) => references.includes(value.start));
    if (uses.length === 0) {
      return [];
    }
    const written = references.filter((reference) => writes.has(reference));
    const onlyPackUses = references.length === uses.length && uses.every(({ copy }) => !copy);
    const helperReferences = new Set(uses.map(({ value }) => value.start));
    const canRedirectMutations =
      written.length > 0 &&
      uses.every(({ copy }) => !copy) &&
      written.every((reference) => {
        const mutation = rewritable.get(reference);
        if (!mutation) {
          return false;
        }
        helperReferences.add(reference);
        const type = mutation.typeReference;
        if (type !== undefined) {
          helperReferences.add(type);
        }
        return type === undefined || references.includes(type);
      }) &&
      references.every((reference) => helperReferences.has(reference));
    return [
      {
        statement,
        binding,
        uses,
        mutates: written.length > 0,
        canRedirectMutations,
        mutatesArray: written.some((reference) => rewritable.get(reference)?.array),
        onlyPack: onlyPackUses || canRedirectMutations,
      },
    ];
  });
}

function importSeparator(source: string, comments: readonly Span[], left: Span, right: Span) {
  for (
    let index = source.indexOf(',', left.end);
    index !== -1 && index < right.start;
    index = source.indexOf(',', index + 1)
  ) {
    if (!comments.some(({ start, end }) => start <= index && index < end)) {
      return index;
    }
  }
  return undefined;
}

function rewritePackConfigImports(
  source: string,
  comments: readonly Span[],
  imports: ReturnType<typeof importedPackBindings>,
  originalName: string,
  convertedName: string,
  previousImports: ReturnType<typeof importedPackBindings>,
): string {
  const edits: Array<Span & { text: string }> = previousImports.map(({ statement }) => {
    const { start, end, value } = statement.source;
    return {
      start,
      end,
      text: JSON.stringify(value.slice(0, -path.posix.basename(value).length) + convertedName),
    };
  });
  const originalStem = path.parse(originalName).name;
  const convertedStem = path.parse(convertedName).name;
  const newline = source.includes('\r\n') ? '\r\n' : '\n';
  let nextName = 0;
  for (const { statement, binding, uses, onlyPack } of imports) {
    const original = statement.source.value;
    const filename = path.posix.basename(original);
    const replacement =
      path.extname(originalName) === path.extname(convertedName)
        ? filename.replace(originalStem, convertedStem)
        : convertedName;
    const converted = original.slice(0, -filename.length) + replacement;
    const literal = JSON.stringify(converted);
    if (onlyPack && statement.specifiers.length === 1) {
      edits.push({ start: statement.source.start, end: statement.source.end, text: literal });
      continue;
    }

    let name = binding.local.name;
    const index = statement.specifiers.indexOf(binding);
    const next = statement.specifiers[index + 1];
    const previous = statement.specifiers[index - 1];
    // A named default specifier can share the braces with other named imports.
    // Keep punctuation and comments outside the removed binding.
    const separator = next
      ? importSeparator(source, comments, binding, next)
      : previous?.type === 'ImportSpecifier'
        ? importSeparator(source, comments, previous, binding)
        : undefined;
    if (onlyPack && separator !== undefined) {
      edits.push({ start: binding.start, end: binding.end, text: '' });
      edits.push({ start: separator, end: separator + 1, text: '' });
    } else {
      // Other consumers still need the original module and binding.
      do {
        name = nextName++ === 0 ? 'vitePackConfig' : `vitePackConfig${nextName}`;
      } while (source.includes(name));
      for (const { property, value, copy } of uses) {
        const reference = copy?.reference ?? value;
        const replacement = copy
          ? source.slice(copy.initializer.start, value.start) +
            name +
            source.slice(value.end, copy.initializer.end)
          : name;
        edits.push({
          start: reference.start,
          end: reference.end,
          text: property?.shorthand ? `pack: ${replacement}` : replacement,
        });
      }
    }
    const lineEnd = source.indexOf(newline, statement.end);
    const insertion =
      lineEnd === -1
        ? source.length
        : (comments.find(({ start, end }) => start <= lineEnd && lineEnd < end)?.end ?? lineEnd);
    edits.push({
      start: insertion,
      end: insertion,
      text: `${newline}import ${name} from ${literal};`,
    });
  }
  return applySourceEdits(source, edits);
}

/** Return the active pack configuration when equivalence cannot be established. */
export function findDivergedTsdownPackConfig(
  tsdownConfigPath: string,
  workspaceRoot: string,
  commandDirectory: string,
): string | undefined {
  const configDirectory = path.dirname(tsdownConfigPath);
  // A package-local tsdown config will get a local Vite config during migration.
  const vitePath =
    commandDirectory === configDirectory
      ? findViteConfig(commandDirectory)
      : findViteConfigUp(commandDirectory, workspaceRoot);
  if (!vitePath) {
    return undefined;
  }
  try {
    const hasPack = hasConfigKey(vitePath, 'pack');
    if (tsdownConfigPath.endsWith('.json')) {
      return hasPack && !matchingInlinePackConfig(vitePath, tsdownConfigPath, workspaceRoot)
        ? vitePath
        : undefined;
    }
    const { convertedPath, previousPath } = packConfigPaths(tsdownConfigPath);
    if (!hasPack && !fs.existsSync(convertedPath)) {
      // Use the same merge as the later config edit before replacing the build command.
      const merged = mergeTsdownConfig(vitePath, `./${path.basename(tsdownConfigPath)}`);
      return merged.updated ? undefined : vitePath;
    }
    const source = parseSource(vitePath, fs.readFileSync(vitePath, 'utf8'));
    const packs = packProperties(source.program, true);
    if (packs.length === 0) {
      return vitePath;
    }
    const assignments = packAssignments(source, packs);
    const usesConfig = (file: string) =>
      importedPackBindings(source, assignments, vitePath, file).some(
        ({ uses, mutates }) =>
          !mutates &&
          uses.length === packs.length &&
          uses.every(
            ({ property, value }) => property && unwrapTypeExpression(property.value) === value,
          ),
      );
    if (usesConfig(tsdownConfigPath)) {
      return undefined;
    }
    const { content } = rewriteTsdownConfigForPack(tsdownConfigPath, workspaceRoot);
    const expected = content ?? fs.readFileSync(tsdownConfigPath, 'utf8');
    const activePath = [convertedPath, previousPath].find(
      (file) => file && fs.existsSync(file) && usesConfig(file),
    );
    if (activePath) {
      return sameConfigModule(activePath, expected, fs.readFileSync(activePath, 'utf8'))
        ? undefined
        : activePath;
    }
    const expectedValue = defaultConfigValue(parseSource(tsdownConfigPath, expected).program);
    return expectedValue &&
      packs.every(
        (pack) => pack && sameStaticValue(unwrapTypeExpression(pack.value), expectedValue),
      )
      ? undefined
      : vitePath;
  } catch {
    // The original command is the only verified build when comparison fails.
    return vitePath;
  }
}

/** Write a separate converted config only when pack needs compatibility changes. */
function mergeRetainedTsdownConfig(
  viteConfigPath: string,
  tsdownConfigPath: string,
  workspaceRoot: string,
  keepTsdownConfig: boolean,
  silent: boolean,
  report: MigrationReport | undefined,
): boolean | undefined {
  const originalName = path.basename(tsdownConfigPath);
  const { convertedName, convertedPath, previousPath, realSource, installedPreset } =
    packConfigPaths(tsdownConfigPath);
  const source = fs.readFileSync(viteConfigPath, 'utf8');
  const hasPack = hasConfigKey(viteConfigPath, 'pack');
  const parsed = hasPack ? parseSource(viteConfigPath, source) : undefined;
  const assignments = parsed ? packAssignments(parsed, packProperties(parsed.program)) : [];
  const originalImports = parsed
    ? importedPackBindings(parsed, assignments, viteConfigPath, tsdownConfigPath)
    : [];
  const convertedImports = parsed
    ? importedPackBindings(parsed, assignments, viteConfigPath, convertedPath)
    : [];
  const previousImports =
    parsed && previousPath
      ? importedPackBindings(parsed, assignments, viteConfigPath, previousPath)
      : [];
  const upgradesModule = previousImports.length > 0;
  const importsConverted = convertedImports.length > 0 || upgradesModule;
  const convertedEntry = fs.lstatSync(convertedPath, { throwIfNoEntry: false });
  const convertedExists = fs.statSync(convertedPath, { throwIfNoEntry: false })?.isFile();
  if (
    importsConverted &&
    originalImports.length === 0 &&
    !upgradesModule &&
    convertedExists &&
    (installedPreset ? convertedEntry?.isFile() : !realSource || convertedEntry?.isSymbolicLink())
  ) {
    return preparePackConfigTsconfigEdits(
      viteConfigPath,
      convertedPath,
      workspaceRoot,
      tsdownConfigPath,
    ).apply();
  }
  if (!keepTsdownConfig && !importsConverted) {
    return undefined;
  }
  if (hasPack && !importsConverted && originalImports.length === 0) {
    return undefined;
  }

  const packImports = [...originalImports, ...convertedImports, ...previousImports];
  const mappings = packImports.flatMap(({ uses }) =>
    uses.flatMap(({ mapping }) => (mapping ? [mapping] : [])),
  );
  const usesArray = mappings.length > 0 || packImports.some(({ mutatesArray }) => mutatesArray);
  const usesBundledDefaults =
    usesArray || packImports.some(({ canRedirectMutations }) => canRedirectMutations);
  const { content: converted, warnings } = rewriteTsdownConfigForPack(
    tsdownConfigPath,
    workspaceRoot,
    // Derived configs already ran under vp pack. Keep their bundled defaults.
    usesBundledDefaults ? viteConfigPath : undefined,
  );
  for (const warning of warnings) {
    warnMigration(`${displayRelative(tsdownConfigPath)}: ${warning}`, report);
  }
  if (converted === undefined && !importsConverted) {
    return undefined;
  }
  if (
    originalImports.some(({ mutates, canRedirectMutations }) => mutates && !canRedirectMutations)
  ) {
    warnMigration(
      `${displayRelative(viteConfigPath)} changes the imported options from ${displayRelative(tsdownConfigPath)} or passes them to code that can change them. Keep those changes when you migrate the pack options manually. See ${documentationUrl('/guide/migrate#tsdown')}`,
      report,
    );
    return false;
  }
  if (usesArray) {
    const array =
      !mappings.includes('unverified') &&
      defaultConfigValue(
        parseSource(tsdownConfigPath, converted ?? fs.readFileSync(tsdownConfigPath, 'utf8'))
          .program,
      );
    if (
      !array ||
      array.type !== 'ArrayExpression' ||
      !array.elements.every((entry) => staticObject(entry))
    ) {
      warnMigration(
        `Cannot verify how ${displayRelative(viteConfigPath)} selects options from ${displayRelative(tsdownConfigPath)}. Migrate the pack options manually. See ${documentationUrl('/guide/migrate#tsdown')}`,
        report,
      );
      return false;
    }
  }

  // Keep relative imports beside a workspace-owned source. Installed presets
  // get a project-owned file because a reinstall can replace their directory.
  const siblingTarget = realSource ? convertedPackLinkTarget(realSource, workspaceRoot) : undefined;
  const linkedTarget = installedPreset ? undefined : siblingTarget;
  if (realSource && !installedPreset && !linkedTarget) {
    warnMigration(
      `The shared config at ${displayRelative(tsdownConfigPath)} points outside the workspace. Migrate its pack options manually. See ${documentationUrl('/guide/migrate#tsdown')}`,
      report,
    );
    return undefined;
  }

  let config = source;
  if (parsed && (originalImports.length > 0 || upgradesModule)) {
    config = rewritePackConfigImports(
      source,
      parsed.comments,
      originalImports,
      originalName,
      convertedName,
      previousImports,
    );
  } else if (!hasPack) {
    const result = mergeTsdownConfig(viteConfigPath, `./${convertedName}`);
    if (!result.updated) {
      return undefined;
    }
    config = result.content;
  }

  const existingLink = convertedEntry?.isSymbolicLink()
    ? path.resolve(fs.realpathSync(path.dirname(convertedPath)), fs.readlinkSync(convertedPath))
    : undefined;
  const reuseLink = linkedTarget !== undefined && existingLink === linkedTarget;
  const replaceInstalledLink =
    !!installedPreset && siblingTarget !== undefined && existingLink === siblingTarget;
  let content = converted ?? fs.readFileSync(tsdownConfigPath, 'utf8');
  if (installedPreset && realSource) {
    if (replaceInstalledLink && convertedExists) {
      content = fs.readFileSync(convertedPath, 'utf8');
    }
    const relocated = relocateInstalledPackConfig(
      content,
      installedPreset,
      realSource,
      convertedPath,
    );
    if (relocated === undefined) {
      warnMigration(
        `Cannot copy ${displayRelative(tsdownConfigPath)} without changing how the installed preset resolves files. Migrate its pack options manually. See ${documentationUrl('/guide/migrate#tsdown')}`,
        report,
      );
      return hasPack ? false : undefined;
    }
    content = relocated;
  }
  if (upgradesModule && previousPath && fs.existsSync(previousPath)) {
    let matches = false;
    try {
      matches = sameConfigModule(previousPath, content, fs.readFileSync(previousPath, 'utf8'));
    } catch {
      // Keep a previous copy whose content cannot be verified.
    }
    if (!matches) {
      warnMigration(
        `${displayRelative(previousPath)} differs from the generated pack options. Migrate this config to ${convertedName} manually. See ${documentationUrl('/guide/migrate#tsdown')}`,
        report,
      );
      return false;
    }
  }
  const writePath = linkedTarget ?? convertedPath;
  const existingFile = reuseLink || replaceInstalledLink ? undefined : convertedEntry;
  const targetEntry = linkedTarget
    ? fs.lstatSync(linkedTarget, { throwIfNoEntry: false })
    : existingFile;
  const outputs = [{ file: convertedPath, entry: existingFile }];
  if (linkedTarget) {
    outputs.push({ file: linkedTarget, entry: targetEntry });
  }
  const conflict = outputs.find(
    ({ file, entry }) => entry && (!entry.isFile() || fs.readFileSync(file, 'utf8') !== content),
  );
  if (conflict) {
    warnMigration(
      `Cannot write the converted pack config because ${displayRelative(conflict.file)} already exists. Merge ${displayRelative(tsdownConfigPath)} into pack manually.`,
      report,
    );
    return false;
  }
  const tsconfigEdits = preparePackConfigTsconfigEdits(
    viteConfigPath,
    convertedPath,
    workspaceRoot,
    tsdownConfigPath,
  );
  if (tsconfigEdits.missingPreset) {
    const { filename, specifier } = tsconfigEdits.missingPreset;
    warnMigration(
      `Resolve the TypeScript preset \`${specifier}\` used by ${displayRelative(filename)} and rerun \`vp migrate\` to add the converted pack config.`,
      report,
    );
    return false;
  }
  if (tsconfigEdits.excludedConfig) {
    warnMigration(
      `Allow \`${convertedName}\` in ${displayRelative(tsconfigEdits.excludedConfig)} and rerun \`vp migrate\` to add the converted pack config.`,
      report,
    );
    return false;
  }
  if (!targetEntry) {
    writePackConfigOutput(
      writePath,
      (file) => fs.writeFileSync(file, content, { flag: 'wx' }),
      replaceInstalledLink,
    );
  }
  if (linkedTarget && !reuseLink) {
    const relativeTarget = path.relative(
      fs.realpathSync(path.dirname(convertedPath)),
      linkedTarget,
    );
    writePackConfigOutput(
      convertedPath,
      (file) => fs.symlinkSync(relativeTarget, file, 'file'),
      convertedEntry !== undefined,
    );
  }
  const included = tsconfigEdits.apply();
  try {
    if (config !== source) {
      fs.writeFileSync(viteConfigPath, config);
    }
  } catch (error) {
    if (!convertedEntry && !included) {
      fs.unlinkSync(convertedPath);
    }
    throw error;
  }
  if (report) {
    report.tsdownImportCount++;
  }
  if (!silent) {
    prompts.log.success(`✔ Added ${displayRelative(convertedPath)} for vp pack`);
  }
  return true;
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
  packages?: WorkspacePackage[],
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
  packages?: WorkspacePackage[],
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
    delete normalizedOxlintConfig.rules![PREFER_VITE_PLUS_IMPORTS_RULE];
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
      { modulePlan: plan, deferRemoval: preparedOxlintConfigs !== undefined },
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
  const { ast, comments } = JSON.parse(analyzeMigrationSource(filename, source)) as {
    ast: { node: Program };
    comments: Array<{ start: number; end: number }>;
  };
  let offset = ast.node.hashbang?.end ?? 0;
  for (const statement of ast.node.body) {
    if (statement.type !== 'ExpressionStatement' || typeof statement.directive !== 'string') {
      break;
    }
    offset = statement.end;
  }
  const firstStatement = ast.node.body[0]?.start ?? source.length;
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
  if ((node as { type?: unknown }).type === 'ObjectExpression') {
    const { properties } = node as ObjectExpression;
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
  let marked: string[];
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
  {
    modulePlan,
    deferRemoval = false,
    workspaceRoot = projectPath,
  }: { modulePlan?: OxlintModulePlan; deferRemoval?: boolean; workspaceRoot?: string } = {},
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
      prompts.log.info(
        `${configKey} config already present in ${displayRelative(fullViteConfigPath)} — ${deferRemoval ? 'deferred removal of' : 'removed redundant'} ${displayRelative(fullJsonConfigPath)}`,
      );
    }
    return true;
  }
  // Only delete the original JSON after a successful merge. Resolved lint
  // configs contain object extends, which JSON Oxlint configs do not accept.
  const mergeConfig = replaceDefaults ? upsertJsonConfig : mergeJsonConfig;
  let result;
  if (configKey === 'pack') {
    result = mergeTsdownJsonConfig(fullJsonConfigPath, workspaceRoot, fullViteConfigPath);
  } else if (!modulePlan) {
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
  const writtenModules: string[] = [];
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
    prompts.log.success(
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
    prompts.log.success(
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
  manualWarnings?: string[],
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
    prompts.log.success(
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
  if (extracted != null) {
    lint = JSON.parse(extracted);
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
    oxlintOwnerDirs?: string[];
    importRuleConfig?: string;
    preserveTsdownDirs?: string[];
  } = {},
): boolean {
  const result = rewriteImportsInDirectory(
    projectPath,
    options.preserveNuxtVitestImports ?? true,
    options.oxlintOwnerDirs ?? [],
    options.importRuleConfig,
    options.preserveTsdownDirs,
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

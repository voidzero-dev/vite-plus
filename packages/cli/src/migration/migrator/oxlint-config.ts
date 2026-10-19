import fs from 'node:fs';
import path from 'node:path';

import type { Program } from '@oxc-project/types';
import { parse as parseJsonc, type ParseError } from 'jsonc-parser';
import type { OxlintConfig } from 'oxlint';

import { analyzeMigrationSource, ResolverFactory } from '../../../binding/index.js';
import { detectConfigs } from '../detector.ts';
import { ensureSvelteRuneGlobals } from '../migrator.ts';
import {
  OXLINT_OWNER_CONFIG_KEYS,
  type OxlintConfigFile,
  type OxlintConfigMigration,
  type OxlintModulePlan,
} from './shared.ts';

/** Read the inheritance graph before migration removes any workspace configs. */
export function resolveOxlintConfigExtends(
  configPath: string,
  cache = new Map<string, OxlintConfigFile>(),
  stack: string[] = [],
): OxlintConfigFile {
  const filename = fs.realpathSync(configPath);
  if (stack.includes(filename)) {
    throw new Error(`Circular Oxlint extends: ${[...stack, filename].join(' -> ')}`);
  }
  const cached = cache.get(filename);
  if (cached) {
    return cached;
  }
  const errors: ParseError[] = [];
  const config = parseJsonc(fs.readFileSync(filename, 'utf8'), errors, {
    allowTrailingComma: true,
  }) as Record<string, unknown>;
  if (errors.length > 0 || !config || typeof config !== 'object' || Array.isArray(config)) {
    throw new Error(`Invalid Oxlint JSON config: ${filename}`);
  }
  delete config.$schema;
  const inherited = config.extends;
  if (
    inherited !== undefined &&
    (!Array.isArray(inherited) || inherited.some((entry) => typeof entry !== 'string'))
  ) {
    throw new Error(`Expected an array of file paths in ${filename}: extends`);
  }
  const extendsPaths = ((inherited ?? []) as string[]).map((entry) =>
    path.resolve(path.dirname(filename), entry),
  );
  const children = extendsPaths.map((childPath) => {
    const child = resolveOxlintConfigExtends(childPath, cache, [...stack, filename]);
    child.shared = true;
    return child;
  });
  if (inherited !== undefined) {
    config.extends = children.map((child) => child.config);
  }
  const file: OxlintConfigFile = {
    filename,
    config,
    extends: children,
    extendsPaths,
  };
  cache.set(filename, file);
  return file;
}

export function prepareOxlintConfigs(
  projectPaths: string[],
): Map<string, OxlintConfigMigration | Error> {
  const configs = new Map<string, OxlintConfigMigration | Error>();
  const cache = new Map<string, OxlintConfigFile>();
  for (const projectPath of projectPaths) {
    const filename = detectConfigs(projectPath).oxlintConfig;
    if (!filename) {
      continue;
    }
    const configPath = path.join(projectPath, filename);
    try {
      configs.set(configPath, { file: resolveOxlintConfigExtends(configPath, cache) });
    } catch (error) {
      configs.set(configPath, error instanceof Error ? error : new Error(String(error)));
    }
  }
  return configs;
}

function collectConfigFiles(file: OxlintConfigFile, target: Set<OxlintConfigFile>): void {
  if (target.has(file)) {
    return;
  }
  target.add(file);
  for (const child of file.extends) {
    collectConfigFiles(child, target);
  }
}

function configPathInRealDirectory(filename: string): string {
  return path.join(fs.realpathSync(path.dirname(filename)), path.basename(filename));
}

/** Retain each link in a chain, without conflating independent aliases of its target. */
function retainConfigPath(filename: string, retained: Set<string>): void {
  filename = configPathInRealDirectory(filename);
  while (!retained.has(filename)) {
    retained.add(filename);
    if (!fs.lstatSync(filename).isSymbolicLink()) {
      return;
    }
    filename = configPathInRealDirectory(
      path.resolve(path.dirname(filename), fs.readlinkSync(filename)),
    );
  }
}

/** Retained JSON consumers still need their full inheritance graph on disk. */
export function removeMigratedOxlintConfigs(
  configs: Map<string, OxlintConfigMigration | Error>,
): void {
  const files = new Set<OxlintConfigFile>();
  const retained = new Set<OxlintConfigFile>();
  const retainedPaths = new Set<string>();
  const removableSources = new Set<OxlintConfigFile>();
  const removalCandidates = new Set<string>();
  for (const [filename, migration] of configs) {
    // An unreadable graph can contain references we could not discover.
    // Preserve all candidates rather than break that consumer's fallback.
    if (migration instanceof Error) {
      return;
    }
    const { file, migrated } = migration;
    collectConfigFiles(file, files);
    if (!migrated) {
      retainConfigPath(filename, retainedPaths);
      collectConfigFiles(file, retained);
      continue;
    }
    removalCandidates.add(filename);
    // Unlinking a workspace symlink leaves its shared JSON target on disk.
    if (!fs.lstatSync(filename).isSymbolicLink()) {
      removableSources.add(file);
    }
  }
  for (const file of files) {
    if (!removableSources.has(file)) {
      collectConfigFiles(file, retained);
    }
  }
  for (const file of retained) {
    retainConfigPath(file.filename, retainedPaths);
    for (const filename of file.extendsPaths) {
      retainConfigPath(filename, retainedPaths);
    }
  }
  for (const filename of removalCandidates) {
    if (!retainedPaths.has(configPathInRealDirectory(filename))) {
      fs.unlinkSync(filename);
    }
  }
}

function relativeImport(from: string, to: string): string {
  const relative = path.relative(path.dirname(from), to).split(path.sep).join('/');
  if (path.isAbsolute(relative)) {
    throw new Error(`Cannot import an Oxlint config across drives: ${to}`);
  }
  // Vite resolves literal filesystem paths, but treats `?` as a query delimiter.
  // Percent-encoding also breaks resolution of otherwise valid paths such as `#`.
  if (relative.includes('?')) {
    throw new Error(`Cannot import a shared Oxlint config whose relative path contains "?": ${to}`);
  }
  // Vite and TypeScript resolve .js imports to .ts source files.
  // This also works with NodeNext without changing allowImportingTsExtensions.
  const specifier = /\.m?ts$/.test(from) ? relative.replace(/\.ts$/, '.js') : relative;
  return specifier.startsWith('../') ? specifier : `./${specifier}`;
}

function printConfig(
  config: OxlintConfig,
  inheritedNames: string[],
  {
    pluginExpression,
    ownerConfigName,
  }: {
    pluginExpression?: (specifier: string) => string;
    ownerConfigName?: string;
  } = {},
): string {
  function printPlugin(entry: NonNullable<OxlintConfig['jsPlugins']>[number]): string {
    const specifier = typeof entry === 'string' ? entry : entry.specifier;
    const expression = pluginExpression?.(specifier) ?? JSON.stringify(specifier);
    return typeof entry === 'string'
      ? expression
      : `{ "name": ${JSON.stringify(entry.name)}, "specifier": ${expression} }`;
  }
  const properties = Object.entries(config).map(([key, value]) => {
    let code: string;
    if (key === 'extends') {
      code = `[${inheritedNames.join(', ')}]`;
    } else if (ownerConfigName && OXLINT_OWNER_CONFIG_KEYS.has(key)) {
      code = `${ownerConfigName}.${key}`;
    } else if (key === 'jsPlugins' && Array.isArray(config.jsPlugins)) {
      code = `[${config.jsPlugins.map(printPlugin).join(', ')}]`;
    } else if (key === 'overrides' && config.overrides) {
      code = `[${config.overrides.map((override) => printConfig(override, [], { pluginExpression })).join(',\n')}]`;
    } else {
      code = JSON.stringify(value, null, 2);
    }
    return `  ${JSON.stringify(key)}: ${code.replaceAll('\n', '\n  ')}`;
  });
  return `{\n${properties.join(',\n')}\n}`;
}

function isRelativePluginPath(specifier: string): boolean {
  return /^\.{1,2}(?:[/\\]|$)/.test(specifier);
}

function moduleProvenance(file: OxlintConfigFile): string {
  return `// Generated by vp migrate from ${encodeURIComponent(path.basename(file.filename))}.`;
}

function reusableModulePath(file: OxlintConfigFile): string | undefined {
  const filename = file.filename.replace(/\.jsonc?$/i, '') + '.ts';
  if (
    !fs.existsSync(filename.replace(/\.ts$/, '.js')) &&
    fs.existsSync(filename) &&
    fs.readFileSync(filename, 'utf8').split(/\r?\n/, 1)[0] === moduleProvenance(file)
  ) {
    return filename;
  }
  return undefined;
}

export function hasGeneratedOxlintConfig(file: OxlintConfigFile): boolean {
  return reusableModulePath(file) !== undefined || file.extends.some(hasGeneratedOxlintConfig);
}

/** Generate portable shared modules, leaving their plugins in their original layers. */
export function prepareOxlintConfigModules(
  root: OxlintConfigFile,
  config: OxlintConfig,
  viteConfigPath: string,
  workspaceRoot: string,
): OxlintModulePlan {
  viteConfigPath = fs.realpathSync(viteConfigPath);
  const viteConfigDir = path.dirname(viteConfigPath);
  const source = fs.readFileSync(viteConfigPath, 'utf8');
  const names = new Set<string>();
  const { ast } = JSON.parse(analyzeMigrationSource(viteConfigPath, source), (key, value) => {
    if (key === 'name' && typeof value === 'string') {
      names.add(value);
    }
    return value;
  }) as { ast: { node: Program } };
  if (
    /\.c[jt]s$/.test(viteConfigPath) ||
    !ast.node.body.some((node) => node.type === 'ExportDefaultDeclaration')
  ) {
    throw new Error('Shared Oxlint configs require a Vite config with an ESM default export');
  }
  const modules: OxlintModulePlan['modules'] = new Map();
  const boundary = fs.realpathSync(workspaceRoot);
  let resolver: ResolverFactory | undefined;

  const validated = new Set<OxlintConfigFile>();
  function validate(file: OxlintConfigFile): void {
    if (validated.has(file)) {
      return;
    }
    validated.add(file);
    const relative = path.relative(boundary, file.filename);
    if (
      path.isAbsolute(relative) ||
      relative === '..' ||
      relative.startsWith(`..${path.sep}`) ||
      relative.split(path.sep).includes('node_modules')
    ) {
      throw new Error(
        `Shared Oxlint config is outside the project or in node_modules: ${file.filename}`,
      );
    }
    const configDir = path.dirname(file.filename);
    for (const layer of [file.config, ...(file.config.overrides ?? [])]) {
      for (const entry of layer.jsPlugins ?? []) {
        const specifier = typeof entry === 'string' ? entry : entry.specifier;
        if (
          isRelativePluginPath(specifier) ||
          path.isAbsolute(specifier) ||
          configDir === viteConfigDir
        ) {
          continue;
        }
        resolver ??= new ResolverFactory({ conditionNames: ['module-sync', 'node', 'import'] });
        const original = resolver.sync(configDir, specifier);
        const migrated = resolver.sync(viteConfigDir, specifier);
        if (original.path && original.path !== migrated.path) {
          throw new Error(
            `JS plugin "${specifier}" in ${file.filename} resolves differently from the Vite config directory; migrate this plugin manually`,
          );
        }
      }
    }
    file.extends.forEach(validate);
  }
  const importedFiles = root.shared ? [root] : root.extends;
  importedFiles.forEach(validate);

  function generate(file: OxlintConfigFile): string {
    const filename = file.filename.replace(/\.jsonc?$/i, '') + '.ts';
    const existing = modules.get(filename);
    if (existing && existing.file !== file) {
      throw new Error(`Multiple Oxlint configs would create ${filename}`);
    }
    if (file.modulePath) {
      return file.modulePath;
    }
    if (existing) {
      return filename;
    }
    const runtimePath = filename.replace(/\.ts$/, '.js');
    const reusable = reusableModulePath(file);
    if (reusable) {
      // Reuse earlier output, including user edits, without rewriting it.
      file.modulePath = reusable;
      return reusable;
    }
    if (fs.existsSync(filename) || fs.existsSync(runtimePath)) {
      throw new Error(
        `Cannot create shared Oxlint config because ${filename} or ${runtimePath} already exists`,
      );
    }
    const module = { file, content: '' };
    modules.set(filename, module);
    const imports = ['import type { OxlintConfig } from "vite-plus/lint";'];
    const inherited = file.extends.map((child, index) => {
      const name = `oxlintConfig${index + 1}`;
      imports.push(
        `import ${name} from ${JSON.stringify(relativeImport(filename, generate(child)))};`,
      );
      return name;
    });
    let needsFileUrl = false;
    function pluginExpression(specifier: string): string {
      if (isRelativePluginPath(specifier)) {
        needsFileUrl = true;
        const url = specifier.replaceAll('\\', '/').split('/').map(encodeURIComponent).join('/');
        return `fileURLToPath(new URL(${JSON.stringify(url)}, import.meta.url))`;
      }
      return JSON.stringify(specifier);
    }
    const config = structuredClone(file.config);
    ensureSvelteRuneGlobals(config);
    const body = printConfig(config, inherited, { pluginExpression });
    if (needsFileUrl) {
      imports.unshift('import { fileURLToPath } from "node:url";');
    }
    module.content = `${moduleProvenance(file)}\n${imports.join('\n')}\n\nexport default ${body} satisfies OxlintConfig;\n`;
    return filename;
  }

  const imports: string[] = [];
  let nextName = 1;
  const inherited = importedFiles.map((file) => {
    let name: string;
    do {
      name = `oxlintConfig${nextName++}`;
    } while (names.has(name));
    names.add(name);
    imports.push(
      `import ${name} from ${JSON.stringify(relativeImport(viteConfigPath, generate(file)))};`,
    );
    return name;
  });
  return {
    configContent: printConfig(config, inherited, {
      ownerConfigName: root.shared ? inherited[0] : undefined,
    }),
    imports,
    modules,
  };
}

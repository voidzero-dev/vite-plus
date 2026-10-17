import fs from 'node:fs';
import path from 'node:path';

import type { Program } from '@oxc-project/types';
import { parse as parseJsonc, type ParseError } from 'jsonc-parser';
import type { OxlintConfig } from 'oxlint';

import { analyzeMigrationSource, ResolverFactory } from '../../../binding/index.js';
import { detectConfigs } from '../detector.ts';
import { ensureSvelteRuneGlobals } from '../migrator.ts';
import type { OxlintConfigFile, OxlintModulePlan } from './shared.ts';

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
  const children = ((inherited ?? []) as string[]).map((entry) =>
    resolveOxlintConfigExtends(path.resolve(path.dirname(filename), entry), cache, [
      ...stack,
      filename,
    ]),
  );
  if (inherited !== undefined) {
    config.extends = children.map((child) => child.config);
  }
  const file: OxlintConfigFile = { filename, config, extends: children };
  cache.set(filename, file);
  return file;
}

export function prepareOxlintConfigs(
  projectPaths: string[],
): Map<string, OxlintConfigFile | Error> {
  const configs = new Map<string, OxlintConfigFile | Error>();
  const cache = new Map<string, OxlintConfigFile>();
  for (const projectPath of projectPaths) {
    const filename = detectConfigs(projectPath).oxlintConfig;
    if (!filename) {
      continue;
    }
    const configPath = path.join(projectPath, filename);
    try {
      configs.set(configPath, resolveOxlintConfigExtends(configPath, cache));
    } catch (error) {
      configs.set(configPath, error instanceof Error ? error : new Error(String(error)));
    }
  }
  return configs;
}

function relativeImport(from: string, to: string): string {
  const relative = path.relative(path.dirname(from), to).split(path.sep).join('/');
  if (path.isAbsolute(relative)) {
    throw new Error(`Cannot import an Oxlint config across drives: ${to}`);
  }
  // Vite and TypeScript resolve .js imports to .ts source files.
  // This also works with NodeNext without changing allowImportingTsExtensions.
  const specifier = /\.m?ts$/.test(from) ? relative.replace(/\.ts$/, '.js') : relative;
  return specifier.startsWith('../') ? specifier : `./${specifier}`;
}

function printConfig(
  config: OxlintConfig,
  inheritedNames: string[],
  pluginExpression?: (specifier: string) => string,
): string {
  const printPlugin = (entry: NonNullable<OxlintConfig['jsPlugins']>[number]): string => {
    const specifier = typeof entry === 'string' ? entry : entry.specifier;
    const expression = pluginExpression?.(specifier) ?? JSON.stringify(specifier);
    return typeof entry === 'string'
      ? expression
      : `{ "name": ${JSON.stringify(entry.name)}, "specifier": ${expression} }`;
  };
  return `{
${Object.entries(config)
  .map(([key, value]) => {
    let code = JSON.stringify(value, null, 2);
    if (key === 'extends') {
      code = `[${inheritedNames.join(', ')}]`;
    } else if (key === 'jsPlugins' && Array.isArray(config.jsPlugins)) {
      code = `[${config.jsPlugins.map(printPlugin).join(', ')}]`;
    } else if (key === 'overrides' && config.overrides) {
      code = `[${config.overrides.map((override) => printConfig(override, [], pluginExpression)).join(',\n')}]`;
    }
    return `  ${JSON.stringify(key)}: ${code.replaceAll('\n', '\n  ')}`;
  })
  .join(',\n')}
}`;
}

/** Generate portable shared modules, leaving their plugins in their original layers. */
export function prepareOxlintConfigModules(
  root: OxlintConfigFile,
  config: OxlintConfig,
  viteConfigPath: string,
  workspaceRoot: string,
): OxlintModulePlan {
  viteConfigPath = fs.realpathSync(viteConfigPath);
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
    for (const layer of [file.config, ...(file.config.overrides ?? [])]) {
      for (const entry of layer.jsPlugins ?? []) {
        const specifier = typeof entry === 'string' ? entry : entry.specifier;
        if (
          !/^\.{1,2}(?:[/\\]|$)/.test(specifier) &&
          !path.isAbsolute(specifier) &&
          path.dirname(file.filename) !== path.dirname(viteConfigPath)
        ) {
          resolver ??= new ResolverFactory({ conditionNames: ['module-sync', 'node', 'import'] });
          const original = resolver.sync(path.dirname(file.filename), specifier);
          const migrated = resolver.sync(path.dirname(viteConfigPath), specifier);
          if (original.path && original.path !== migrated.path) {
            throw new Error(
              `JS plugin "${specifier}" in ${file.filename} resolves differently from the Vite config directory; migrate this plugin manually`,
            );
          }
        }
      }
    }
    file.extends.forEach(validate);
  }
  root.extends.forEach(validate);

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
    if (fs.existsSync(filename) || fs.existsSync(runtimePath)) {
      throw new Error(
        `Cannot create shared Oxlint config because ${filename} or ${runtimePath} already exists`,
      );
    }
    modules.set(filename, { file, content: '' });
    const imports = ['import type { OxlintConfig } from "vite-plus/lint";'];
    const inherited = file.extends.map((child, index) => {
      const name = `oxlintConfig${index + 1}`;
      imports.push(
        `import ${name} from ${JSON.stringify(relativeImport(filename, generate(child)))};`,
      );
      return name;
    });
    let needsFileUrl = false;
    const pluginExpression = (specifier: string): string => {
      if (/^\.{1,2}(?:[/\\]|$)/.test(specifier)) {
        needsFileUrl = true;
        const url = specifier.replaceAll('\\', '/').split('/').map(encodeURIComponent).join('/');
        return `fileURLToPath(new URL(${JSON.stringify(url)}, import.meta.url))`;
      }
      return JSON.stringify(specifier);
    };
    const config = structuredClone(file.config);
    ensureSvelteRuneGlobals(config);
    const body = printConfig(config, inherited, pluginExpression);
    if (needsFileUrl) {
      imports.unshift('import { fileURLToPath } from "node:url";');
    }
    modules.set(filename, {
      file,
      content: `${imports.join('\n')}\n\nexport default ${body} satisfies OxlintConfig;\n`,
    });
    return filename;
  }

  const imports: string[] = [];
  let nextName = 1;
  const inherited = root.extends.map((file) => {
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
  return { configContent: printConfig(config, inherited), imports, modules };
}

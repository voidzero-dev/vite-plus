import fs from 'node:fs';
import path from 'node:path';

import type * as t from '@oxc-project/types';

import { analyzeMigrationSource, parseLiteralCommands } from '../../../binding/index.js';
import { readJsonFile } from '../../utils/json.ts';
import { detectConfigs } from '../detector.ts';

const CONFIG_MODULE_EXTENSIONS = ['.ts', '.mts', '.cts', '.tsx', '.js', '.mjs', '.cjs', '.jsx'];
const CONFIG_SOURCE_EXTENSIONS: Record<string, string[]> = {
  '.js': ['.ts', '.tsx'],
  '.mjs': ['.mts'],
  '.cjs': ['.cts'],
  '.jsx': ['.tsx'],
};

interface ConfigPackage {
  name?: string;
  scripts?: Record<string, string>;
  exports?: unknown;
  main?: string;
  module?: string;
}

interface SourceReference {
  node: t.Node;
  base: 'module' | 'cwd' | 'either';
  required: boolean;
}

function stringValue(node: t.Node | undefined): string | undefined {
  if (node?.type === 'Literal' && typeof node.value === 'string') {
    return node.value;
  }
  if (node?.type === 'TemplateLiteral' && node.expressions.length === 0) {
    return node.quasis[0].value.cooked ?? undefined;
  }
  return undefined;
}

function pathPrefix(node: t.Node): string | undefined {
  if (node.type === 'TemplateLiteral') {
    return node.quasis[0].value.cooked ?? undefined;
  }
  if (node.type === 'BinaryExpression' && node.operator === '+') {
    return pathPrefix(node.left);
  }
  return stringValue(node);
}

function callName(node: t.Node): string | undefined {
  if (node.type === 'Identifier') {
    return node.name;
  }
  if (node.type === 'MemberExpression') {
    return node.computed ? stringValue(node.property) : callName(node.property);
  }
  return undefined;
}

function moduleUrlArgument(node: t.Node | undefined): t.Node | undefined {
  if (node?.type !== 'NewExpression' || callName(node.callee) !== 'URL') {
    return undefined;
  }
  const base = node.arguments[1];
  if (
    base?.type === 'MemberExpression' &&
    callName(base) === 'url' &&
    base.object.type === 'MetaProperty' &&
    base.object.meta.name === 'import' &&
    base.object.property.name === 'meta'
  ) {
    return node.arguments[0];
  }
  return undefined;
}

function sourceReferences(filename: string, source: string) {
  const nodes: t.Node[] = [];
  JSON.parse(analyzeMigrationSource(filename, source), (_key, value) => {
    if (value && typeof value === 'object' && typeof value.type === 'string') {
      nodes.push(value as t.Node);
    }
    return value;
  });
  const reads = new Set([
    'readFile',
    'readFileSync',
    'open',
    'openSync',
    'access',
    'accessSync',
    'existsSync',
  ]);
  for (const node of nodes) {
    if (
      node.type === 'ImportDeclaration' &&
      /^(?:node:)?fs(?:\/promises)?$/.test(node.source.value)
    ) {
      for (const specifier of node.specifiers) {
        if (
          specifier.type === 'ImportSpecifier' &&
          reads.has(callName(specifier.imported) ?? stringValue(specifier.imported) ?? '')
        ) {
          reads.add(specifier.local.name);
        }
      }
    }
  }
  const references = new Map<t.Node, SourceReference>();
  const add = (node: t.Node | undefined, base: SourceReference['base'], required = true) => {
    if (node) {
      references.set(node, { node, base, required });
    }
  };
  let changesDirectory = false;
  for (const node of nodes) {
    if (
      node.type === 'ImportDeclaration' ||
      node.type === 'ExportAllDeclaration' ||
      node.type === 'ExportNamedDeclaration'
    ) {
      add(node.source ?? undefined, 'module');
    } else if (node.type === 'ImportExpression') {
      add(moduleUrlArgument(node.source) ?? node.source, 'module');
    } else if (node.type === 'NewExpression') {
      add(moduleUrlArgument(node), 'module');
    } else if (node.type === 'CallExpression') {
      const name = callName(node.callee);
      if (
        name === 'require' ||
        (name === 'resolve' &&
          node.callee.type === 'MemberExpression' &&
          callName(node.callee.object) === 'require')
      ) {
        add(node.arguments[0], 'module');
      } else if (reads.has(name ?? '')) {
        const url = moduleUrlArgument(node.arguments[0]);
        add(url ?? node.arguments[0], url ? 'module' : 'cwd');
      } else if (name === 'chdir') {
        changesDirectory = true;
      }
    }
  }
  // A string passed through an unknown helper can denote either an import or
  // a filesystem path. Keep both possibilities without overriding known uses.
  const known = [...references.keys()];
  for (const node of nodes) {
    if (
      stringValue(node) !== undefined &&
      !known.some((parent) => node.start >= parent.start && node.end <= parent.end)
    ) {
      add(node, 'either', false);
    }
  }
  return { references: [...references.values()], changesDirectory };
}

function commandContext(words: string[], directory: string | undefined) {
  let start = words.findIndex((word) => !/^[A-Za-z_]\w*=/.test(word));
  if (start < 0) {
    return { words: [], directory };
  }
  const manager = words[start];
  if (!['pnpm', 'npm', 'yarn', 'bun', 'npx', 'bunx'].includes(manager)) {
    return { words: words.slice(start), directory };
  }
  start++;
  for (; start < words.length; start++) {
    const word = words[start];
    const flag = word.split('=')[0];
    if (['--dir', '-C', '--cwd', '--prefix'].includes(flag)) {
      const target = word.includes('=') ? word.slice(word.indexOf('=') + 1) : words[++start];
      const resolved = directory && target ? path.resolve(directory, target) : undefined;
      directory =
        resolved && fs.existsSync(resolved) && fs.statSync(resolved).isDirectory()
          ? fs.realpathSync(resolved)
          : undefined;
    } else if (['--filter', '--recursive', '-r', '-R'].includes(flag)) {
      // Recursive/filter commands can execute in several workspace packages.
      directory = undefined;
      if (word === '--filter') {
        start++;
      }
    } else if (['exec', 'run', 'x'].includes(word)) {
      start++;
      if (words[start] === '--') {
        start++;
      }
      break;
    } else if (!word.startsWith('-')) {
      break;
    }
  }
  return { words: words.slice(start), directory };
}

function exportTargets(value: unknown): string[] {
  if (typeof value === 'string') {
    return [value];
  }
  // Inspect all conditions and fallback entries, independent of the config loader.
  return value && typeof value === 'object' ? Object.values(value).flatMap(exportTargets) : [];
}

function workspaceConfigEntries(pkg: ConfigPackage, subpath: string): string[] {
  if (pkg.exports === undefined) {
    if (subpath !== '.') {
      return [subpath];
    }
    const entries = [pkg.module, pkg.main].filter((entry): entry is string => !!entry);
    return entries.length ? entries : ['./index'];
  }
  if (
    pkg.exports &&
    typeof pkg.exports === 'object' &&
    !Array.isArray(pkg.exports) &&
    Object.keys(pkg.exports).some((key) => key.startsWith('.'))
  ) {
    const exports = pkg.exports as Record<string, unknown>;
    if (Object.hasOwn(exports, subpath)) {
      return exportTargets(exports[subpath]);
    }
    return Object.entries(exports).flatMap(([key, value]) => {
      const star = key.indexOf('*');
      if (
        star === -1 ||
        !subpath.startsWith(key.slice(0, star)) ||
        !subpath.endsWith(key.slice(star + 1)) ||
        subpath.length < key.length - 1
      ) {
        return [];
      }
      const match = subpath.slice(star, subpath.length - (key.length - star - 1));
      return exportTargets(value).map((target) => target.replaceAll('*', match));
    });
  }
  return subpath === '.' ? exportTargets(pkg.exports) : [];
}

function resolveConfigReferences(directory: string, reference: string): string[] | undefined {
  // Package imports can select different modules for different loaders or
  // conditions. An unresolved reference must never authorize deleting a config.
  if (reference.startsWith('#')) {
    return undefined;
  }
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
    if (fs.existsSync(candidate)) {
      if (fs.statSync(candidate).isFile()) {
        resolved.add(candidate);
      } else if (fs.existsSync(path.join(candidate, 'package.json'))) {
        // A directory's main/module/exports fields can supersede its index.
        // Keep the configs even if another candidate above was found.
        return undefined;
      }
    }
  }
  return [...resolved];
}

/** Find JSON files still needed by scripts, modules, or retained tool configs. */
export function findReferencedOxcConfigs(projectPaths: string[]): Set<string> {
  const projectDirectories = [...projectPaths]
    .filter((directory) => fs.existsSync(directory))
    .map((directory) => fs.realpathSync(directory))
    .toSorted((left, right) => right.length - left.length);
  // A helper module belongs to its nearest workspace package. Uncertain paths
  // there must not prevent unrelated packages from migrating their own configs.
  const referenceDirectory = (directory: string): string =>
    projectDirectories.find(
      (project) => directory === project || directory.startsWith(project + path.sep),
    ) ?? directory;
  const configReferences: { directory: string; content: string }[] = [];
  const lintConfigPaths = new Set<string>();
  const preservedConfigs = new Set<string>();
  function preserveJsonConfig(filename: string): void {
    preservedConfigs.add(fs.realpathSync(filename));
    // Match sources by identity, but resolve extends from the declaring alias.
    lintConfigPaths.add(
      path.join(fs.realpathSync(path.dirname(filename)), path.basename(filename)),
    );
  }
  const modules: { filename: string; directory: string | undefined }[] = [];
  const moduleDirectories = new Map<string, Set<string | undefined>>();
  function addModule(filename: string, directory: string | undefined) {
    filename = fs.realpathSync(filename);
    const directories = moduleDirectories.get(filename) ?? new Set();
    if (!directories.has(directory)) {
      directories.add(directory);
      moduleDirectories.set(filename, directories);
      modules.push({ filename, directory });
    }
  }
  const unresolvedConfigDirectories = new Set<string>();
  function preserveUncertainDirectory(directory: string | undefined) {
    for (const project of directory ? [referenceDirectory(directory)] : projectDirectories) {
      unresolvedConfigDirectories.add(project);
    }
  }
  const migrationCandidates = new Set<string>();
  const packages = new Map<string, ConfigPackage>();
  const workspacePackages = new Map<string, { directory: string; pkg: ConfigPackage }>();
  for (const directory of projectDirectories) {
    const packageJsonPath = path.join(directory, 'package.json');
    if (fs.existsSync(packageJsonPath)) {
      const pkg = readJsonFile(packageJsonPath) as ConfigPackage;
      packages.set(directory, pkg);
      if (pkg.name) {
        workspacePackages.set(pkg.name, { directory, pkg });
      }
    }
  }
  for (const directory of projectDirectories) {
    for (const script of Object.values(packages.get(directory)?.scripts ?? {})) {
      const commands = parseLiteralCommands(script, true);
      const onlyAndCommands = commands?.every(
        (tokens, index) =>
          index === 0 ||
          script.slice(commands[index - 1].at(-1)!.end, tokens[0].start).trim() === '&&',
      );
      const commandWords = commands?.map((tokens) => tokens.map(({ value }) => value)) ?? [
        (script.match(/(?:[^\s"';&|]+|"[^"]*"|'[^']*')+/g) ?? []).map((word) =>
          word.replace(/(["'])(.*?)\1/g, '$2'),
        ),
      ];
      // Unsupported shell syntax can change directories in a subshell or expand
      // an unknown path. In that case any workspace package can be the target.
      let workingDirectory: string | undefined =
        !commands && /\b(?:cd|pushd|popd)\b/.test(script) ? undefined : directory;
      for (let words of commandWords) {
        if (commands && ['cd', 'pushd', 'popd'].includes(words[0])) {
          const args = words.slice(words[1] === '--' || words[1] === '/d' ? 2 : 1);
          // Other control flow can skip cd and still execute a later command.
          // Only resolve a literal directory change in an && chain.
          const target =
            words[0] === 'cd' &&
            args.length === 1 &&
            args[0] !== '-' &&
            !args[0].startsWith('~') &&
            workingDirectory &&
            onlyAndCommands
              ? path.resolve(workingDirectory, args[0])
              : undefined;
          workingDirectory =
            target && fs.existsSync(target) && fs.statSync(target).isDirectory()
              ? fs.realpathSync(target)
              : undefined;
          continue;
        }
        const command = words.find(
          (word) => !/^[A-Za-z_]\w*=/.test(word) && word !== 'command' && word !== 'builtin',
        );
        if (command && ['cd', 'pushd', 'popd'].includes(command)) {
          workingDirectory = undefined;
        }
        const context = commandContext(words, workingDirectory);
        const commandDirectory = context.directory;
        words = context.words;
        const owner = commandDirectory ? referenceDirectory(commandDirectory) : directory;
        const runtimeArgs =
          words[0] === 'vp' && words[1] === 'node'
            ? words.slice(2)
            : ['node', 'tsx', 'ts-node', 'oxnode', 'bun', 'vpx'].includes(words[0])
              ? words.slice(1)
              : CONFIG_MODULE_EXTENSIONS.includes(path.extname(words[0] ?? ''))
                ? words
                : undefined;
        if (runtimeArgs) {
          let found = false;
          for (const argument of runtimeArgs) {
            if (
              !argument.startsWith('-') &&
              CONFIG_MODULE_EXTENSIONS.includes(path.extname(argument))
            ) {
              for (const base of commandDirectory ? [commandDirectory] : projectDirectories) {
                for (const filename of resolveConfigReferences(base, argument) ?? []) {
                  addModule(filename, commandDirectory);
                  found = true;
                }
              }
            }
          }
          if (
            !found ||
            runtimeArgs.some((arg) => ['-e', '-p', '--eval', '--print'].includes(arg))
          ) {
            preserveUncertainDirectory(commandDirectory);
          }
        }
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
          if (!commandDirectory) {
            preserveUncertainDirectory(undefined);
            continue;
          }
          const filenames = resolveConfigReferences(commandDirectory, reference);
          if (!filenames?.length) {
            unresolvedConfigDirectories.add(owner);
          } else {
            // Known references are matched by file identity below. Do not let
            // their basenames also preserve a different config in this package.
            words[index] = '';
          }
          for (const filename of filenames ?? []) {
            if (/\.jsonc?$/.test(filename)) {
              preserveJsonConfig(filename);
            } else if (CONFIG_MODULE_EXTENSIONS.includes(path.extname(filename))) {
              addModule(filename, commandDirectory);
            } else {
              unresolvedConfigDirectories.add(owner);
            }
          }
        }
        for (const project of commandDirectory ? [owner] : projectDirectories) {
          configReferences.push({ directory: project, content: words.join(' ') });
        }
      }
    }
    const projectConfigs = detectConfigs(directory);
    if (projectConfigs.oxlintConfig) {
      migrationCandidates.add(path.join(directory, projectConfigs.oxlintConfig));
    }
    if (projectConfigs.viteConfig) {
      addModule(path.join(directory, projectConfigs.viteConfig), directory);
    }
  }
  // A workspace without its own Vite config can load an ancestor's config while
  // retaining its own process cwd.
  for (const directory of projectDirectories) {
    if (!detectConfigs(directory).viteConfig) {
      const ancestor = projectDirectories.find(
        (candidate) =>
          directory.startsWith(candidate + path.sep) && detectConfigs(candidate).viteConfig,
      );
      if (ancestor) {
        addModule(path.join(ancestor, detectConfigs(ancestor).viteConfig!), directory);
      }
    }
  }
  const analyses = new Map<string, ReturnType<typeof sourceReferences>>();
  for (const { filename: modulePath, directory: consumerDirectory } of modules) {
    const source = fs.readFileSync(modulePath, 'utf8');
    const provenance = source.match(/^\/\/ Generated by vp migrate from (\S+)\.\r?(?:\n|$)/);
    if (provenance) {
      // Custom JSON sources remain on disk after conversion. On a retry their
      // workspace consumer may be gone, but their inherited sources are still needed.
      try {
        const filename = path.resolve(path.dirname(modulePath), decodeURIComponent(provenance[1]));
        if (fs.existsSync(filename) && !migrationCandidates.has(filename)) {
          preserveJsonConfig(filename);
        }
      } catch {
        // A user-edited provenance comment is not a resolvable source reference.
      }
    }
    const content = source
      // Migration bookkeeping names a source without consuming it at runtime.
      .replace(/^\/\/ vp migrate: (?:pending \S+\.|merged \S+ \([a-f0-9]{32}\)\.)\r?$/gm, '')
      .replace(/^\/\/ Generated by vp migrate from \S+\.\r?$/gm, '');
    const directory = referenceDirectory(path.dirname(modulePath));
    let analysis = analyses.get(modulePath);
    if (!analysis) {
      try {
        analysis = sourceReferences(modulePath, content);
        analyses.set(modulePath, analysis);
      } catch {
        // Unsupported syntax must not authorize removing dependencies we cannot inspect.
        preserveUncertainDirectory(undefined);
        continue;
      }
    }
    const cwd = analysis.changesDirectory ? undefined : consumerDirectory;
    let unresolvedContent = content;
    for (const { node, base, required } of analysis.references.toSorted(
      (a, b) => b.node.start - a.node.start,
    )) {
      const reference = stringValue(node);
      if (reference === undefined) {
        const prefix = pathPrefix(node);
        const target =
          prefix && /^(?:\.{1,2}\/|\/)/.test(prefix)
            ? path.resolve(
                base === 'cwd' ? (cwd ?? directory) : path.dirname(modulePath),
                path.dirname(prefix + '_'),
              )
            : undefined;
        preserveUncertainDirectory(target);
        // An unresolved module can also contain reads relative to its consumer.
        preserveUncertainDirectory(cwd);
        continue;
      }
      const name = reference.match(/^(?:@[^/]+\/)?[^/]+/)?.[0] ?? '';
      const workspace = base !== 'cwd' && workspacePackages.get(name);
      const relative = /^(?:\.{1,2}\/|\/|#)/.test(reference);
      const file = base === 'cwd' || relative || /\.jsonc?$/.test(reference);
      if (!file && !workspace) {
        continue;
      }
      const filenames = new Set<string>();
      if (workspace && !relative) {
        const { directory: packageDirectory, pkg } = workspace;
        const entries = workspaceConfigEntries(pkg, `.${reference.slice(name.length)}`);
        for (const entry of entries) {
          const resolved = resolveConfigReferences(packageDirectory, entry);
          if (!resolved?.length) {
            preserveUncertainDirectory(packageDirectory);
          }
          for (const filename of resolved ?? []) {
            filenames.add(filename);
          }
        }
        if (!entries.length) {
          preserveUncertainDirectory(packageDirectory);
        }
      } else {
        const bases = new Set<string>();
        if (base !== 'cwd') {
          bases.add(path.dirname(modulePath));
        }
        if (base !== 'module') {
          if (cwd) {
            bases.add(cwd);
          } else {
            preserveUncertainDirectory(undefined);
          }
        }
        for (const origin of bases) {
          const resolved = resolveConfigReferences(origin, reference);
          if (required && !resolved?.length) {
            preserveUncertainDirectory(path.dirname(path.resolve(origin, reference)));
            if (base === 'module') {
              preserveUncertainDirectory(cwd);
            }
          }
          for (const filename of resolved ?? []) {
            filenames.add(filename);
          }
        }
      }
      for (const filename of filenames) {
        if (/\.jsonc?$/.test(filename)) {
          preserveJsonConfig(filename);
        } else if (CONFIG_MODULE_EXTENSIONS.includes(path.extname(filename))) {
          addModule(filename, cwd);
        }
      }
      if (filenames.size || required) {
        // Known references must not also retain unrelated same-named configs.
        unresolvedContent =
          unresolvedContent.slice(0, node.start) + unresolvedContent.slice(node.end);
      }
    }
    for (const owner of new Set([
      directory,
      ...(cwd ? [referenceDirectory(cwd)] : projectDirectories),
    ])) {
      configReferences.push({ directory: owner, content: unresolvedContent });
    }
  }
  for (const directory of projectDirectories) {
    const configs = detectConfigs(directory);
    for (const filename of [configs.oxlintConfig, configs.oxfmtConfig]) {
      if (!filename) {
        continue;
      }
      if (
        unresolvedConfigDirectories.has(directory) ||
        configReferences.some(
          (reference) => reference.directory === directory && reference.content.includes(filename),
        )
      ) {
        preserveJsonConfig(path.join(directory, filename));
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
      // A retained JSON consumer still needs its complete inheritance chain.
      for (const extendedConfig of json.extends) {
        if (typeof extendedConfig === 'string') {
          const resolvedPath = path.resolve(path.dirname(lintConfigPath), extendedConfig);
          if (fs.existsSync(resolvedPath)) {
            preserveJsonConfig(resolvedPath);
          }
        }
      }
    }
  }
  return preservedConfigs;
}

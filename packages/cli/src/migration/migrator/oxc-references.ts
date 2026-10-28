import fs from 'node:fs';
import path from 'node:path';

import { parseLiteralCommands } from '../../../binding/index.js';
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
  const modulePaths = new Set<string>();
  const unresolvedConfigDirectories = new Set<string>();
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
      for (const words of commandWords) {
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
        const owner = workingDirectory ? referenceDirectory(workingDirectory) : directory;
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
          if (!workingDirectory) {
            for (const project of projectDirectories) {
              unresolvedConfigDirectories.add(project);
            }
            continue;
          }
          const filenames = resolveConfigReferences(workingDirectory, reference);
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
              modulePaths.add(fs.realpathSync(filename));
            } else {
              unresolvedConfigDirectories.add(owner);
            }
          }
        }
        configReferences.push({ directory: owner, content: words.join(' ') });
      }
    }
    const projectConfigs = detectConfigs(directory);
    if (projectConfigs.oxlintConfig) {
      migrationCandidates.add(path.join(directory, projectConfigs.oxlintConfig));
    }
    if (projectConfigs.viteConfig) {
      modulePaths.add(fs.realpathSync(path.join(directory, projectConfigs.viteConfig)));
    }
  }
  for (const modulePath of modulePaths) {
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
    // Quoted paths and package aliases cover imports, re-exports, require(),
    // dynamic import(), and readFileSync/new URL calls, including comments
    // between tokens. Extra matches only preserve more configs.
    // Canonical paths keep cycles, including symlink cycles, finite.
    const unresolvedContent = content.replace(
      /(["'`])([^"'`\r\n]+)\1/g,
      (match, _quote, reference: string) => {
        const name = reference.match(/^(?:@[^/]+\/)?[^/]+/)?.[0] ?? '';
        const workspace = workspacePackages.get(name);
        let filenames: string[] | undefined;
        if (/^(?:\.{1,2}\/|\/|#)/.test(reference)) {
          filenames = resolveConfigReferences(path.dirname(modulePath), reference);
        } else if (workspace) {
          const { directory: packageDirectory, pkg } = workspace;
          const entries = workspaceConfigEntries(pkg, `.${reference.slice(name.length)}`);
          filenames = [];
          for (const entry of entries) {
            const resolved = resolveConfigReferences(packageDirectory, entry);
            if (!resolved?.length) {
              unresolvedConfigDirectories.add(packageDirectory);
            }
            filenames.push(...(resolved ?? []));
          }
          if (!entries.length) {
            unresolvedConfigDirectories.add(packageDirectory);
          }
        } else {
          return match;
        }
        if (!filenames) {
          unresolvedConfigDirectories.add(directory);
        }
        for (const filename of filenames ?? []) {
          if (/\.jsonc?$/.test(filename)) {
            preserveJsonConfig(filename);
          } else if (CONFIG_MODULE_EXTENSIONS.includes(path.extname(filename))) {
            modulePaths.add(fs.realpathSync(filename));
          }
        }
        return filenames?.length ? '' : match;
      },
    );
    configReferences.push({ directory, content: unresolvedContent });
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

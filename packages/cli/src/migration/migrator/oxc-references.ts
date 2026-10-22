import fs from 'node:fs';
import path from 'node:path';

import { readJsonFile } from '../../utils/json.ts';
import { detectConfigs } from '../detector.ts';

const CONFIG_MODULE_EXTENSIONS = ['.ts', '.mts', '.cts', '.tsx', '.js', '.mjs', '.cjs', '.jsx'];
const CONFIG_SOURCE_EXTENSIONS: Record<string, string[]> = {
  '.js': ['.ts', '.tsx'],
  '.mjs': ['.mts'],
  '.cjs': ['.cts'],
  '.jsx': ['.tsx'],
};

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
  for (const directory of projectDirectories) {
    const packageJsonPath = path.join(directory, 'package.json');
    if (fs.existsSync(packageJsonPath)) {
      const pkg = readJsonFile(packageJsonPath) as { scripts?: Record<string, string> };
      for (const script of Object.values(pkg.scripts ?? {})) {
        // Inspect config arguments without executing shell commands. An unknown
        // path (for example an environment variable or a preceding `cd`) means
        // we cannot safely decide that a standalone config is unused.
        const words = (script.match(/(?:[^\s"';&|]+|"[^"]*"|'[^']*')+/g) ?? []).map((word) =>
          word.replace(/(["'])(.*?)\1/g, '$2'),
        );
        const changesDirectory = words.includes('cd');
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
          const filenames = resolveConfigReferences(directory, reference);
          if (!filenames?.length || changesDirectory) {
            unresolvedConfigDirectories.add(directory);
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
              unresolvedConfigDirectories.add(directory);
            }
          }
        }
        configReferences.push({ directory, content: words.join(' ') });
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
      /(["'`])((?:\.{1,2}\/|\/|#)[^"'`\r\n]+)\1/g,
      (match, _quote, reference: string) => {
        const filenames = resolveConfigReferences(path.dirname(modulePath), reference);
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

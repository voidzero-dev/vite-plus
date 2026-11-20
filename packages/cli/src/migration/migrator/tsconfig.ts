import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import {
  applyEdits,
  createScanner,
  findNodeAtLocation,
  parse as parseJsonc,
  parseTree,
  type ParseError,
} from 'jsonc-parser';
import { minimatch } from 'minimatch';

import { ResolverFactory } from '../../../binding/index.js';
import { appendMissingJsoncArrayItems, detectFormattingOptions } from '../../utils/json.ts';
import { displayRelative } from '../../utils/path.ts';
import {
  findTsconfigFiles,
  hasTypesToRewriteInTsconfig,
  removeDeprecatedTsconfigFalseOption,
  rewriteTypesInTsconfig,
} from '../../utils/tsconfig.ts';
import { type MigrationReport } from '../report.ts';
import { warnMigration } from './shared.ts';

export function cleanupDeprecatedTsconfigOptions(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
): void {
  const deprecatedOptions = ['esModuleInterop', 'allowSyntheticDefaultImports'];
  const files = findTsconfigFiles(projectPath);
  for (const filePath of files) {
    for (const name of deprecatedOptions) {
      if (removeDeprecatedTsconfigFalseOption(filePath, name)) {
        if (report) {
          report.removedConfigCount++;
        }
        if (!silent) {
          prompts.log.success(`✔ Removed ${name}: false from ${displayRelative(filePath)}`);
        }
        warnMigration(
          `Removed \`"${name}": false\` from ${displayRelative(filePath)} — this option has been deprecated. See https://github.com/oxc-project/tsgolint/issues/351, https://github.com/microsoft/TypeScript/issues/62529`,
          report,
        );
      }
    }
  }
}

export function rewriteTsconfigTypes(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
  keepTsdown = false,
): boolean {
  let changed = false;
  const files = findTsconfigFiles(projectPath);
  for (const filePath of files) {
    if (rewriteTypesInTsconfig(filePath, keepTsdown)) {
      changed = true;
      if (report) {
        report.removedConfigCount++;
      }
      if (!silent) {
        prompts.log.success(`✔ Rewrote types in ${displayRelative(filePath)}`);
      }
    }
  }
  return changed;
}

export function hasTsconfigTypesToRewrite(projectPath: string, keepTsdown = false): boolean {
  return findTsconfigFiles(projectPath).some((filePath) =>
    hasTypesToRewriteInTsconfig(filePath, keepTsdown),
  );
}

const projectFileKeys = ['files', 'include', 'exclude'] as const;
const javaScriptOptionKeys = ['allowJs', 'checkJs'] as const;
const configDirTemplate = '${configDir}';
type ProjectFileKey = (typeof projectFileKeys)[number];
type ProjectFileList = { filename: string; entries: string[] };
type ProjectFileLists = Partial<Record<ProjectFileKey, ProjectFileList>>;
type JavaScriptOptions = Partial<Record<(typeof javaScriptOptionKeys)[number], boolean>>;
type ProjectFileConfig = {
  text: string;
  lists: ProjectFileLists;
  javaScript: JavaScriptOptions;
  references?: { path?: unknown }[];
  missingPreset?: string;
};

function readProjectFileLists(
  filename: string,
  resolver: ResolverFactory,
  edits: ReadonlyMap<string, string>,
  ancestors = new Set<string>(),
): ProjectFileConfig | undefined {
  if (ancestors.has(filename) || !fs.statSync(filename, { throwIfNoEntry: false })?.isFile()) {
    return undefined;
  }
  const text = edits.get(filename) ?? fs.readFileSync(filename, 'utf8');
  const errors: ParseError[] = [];
  const config = parseJsonc(text, errors, { allowTrailingComma: true }) as
    | ({
        extends?: unknown;
        compilerOptions?: Partial<Record<keyof JavaScriptOptions, unknown>>;
        references?: ProjectFileConfig['references'];
      } & Partial<Record<ProjectFileKey, unknown>>)
    | undefined;
  if (!config || errors.length) {
    return undefined;
  }
  const directory = path.dirname(filename);
  const lists: ProjectFileLists = {};
  const javaScript: JavaScriptOptions = {};
  let missingPreset: string | undefined;
  const bases = Array.isArray(config.extends) ? config.extends : [config.extends];
  ancestors.add(filename);
  for (const base of bases) {
    if (typeof base !== 'string' || base === '') {
      continue;
    }
    const resolved = resolveProjectConfigBase(directory, base, resolver);
    const inherited = resolved
      ? readProjectFileLists(resolved, resolver, edits, ancestors)
      : undefined;
    if (!inherited) {
      missingPreset ??= base;
      continue;
    }
    Object.assign(lists, inherited.lists);
    Object.assign(javaScript, inherited.javaScript);
    missingPreset ??= inherited.missingPreset;
  }
  ancestors.delete(filename);
  for (const key of projectFileKeys) {
    const entries = config[key];
    if (Array.isArray(entries) && entries.every((entry) => typeof entry === 'string')) {
      lists[key] = { filename, entries };
    }
  }
  for (const key of javaScriptOptionKeys) {
    const value = config.compilerOptions?.[key];
    if (typeof value === 'boolean') {
      javaScript[key] = value;
    }
  }
  // Project references are local to this config. TypeScript does not inherit them.
  return { text, lists, javaScript, references: config.references, missingPreset };
}

function resolveProjectConfigBase(
  directory: string,
  base: string,
  resolver: ResolverFactory,
): string | undefined {
  const specifier = base.replaceAll('\\', '/');
  if (!specifier.startsWith('./') && !specifier.startsWith('../') && !path.isAbsolute(specifier)) {
    return resolver.sync(directory, specifier).path;
  }
  const target = path.resolve(directory, specifier);
  return fs.statSync(target, { throwIfNoEntry: false })?.isFile() || target.endsWith('.json')
    ? target
    : `${target}.json`;
}

type PackConfigTsconfigEdits = {
  apply: () => boolean;
  missingPreset?: { filename: string; specifier: string };
  excludedConfig?: string;
};

function coversProjectFile(project: ProjectFileConfig, filename: string, file: string): boolean {
  const { files, include, exclude } = project.lists;
  const directory = path.dirname(filename);
  const includes = include ?? (files ? undefined : { filename, entries: ['**/*'] });
  return !!(
    matchTsconfigPaths(files, directory, file, false) ||
    (matchTsconfigPaths(includes, directory, file, true) &&
      !matchTsconfigPaths(exclude, directory, file, true))
  );
}

function removeProjectFileEntry(text: string, key: ProjectFileKey, index: number): string {
  const tree = parseTree(text);
  const elements = tree && findNodeAtLocation(tree, [key])?.children;
  const entry = elements?.[index];
  if (!entry) {
    return text;
  }
  // Delete the string and its adjacent comma without reprinting the array.
  // jsonc-parser's array removal can consume the previous quote in compact JSON.
  const scanner = createScanner(text, true);
  scanner.setPosition(entry.offset + entry.length);
  scanner.scan();
  const previous = elements?.[index - 1];
  if (text[scanner.getTokenOffset()] !== ',' && previous) {
    scanner.setPosition(previous.offset + previous.length);
    scanner.scan();
  }
  const edits = [{ offset: entry.offset, length: entry.length, content: '' }];
  const comma = scanner.getTokenOffset();
  if (text[comma] === ',') {
    edits.push({ offset: comma, length: 1, content: '' });
  }
  return applyEdits(text, edits);
}

function removeReferencedPackFile(
  filename: string,
  project: ProjectFileConfig,
  generatedPath: string,
): string {
  let text = project.text;
  for (const key of ['files', 'include'] as const) {
    const list = project.lists[key];
    if (list?.filename !== filename) {
      continue;
    }
    const directory = path.dirname(filename);
    // Earlier migrations could add the exact generated path to the consumer.
    // Keep unrelated entries and patterns when moving it to the owning project.
    for (let index = list.entries.length - 1; index >= 0; index--) {
      const entry = resolveTsconfigEntry(list, directory, list.entries[index]);
      if (entry === generatedPath) {
        text = removeProjectFileEntry(text, key, index);
      }
    }
  }
  return text;
}

/** Prepare edits where TypeScript owns the input config, including referenced projects. */
export function preparePackConfigTsconfigEdits(
  viteConfigPath: string,
  generatedPath: string,
  workspaceRoot: string,
  originalPath?: string,
): PackConfigTsconfigEdits {
  const isJavaScript = /\.[cm]?js$/.test(generatedPath);
  if (!isJavaScript && !/\.[cm]?ts$/.test(generatedPath)) {
    return { apply: () => false };
  }
  const root = path.resolve(workspaceRoot);
  const realRoot = fs.realpathSync(root);
  // TypeScript resolves package presets as JSON using the NodeNext CJS conditions.
  const resolver = new ResolverFactory({
    extensions: ['.json'],
    conditionNames: ['require', 'types', 'node'],
    mainFields: ['tsconfig'],
    mainFiles: ['tsconfig'],
  });
  const filenames = new Set<string>();
  for (
    let directory = path.dirname(path.resolve(viteConfigPath));
    ;
    directory = path.dirname(directory)
  ) {
    for (const file of findTsconfigFiles(directory)) {
      filenames.add(file);
    }
    if (directory === root || directory === path.dirname(directory)) {
      break;
    }
  }
  const visited = new Set<string>();
  const edits = new Map<string, string>();
  const projects = new Map<string, { config: ProjectFileConfig; references: string[] }>();
  for (const filename of filenames) {
    if (!fs.lstatSync(filename, { throwIfNoEntry: false })?.isFile()) {
      continue;
    }
    const realFilename = fs.realpathSync(filename);
    const fromRoot = path.relative(realRoot, realFilename);
    if (
      path.isAbsolute(fromRoot) ||
      fromRoot.split(path.sep)[0] === '..' ||
      fromRoot.split(path.sep).includes('node_modules') ||
      visited.has(realFilename)
    ) {
      continue;
    }
    visited.add(realFilename);
    const project = readProjectFileLists(filename, resolver, edits);
    if (!project) {
      continue;
    }
    const directory = path.dirname(filename);
    const references: string[] = [];
    for (const reference of Array.isArray(project.references) ? project.references : []) {
      if (typeof reference?.path === 'string') {
        const target = path.resolve(directory, reference.path.replaceAll('\\', '/'));
        const config = fs.statSync(target, { throwIfNoEntry: false })?.isDirectory()
          ? path.join(target, 'tsconfig.json')
          : target;
        references.push(config);
        filenames.add(config);
      }
    }
    projects.set(filename, { config: project, references });
  }

  const owners = new Set<string>();
  const consumers = new Set<string>();
  if (originalPath) {
    for (const [filename, { config, references }] of projects) {
      if (!coversProjectFile(config, filename, viteConfigPath)) {
        continue;
      }
      const pending = new Set(references);
      pending.delete(filename);
      for (const reference of pending) {
        const target = projects.get(reference);
        if (!target) {
          continue;
        }
        const { javaScript } = target.config;
        if (
          (!isJavaScript || (javaScript.allowJs ?? javaScript.checkJs)) &&
          coversProjectFile(target.config, reference, originalPath)
        ) {
          owners.add(reference);
          consumers.add(filename);
        }
        for (const child of target.references) {
          if (child !== filename) {
            pending.add(child);
          }
        }
      }
    }
  }

  let missingPreset: PackConfigTsconfigEdits['missingPreset'];
  let excludedConfig: PackConfigTsconfigEdits['excludedConfig'];
  for (const [filename, { config }] of projects) {
    const directory = path.dirname(filename);
    // An earlier edit can change effective paths inherited by a later project.
    const project = edits.size
      ? (readProjectFileLists(filename, resolver, edits) ?? config)
      : config;
    const { text, lists, javaScript } = project;
    if (consumers.has(filename) && !owners.has(filename)) {
      const updated = removeReferencedPackFile(filename, project, generatedPath);
      if (updated !== text) {
        edits.set(filename, updated);
      }
      continue;
    }

    const listed = (file: string) => matchTsconfigPaths(lists.files, directory, file, false);
    const excluded = (file: string) => matchTsconfigPaths(lists.exclude, directory, file, true);
    const includesPattern = (file: string) =>
      matchTsconfigPaths(lists.include, directory, file, true);
    const included = (file: string) => (excluded(file) ? undefined : includesPattern(file));
    const inputPath = owners.has(filename) ? (originalPath ?? viteConfigPath) : viteConfigPath;
    const explicitlyListed = listed(inputPath);
    const includedViteConfig = included(inputPath);
    const generatedListed = listed(generatedPath);
    if (project.missingPreset) {
      // Explicit local files can safely be edited without knowing inherited
      // patterns. Otherwise the unresolved preset may own the project scope.
      const hasLocalFiles = lists.files?.filename === filename;
      const unrelated =
        hasLocalFiles &&
        lists.include?.filename === filename &&
        !explicitlyListed &&
        !includesPattern(inputPath);
      if (unrelated) {
        continue;
      }
      if (isJavaScript || !hasLocalFiles || (!explicitlyListed && !generatedListed)) {
        missingPreset ??= { filename, specifier: project.missingPreset };
        continue;
      }
    }
    // TypeScript defaults allowJs to checkJs when allowJs is omitted.
    if (isJavaScript && !(javaScript.allowJs ?? javaScript.checkJs)) {
      continue;
    }
    if (generatedListed || (!project.missingPreset && included(generatedPath))) {
      continue;
    }
    if (!explicitlyListed && !includedViteConfig) {
      continue;
    }
    const usesConfigDir = explicitlyListed !== 'fixed' && includedViteConfig !== 'fixed';
    const generatedExcluded = excluded(generatedPath);
    if (usesConfigDir && project.missingPreset) {
      missingPreset ??= { filename, specifier: project.missingPreset };
      continue;
    }
    if (usesConfigDir && generatedExcluded) {
      // A required configDir files entry would also require this file in derived projects.
      excludedConfig ??= filename;
      continue;
    }
    // Project-relative includes let derived projects omit the generated file.
    // A fixed files entry bypasses exclude, as it does for an original explicit file.
    const key = !usesConfigDir && (explicitlyListed || generatedExcluded) ? 'files' : 'include';
    const relative = path.relative(directory, generatedPath).split(path.sep).join('/');
    const selected = lists[key];
    const inherited = selected && selected.filename !== filename;
    const rebased = inherited
      ? selected.entries.map((entry) =>
          configDirPath(entry) === undefined
            ? path
                .relative(directory, resolveTsconfigEntry(selected, directory, entry))
                .split(path.sep)
                .join('/')
            : entry,
        )
      : [];
    // Local keys override inherited keys. Carry over the original paths with
    // their original base so the addition preserves the rest of the project.
    const updated = appendMissingJsoncArrayItems(
      text,
      [key],
      inherited ? undefined : selected?.entries,
      [...rebased, usesConfigDir ? `${configDirTemplate}/${relative}` : relative],
      detectFormattingOptions(text),
    );
    if (updated !== text) {
      edits.set(filename, updated);
    }
  }
  return {
    missingPreset,
    excludedConfig,
    apply() {
      for (const [filename, text] of edits) {
        fs.writeFileSync(filename, text);
      }
      return edits.size > 0;
    },
  };
}

function matchTsconfigPaths(
  list: ProjectFileList | undefined,
  projectDirectory: string,
  filename: string,
  allowGlob: boolean,
): 'fixed' | 'configDir' | undefined {
  if (!list) {
    return undefined;
  }
  const file = path.resolve(filename).split(path.sep).join('/');
  let match: 'configDir' | undefined;
  for (const entry of list.entries) {
    const fullPath = resolveTsconfigEntry(list, projectDirectory, entry).split(path.sep).join('/');
    // TypeScript treats a directory with no wildcard as a recursive include.
    const pattern = /[.*?]/.test(path.posix.basename(fullPath)) ? fullPath : `${fullPath}/**/*`;
    const matches = allowGlob
      ? minimatch(file, pattern, {
          dot: true,
          nocase: process.platform === 'win32',
          nobrace: true,
          nocomment: true,
          noext: true,
          nonegate: true,
        })
      : fullPath === file;
    if (matches) {
      if (configDirPath(entry) === undefined) {
        return 'fixed';
      }
      match = 'configDir';
    }
  }
  return match;
}

function configDirPath(entry: string): string | undefined {
  const specifier = entry.replaceAll('\\', '/');
  if (specifier === configDirTemplate || specifier.startsWith(configDirTemplate + '/')) {
    return specifier.slice(configDirTemplate.length + 1);
  }
  return undefined;
}

function resolveTsconfigEntry(
  list: ProjectFileList,
  projectDirectory: string,
  entry: string,
): string {
  const projectPath = configDirPath(entry);
  return projectPath === undefined
    ? path.resolve(path.dirname(list.filename), entry.replaceAll('\\', '/'))
    : path.resolve(projectDirectory, projectPath);
}

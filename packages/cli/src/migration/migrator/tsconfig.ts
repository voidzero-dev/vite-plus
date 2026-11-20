import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import { parse as parseJsonc, type ParseError } from 'jsonc-parser';
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
          `Removed \`"${name}": false\` from ${displayRelative(filePath)} — this option has been deprecated. See https://github.com/microsoft/TypeScript/issues/62529`,
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

/** Prepare edits for each TypeScript project that imports the generated config. */
export function preparePackConfigTsconfigEdits(
  viteConfigPath: string,
  generatedPath: string,
  workspaceRoot: string,
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
  let missingPreset: PackConfigTsconfigEdits['missingPreset'];
  let excludedConfig: PackConfigTsconfigEdits['excludedConfig'];
  for (const filename of filenames) {
    if (!fs.lstatSync(filename, { throwIfNoEntry: false })?.isFile()) {
      continue;
    }
    const realFilename = fs.realpathSync(filename);
    const fromRoot = path.relative(realRoot, realFilename);
    if (
      path.isAbsolute(fromRoot) ||
      fromRoot.split(path.sep)[0] === '..' ||
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
    const { text, lists, javaScript, references } = project;
    if (Array.isArray(references)) {
      for (const reference of references) {
        if (typeof reference?.path === 'string') {
          const target = path.resolve(directory, reference.path.replaceAll('\\', '/'));
          filenames.add(
            fs.statSync(target, { throwIfNoEntry: false })?.isDirectory()
              ? path.join(target, 'tsconfig.json')
              : target,
          );
        }
      }
    }

    const listed = (file: string) => matchTsconfigPaths(lists.files, directory, file, false);
    const excluded = (file: string) => matchTsconfigPaths(lists.exclude, directory, file, true);
    const includesPattern = (file: string) =>
      matchTsconfigPaths(lists.include, directory, file, true);
    const included = (file: string) => (excluded(file) ? undefined : includesPattern(file));
    const explicitlyListed = listed(viteConfigPath);
    const includedViteConfig = included(viteConfigPath);
    const generatedListed = listed(generatedPath);
    if (project.missingPreset) {
      // Explicit local files can safely be edited without knowing inherited
      // patterns. Otherwise the unresolved preset may own the project scope.
      const hasLocalFiles = lists.files?.filename === filename;
      const unrelated =
        hasLocalFiles &&
        lists.include?.filename === filename &&
        !explicitlyListed &&
        !includesPattern(viteConfigPath);
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

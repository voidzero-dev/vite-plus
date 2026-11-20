import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import { parse as parseJsonc, type ParseError } from 'jsonc-parser';
import { minimatch } from 'minimatch';

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
type ProjectFileKey = (typeof projectFileKeys)[number];
type ProjectFileList = { filename: string; entries: string[] };
type ProjectFileLists = Partial<Record<ProjectFileKey, ProjectFileList>>;

function readProjectFileLists(
  filename: string,
  ancestors = new Set<string>(),
): { text: string; lists: ProjectFileLists } | undefined {
  if (ancestors.has(filename) || !fs.statSync(filename, { throwIfNoEntry: false })?.isFile()) {
    return undefined;
  }
  const text = fs.readFileSync(filename, 'utf8');
  const errors: ParseError[] = [];
  const config = parseJsonc(text, errors, { allowTrailingComma: true }) as
    | ({ extends?: unknown } & Partial<Record<ProjectFileKey, unknown>>)
    | undefined;
  if (!config || errors.length) {
    return undefined;
  }
  const directory = path.dirname(filename);
  const lists: ProjectFileLists = {};
  const bases = Array.isArray(config.extends) ? config.extends : [config.extends];
  ancestors.add(filename);
  for (const base of bases) {
    if (typeof base !== 'string' || (!base.startsWith('.') && !path.isAbsolute(base))) {
      continue;
    }
    const target = path.resolve(directory, base);
    const resolved = fs.statSync(target, { throwIfNoEntry: false })?.isFile()
      ? target
      : `${target}.json`;
    Object.assign(lists, readProjectFileLists(resolved, ancestors)?.lists);
  }
  ancestors.delete(filename);
  for (const key of projectFileKeys) {
    const entries = config[key];
    if (Array.isArray(entries) && entries.every((entry) => typeof entry === 'string')) {
      lists[key] = { filename, entries };
    }
  }
  return { text, lists };
}

/** Keep the generated TypeScript config in each project that imports it from Vite config. */
export function includeGeneratedPackConfig(
  viteConfigPath: string,
  generatedPath: string,
  workspaceRoot: string,
): boolean {
  if (!/\.[cm]?ts$/.test(generatedPath)) {
    return false;
  }
  const root = path.resolve(workspaceRoot);
  let changed = false;
  for (
    let directory = path.dirname(path.resolve(viteConfigPath));
    ;
    directory = path.dirname(directory)
  ) {
    for (const filename of findTsconfigFiles(directory)) {
      if (!fs.lstatSync(filename, { throwIfNoEntry: false })?.isFile()) {
        continue;
      }
      const project = readProjectFileLists(filename);
      if (!project) {
        continue;
      }
      const { text, lists } = project;
      const listed = (file: string) => matchesTsconfigPaths(lists.files, file, false);
      const excluded = (file: string) => matchesTsconfigPaths(lists.exclude, file, true);
      const included = (file: string) =>
        matchesTsconfigPaths(lists.include, file, true) && !excluded(file);
      if (listed(generatedPath) || included(generatedPath)) {
        continue;
      }
      const explicitlyListed = listed(viteConfigPath);
      if (!explicitlyListed && !included(viteConfigPath)) {
        continue;
      }
      // A files entry bypasses exclude, as it does for an original explicit file.
      const key = explicitlyListed || excluded(generatedPath) ? 'files' : 'include';
      const relative = path.relative(directory, generatedPath).split(path.sep).join('/');
      const selected = lists[key];
      const inherited = selected && selected.filename !== filename;
      const rebased = inherited
        ? selected.entries.map((entry) =>
            path
              .relative(
                directory,
                path.resolve(path.dirname(selected.filename), entry.replaceAll('\\', '/')),
              )
              .split(path.sep)
              .join('/'),
          )
        : [];
      // Local keys override inherited keys. Carry over the original paths with
      // their original base so the addition preserves the rest of the project.
      const updated = appendMissingJsoncArrayItems(
        text,
        [key],
        inherited ? undefined : selected?.entries,
        [...rebased, relative],
        detectFormattingOptions(text),
      );
      if (updated !== text) {
        fs.writeFileSync(filename, updated);
        changed = true;
      }
    }
    if (directory === root || directory === path.dirname(directory)) {
      return changed;
    }
  }
}

function matchesTsconfigPaths(
  list: ProjectFileList | undefined,
  filename: string,
  allowGlob: boolean,
): boolean {
  const file = path.resolve(filename).split(path.sep).join('/');
  return (
    list?.entries.some((entry) => {
      const specifier = entry.replaceAll('\\', '/');
      const fullPath = path
        .resolve(path.dirname(list.filename), specifier)
        .split(path.sep)
        .join('/');
      if (!allowGlob) {
        return fullPath === file;
      }
      // TypeScript treats a directory with no wildcard as a recursive include.
      const pattern = /[.*?]/.test(path.posix.basename(fullPath)) ? fullPath : `${fullPath}/**/*`;
      return minimatch(file, pattern, {
        dot: true,
        nocase: process.platform === 'win32',
        nobrace: true,
        nocomment: true,
        noext: true,
        nonegate: true,
      });
    }) ?? false
  );
}

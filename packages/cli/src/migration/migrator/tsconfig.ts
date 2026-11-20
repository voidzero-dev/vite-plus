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
      const text = fs.readFileSync(filename, 'utf8');
      const errors: ParseError[] = [];
      const config = parseJsonc(text, errors, { allowTrailingComma: true }) as
        | { files?: unknown; include?: unknown; exclude?: unknown }
        | undefined;
      if (!config || errors.length) {
        continue;
      }

      const listed = (file: string) => matchesTsconfigPaths(directory, config.files, file, false);
      const excluded = (file: string) =>
        matchesTsconfigPaths(directory, config.exclude, file, true);
      const included = (file: string) =>
        matchesTsconfigPaths(directory, config.include, file, true) && !excluded(file);
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
      const updated = appendMissingJsoncArrayItems(
        text,
        [key],
        config[key],
        [relative],
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
  directory: string,
  entries: unknown,
  filename: string,
  allowGlob: boolean,
): boolean {
  const file = path.resolve(filename).split(path.sep).join('/');
  return (
    Array.isArray(entries) &&
    entries.some((entry: unknown) => {
      if (typeof entry !== 'string') {
        return false;
      }
      const specifier = entry.replaceAll('\\', '/');
      const fullPath = path.resolve(directory, specifier).split(path.sep).join('/');
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
    })
  );
}

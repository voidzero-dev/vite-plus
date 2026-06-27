import * as prompts from '@voidzero-dev/vite-plus-prompts';

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
): boolean {
  let changed = false;
  const files = findTsconfigFiles(projectPath);
  for (const filePath of files) {
    if (rewriteTypesInTsconfig(filePath)) {
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

export function hasTsconfigTypesToRewrite(projectPath: string): boolean {
  return findTsconfigFiles(projectPath).some((filePath) => hasTypesToRewriteInTsconfig(filePath));
}

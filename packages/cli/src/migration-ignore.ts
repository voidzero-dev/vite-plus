import path from 'node:path';

import { MigrationIgnoreMatcher } from '../binding/index.js';

export function createMigrationIgnoreMatcher(root: string, patterns: string[]) {
  return new MigrationIgnoreMatcher(path.resolve(root), patterns);
}

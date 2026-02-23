/**
 * Apply Vite+ branding patches to rolldown-vite source after sync.
 *
 * This script modifies user-visible branding strings in the rolldown-vite
 * source to show "VITE+" instead of "VITE". It is called automatically
 * at the end of `sync-remote-deps.ts` and can also be run independently.
 *
 * Changes applied:
 * 1. constants.ts: Add VITE_PLUS_VERSION constant
 * 2. cli.ts: Import VITE_PLUS_VERSION, change banner from 'VITE' to 'VITE+'
 * 3. build.ts: Import VITE_PLUS_VERSION, change build banner and error prefix
 * 4. logger.ts: Change default prefix from '[vite]' to '[vite+]'
 */

import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const ROLLDOWN_VITE_DIR = 'rolldown-vite';
const VITE_NODE_DIR = join(ROLLDOWN_VITE_DIR, 'packages', 'vite', 'src', 'node');

function log(message: string) {
  console.log(`[brand-rolldown-vite] ${message}`);
}

function replaceInFile(filePath: string, search: string, replacement: string): boolean {
  const content = readFileSync(filePath, 'utf-8');
  if (!content.includes(search)) {
    // Already patched or search string not found
    return false;
  }
  const newContent = content.replace(search, replacement);
  writeFileSync(filePath, newContent, 'utf-8');
  return true;
}

export function brandRolldownVite(rootDir: string = process.cwd()) {
  log('Applying Vite+ branding patches...');

  const nodeDir = join(rootDir, VITE_NODE_DIR);

  // 1. constants.ts: Add VITE_PLUS_VERSION constant after VERSION
  const constantsFile = join(nodeDir, 'constants.ts');
  if (
    replaceInFile(
      constantsFile,
      'export const VERSION = version as string',
      'export const VERSION = version as string\n\nexport const VITE_PLUS_VERSION = process.env.VITE_PLUS_VERSION || VERSION',
    )
  ) {
    log('  ✓ constants.ts: Added VITE_PLUS_VERSION');
  } else {
    log('  - constants.ts: Already patched or unchanged');
  }

  // 2. cli.ts: Import VITE_PLUS_VERSION and change dev banner
  const cliFile = join(nodeDir, 'cli.ts');
  let cliPatched = false;

  if (
    replaceInFile(
      cliFile,
      "import { VERSION } from './constants'",
      "import { VERSION, VITE_PLUS_VERSION } from './constants'",
    )
  ) {
    cliPatched = true;
  }
  if (
    replaceInFile(
      cliFile,
      "`${colors.bold('VITE')} v${VERSION}`",
      "`${colors.bold('VITE+')} v${VITE_PLUS_VERSION}`",
    )
  ) {
    cliPatched = true;
  }
  log(
    cliPatched
      ? '  ✓ cli.ts: Updated imports and banner'
      : '  - cli.ts: Already patched or unchanged',
  );

  // 3. build.ts: Import VITE_PLUS_VERSION, change build banner and error prefix
  const buildFile = join(nodeDir, 'build.ts');
  let buildPatched = false;

  if (
    replaceInFile(
      buildFile,
      "  ROLLUP_HOOKS,\n  VERSION,\n} from './constants'",
      "  ROLLUP_HOOKS,\n  VERSION,\n  VITE_PLUS_VERSION,\n} from './constants'",
    )
  ) {
    buildPatched = true;
  }
  if (replaceInFile(buildFile, '`vite v${VERSION} ', '`vite+ v${VITE_PLUS_VERSION} ')) {
    buildPatched = true;
  }
  if (replaceInFile(buildFile, '`[vite]: Rolldown failed', '`[vite+]: Rolldown failed')) {
    buildPatched = true;
  }
  log(
    buildPatched
      ? '  ✓ build.ts: Updated imports, banner, and error prefix'
      : '  - build.ts: Already patched or unchanged',
  );

  // 4. logger.ts: Change default prefix
  const loggerFile = join(nodeDir, 'logger.ts');
  if (replaceInFile(loggerFile, "prefix = '[vite]'", "prefix = '[vite+]'")) {
    log("  ✓ logger.ts: Changed prefix to '[vite+]'");
  } else {
    log('  - logger.ts: Already patched or unchanged');
  }

  log('Done!');
}

/**
 * Apply Vite+ branding patches to rolldown-vite source after sync.
 *
 * This script modifies user-visible branding strings in the rolldown-vite
 * source to show "VITE+" instead of "VITE". It is called automatically
 * at the end of `sync-remote-deps.ts` and can also be run independently.
 *
 * Changes applied:
 * 1. constants.ts: Add VITE_PLUS_VERSION constant
 * 2. cli.ts: Import VITE_PLUS_VERSION, change CLI name, version, and banner
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

/**
 * Replace a string in a file.
 * Returns 'patched' if the replacement was applied, 'already' if the replacement
 * text is already present, or throws an error if neither the search nor the
 * replacement text is found (upstream code changed).
 */
function replaceInFile(
  filePath: string,
  search: string,
  replacement: string,
): 'patched' | 'already' {
  const content = readFileSync(filePath, 'utf-8');
  // Check replacement first: the search string may be a substring of the replacement
  // (e.g. constants.ts where the replacement appends lines after the search string).
  if (content.includes(replacement)) {
    return 'already';
  }
  if (content.includes(search)) {
    const newContent = content.replace(search, replacement);
    writeFileSync(filePath, newContent, 'utf-8');
    return 'patched';
  }
  throw new Error(
    `[brand-rolldown-vite] Patch failed in ${filePath}:\n` +
      `  Could not find search string: ${JSON.stringify(search)}\n` +
      `  The upstream code may have changed. Please update the search string in brand-rolldown-vite.ts.`,
  );
}

function logPatch(file: string, desc: string, result: 'patched' | 'already') {
  if (result === 'patched') {
    log(`  ✓ ${file}: ${desc}`);
  } else {
    log(`  - ${file}: Already patched`);
  }
}

export function brandRolldownVite(rootDir: string = process.cwd()) {
  log('Applying Vite+ branding patches...');

  const nodeDir = join(rootDir, VITE_NODE_DIR);

  // 1. constants.ts: Add VITE_PLUS_VERSION constant after VERSION
  const constantsFile = join(nodeDir, 'constants.ts');
  logPatch(
    'constants.ts',
    'Added VITE_PLUS_VERSION',
    replaceInFile(
      constantsFile,
      'export const VERSION = version as string',
      'export const VERSION = version as string\n\nexport const VITE_PLUS_VERSION: string = process.env.VITE_PLUS_VERSION || VERSION',
    ),
  );

  // 2. cli.ts: Import VITE_PLUS_VERSION, change CLI name, version, and dev banner
  const cliFile = join(nodeDir, 'cli.ts');
  const cliResults = [
    replaceInFile(
      cliFile,
      "import { VERSION } from './constants'",
      "import { VERSION, VITE_PLUS_VERSION } from './constants'",
    ),
    replaceInFile(cliFile, "cac('vite')", "cac('vp')"),
    replaceInFile(cliFile, 'cli.version(VERSION)', 'cli.version(VITE_PLUS_VERSION)'),
    replaceInFile(
      cliFile,
      "`${colors.bold('VITE')} v${VERSION}`",
      "`${colors.bold('VITE+')} v${VITE_PLUS_VERSION}`",
    ),
  ];
  logPatch(
    'cli.ts',
    'Updated imports, CLI name, version, and banner',
    cliResults.includes('patched') ? 'patched' : 'already',
  );

  // 3. build.ts: Import VITE_PLUS_VERSION, change build banner and error prefix
  const buildFile = join(nodeDir, 'build.ts');
  const buildResults = [
    replaceInFile(
      buildFile,
      "  ROLLUP_HOOKS,\n  VERSION,\n} from './constants'",
      "  ROLLUP_HOOKS,\n  VERSION,\n  VITE_PLUS_VERSION,\n} from './constants'",
    ),
    replaceInFile(buildFile, '`vite v${VERSION} ', '`vite+ v${VITE_PLUS_VERSION} '),
    replaceInFile(buildFile, '`[vite]: Rolldown failed', '`[vite+]: Rolldown failed'),
  ];
  logPatch(
    'build.ts',
    'Updated imports, banner, and error prefix',
    buildResults.includes('patched') ? 'patched' : 'already',
  );

  // 4. logger.ts: Change default prefix
  const loggerFile = join(nodeDir, 'logger.ts');
  logPatch(
    'logger.ts',
    "Changed prefix to '[vite+]'",
    replaceInFile(loggerFile, "prefix = '[vite]'", "prefix = '[vite+]'"),
  );

  log('Done!');
}

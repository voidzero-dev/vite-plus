/**
 * Apply Vite+ branding patches to rolldown-vite source after sync.
 *
 * This script modifies user-visible branding strings in the rolldown-vite
 * source to show "VITE+" instead of "VITE". It is called automatically
 * at the end of `sync-remote-deps.ts` and can also be run independently.
 *
 * Changes applied:
 * 1. constants.ts: Add VITE_PLUS_VERSION constant
 * 2. cli.ts: Import VITE_PLUS_VERSION, change CLI name/version, and make banner blue
 * 3. build.ts: Remove startup build banner and change error prefix
 * 4. logger.ts: Change default prefix from '[vite]' to '[vite+]'
 * 5. plugins/reporter.ts: Suppress redundant "vite v<version>" native reporter line
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

function removeAnyInFile(
  filePath: string,
  searches: Array<string | RegExp>,
): 'patched' | 'already' {
  const content = readFileSync(filePath, 'utf-8');
  for (const search of searches) {
    if (typeof search === 'string') {
      if (content.includes(search)) {
        const newContent = content.replace(search, '');
        writeFileSync(filePath, newContent, 'utf-8');
        return 'patched';
      }
      continue;
    }

    if (search.test(content)) {
      const newContent = content.replace(search, '');
      writeFileSync(filePath, newContent, 'utf-8');
      return 'patched';
    }
  }
  return 'already';
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
    replaceInFile(
      cliFile,
      "colors.green(\n            `${colors.bold('VITE+')} v${VITE_PLUS_VERSION}`,\n          )",
      "colors.blue(\n            `${colors.bold('VITE+')} v${VITE_PLUS_VERSION}`,\n          )",
    ),
  ];
  logPatch(
    'cli.ts',
    'Updated imports, CLI name, version, and banner',
    cliResults.includes('patched') ? 'patched' : 'already',
  );

  // 3. build.ts: Remove startup build banner and update error prefix
  const buildFile = join(nodeDir, 'build.ts');
  const buildResults = [
    removeAnyInFile(buildFile, [
      / {4}logger\.info\(\n {6}colors\.[a-zA-Z]+\(\n {8}`vite v\$\{VERSION\} \$\{colors\.green\(\n {10}`building \$\{environment\.name\} environment for \$\{environment\.config\.mode\}\.\.\.`,\n {8}\)\}`,\n {6}\),\n {4}\)\n/,
      / {4}logger\.info\(\n {6}colors\.[a-zA-Z]+\(\n {8}`vite\+ v\$\{VITE_PLUS_VERSION\} \$\{colors\.green\(\n {10}`building \$\{environment\.name\} environment for \$\{environment\.config\.mode\}\.\.\.`,\n {8}\)\}`,\n {6}\),\n {4}\)\n/,
    ]),
    replaceInFile(buildFile, '`[vite]: Rolldown failed', '`[vite+]: Rolldown failed'),
  ];
  logPatch(
    'build.ts',
    'Removed startup banner and updated error prefix',
    buildResults.includes('patched') ? 'patched' : 'already',
  );

  // 4. logger.ts: Change default prefix
  const loggerFile = join(nodeDir, 'logger.ts');
  logPatch(
    'logger.ts',
    "Changed prefix to '[vite+]'",
    replaceInFile(loggerFile, "prefix = '[vite]'", "prefix = '[vite+]'"),
  );

  // 5. reporter.ts: Suppress redundant version-only line from native reporter
  const reporterFile = join(nodeDir, 'plugins', 'reporter.ts');
  const reporterResults = [
    replaceInFile(
      reporterFile,
      'const COMPRESSIBLE_ASSETS_RE = /\\.(?:html|json|svg|txt|xml|xhtml|wasm)$/',
      'const COMPRESSIBLE_ASSETS_RE = /\\.(?:html|json|svg|txt|xml|xhtml|wasm)$/\nconst VITE_VERSION_ONLY_LINE_RE = /^vite v\\S+$/',
    ),
    replaceInFile(
      reporterFile,
      '        logInfo: shouldLogInfo ? (msg) => env.logger.info(msg) : undefined,',
      '        logInfo: shouldLogInfo\n          ? (msg) => {\n              // Keep transformed/chunk/gzip logs but suppress redundant version-only line.\n              if (VITE_VERSION_ONLY_LINE_RE.test(msg.trim())) {\n                return\n              }\n              env.logger.info(msg)\n            }\n          : undefined,',
    ),
  ];
  logPatch(
    'plugins/reporter.ts',
    'Suppressed redundant version-only native reporter line',
    reporterResults.includes('patched') ? 'patched' : 'already',
  );

  log('Done!');
}

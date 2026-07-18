import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';

import { afterEach, describe, expect, test } from 'vitest';

import { brandVite } from '../brand-vite.ts';

const temporaryRoots: string[] = [];

afterEach(() => {
  for (const root of temporaryRoots.splice(0)) {
    rmSync(root, { force: true, recursive: true });
  }
});

function writeViteNodeFile(root: string, relativePath: string, content: string) {
  const filePath = join(root, 'vite', 'packages', 'vite', 'src', 'node', relativePath);
  mkdirSync(dirname(filePath), { recursive: true });
  writeFileSync(filePath, content);
  return filePath;
}

describe('brandVite()', () => {
  test('rebrands the pristine upstream build banner without replacing VERSION metadata', () => {
    const root = mkdtempSync(join(tmpdir(), 'brand-vite-'));
    temporaryRoots.push(root);

    // brandVite patches all five sources in one pass; the other files exercise that public workflow.
    writeViteNodeFile(root, 'constants.ts', 'export const VERSION = version as string\n');
    writeViteNodeFile(
      root,
      'cli.ts',
      `import { VERSION } from './constants'
cac('vite')
cli.version(VERSION)
colors.green(
            \`\${colors.bold('VITE')} v\${VERSION}\`,
          )
`,
    );
    const buildFile = writeViteNodeFile(
      root,
      'build.ts',
      `import {
  ROLLUP_HOOKS,
  VERSION,
} from './constants'

logger.info(
    colors.cyan(
      \`vite v\${VERSION} \${colors.green(
        \`building \${environment.name} environment for \${environment.config.mode}...\`,
      )}\`,
    ),
  )

throw new Error(\`[vite]: Rolldown failed to resolve import\`)

context.meta.viteVersion ??= VERSION
`,
    );
    writeViteNodeFile(root, 'logger.ts', "prefix = '[vite]'\n");
    writeViteNodeFile(
      root,
      'plugins/reporter.ts',
      `import path from 'node:path'
      logInfo: shouldLogInfo ? (msg) => env.logger.info(msg) : undefined,
`,
    );

    brandVite(root);

    const build = readFileSync(buildFile, 'utf-8');
    expect(build).toContain("  VERSION,\n  VITE_PLUS_VERSION,\n} from './constants'");
    expect(build).toContain('`vite+ v${VITE_PLUS_VERSION} ${colors.green(');
    expect(build).toContain('context.meta.viteVersion ??= VERSION');
    expect(build).not.toContain('`vite v${VERSION} ${colors.green(');
  });
});

import { writeSync } from 'node:fs';

import type { OxlintConfig } from 'oxlint';

import { PREFER_VITE_PLUS_IMPORTS_RULE } from '../../oxlint-plugin-config.ts';
import { createMigrationReport } from '../report.ts';
import { checkManualChunksCompat } from './manual-chunks.ts';
import { LINT_CONFIG_RESULT_PREFIX, ROLLDOWN_COMPAT_RESULT_PREFIX } from './protocol.ts';

// Plugin objects and unrelated rule options need not be JSON serializable.
// Return only the settings used by migration's import rewrite, including extends.
function importRuleMetadata(lint: OxlintConfig): OxlintConfig {
  return {
    rules: { [PREFER_VITE_PLUS_IMPORTS_RULE]: lint.rules?.[PREFER_VITE_PLUS_IMPORTS_RULE] },
    extends: lint.extends?.map(importRuleMetadata),
    overrides: lint.overrides?.map((override) => ({
      files: override.files,
      excludeFiles: override.excludeFiles,
      rules: { [PREFER_VITE_PLUS_IMPORTS_RULE]: override.rules?.[PREFER_VITE_PLUS_IMPORTS_RULE] },
    })),
  };
}

function writeWorkerResult(
  prefix: string,
  result: { lint: OxlintConfig | null } | { warnings: Array<string> },
): void {
  writeSync(process.stdout.fd, `${prefix}${JSON.stringify(result)}\n`);
}

async function main(): Promise<void> {
  const rootDir = process.argv[2];
  if (!rootDir) {
    return;
  }

  const { resolveConfig } = await import('../../index.js');
  const { withConfigMetadataResolution } = await import('../../define-config.js');
  const readLint = process.argv[3] === 'lint';
  // Bundling supports CommonJS lint configs. The runner avoids bundler warnings
  // during compatibility checks. Neither probe needs lazy plugin factories.
  const config = await withConfigMetadataResolution(() =>
    resolveConfig(
      { root: rootDir, logLevel: 'silent', configLoader: readLint ? 'bundle' : 'runner' },
      'build',
    ),
  );
  if (readLint) {
    const lint = config.lint ? importRuleMetadata(config.lint) : null;
    writeWorkerResult(LINT_CONFIG_RESULT_PREFIX, { lint });
    return;
  }

  const report = createMigrationReport();
  checkManualChunksCompat(config.build?.rollupOptions?.output, report);
  writeWorkerResult(ROLLDOWN_COMPAT_RESULT_PREFIX, { warnings: report.warnings });
}

// Exit even if project plugins left active handles. On failure, the parent
// detects the missing result and applies the policy for the requested probe.
try {
  await main();
} finally {
  process.exit(0);
}

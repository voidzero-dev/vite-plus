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

async function main(): Promise<void> {
  const rootDir = process.argv[2];
  if (!rootDir) {
    return;
  }

  try {
    const { resolveConfig } = await import('../../index.js');
    const { withConfigMetadataResolution } = await import('../../define-config.js');
    const readLint = process.argv[3] === 'lint';
    // Keep the normal bundling loader for lint metadata, including CommonJS
    // configs. Compatibility probes use 'runner' to avoid bundler warnings.
    // Reads the config only for migration metadata, so skip the user's
    // plugin factory (lazyPlugins) while it resolves, otherwise a blocking or
    // slow factory would hang this worker and a throwing factory would drop the
    // warning silently.
    const config = await withConfigMetadataResolution(() =>
      resolveConfig(
        { root: rootDir, logLevel: 'silent', configLoader: readLint ? 'bundle' : 'runner' },
        'build',
      ),
    );
    if (readLint) {
      writeSync(
        process.stdout.fd,
        `${LINT_CONFIG_RESULT_PREFIX}${JSON.stringify({ lint: config.lint ? importRuleMetadata(config.lint) : null })}\n`,
      );
      return;
    }
    const report = createMigrationReport();
    checkManualChunksCompat(config.build?.rollupOptions?.output, report);
    writeSync(
      process.stdout.fd,
      `${ROLLDOWN_COMPAT_RESULT_PREFIX}${JSON.stringify({ warnings: report.warnings })}\n`,
    );
  } catch {
    // No result means config loading failed. The parent handles it for each mode.
  }
}

// Config plugins may leave active handles behind. Once the result has been
// written synchronously, terminate this disposable worker without waiting for
// project-owned cleanup.
main().then(
  () => process.exit(0),
  () => process.exit(0),
);

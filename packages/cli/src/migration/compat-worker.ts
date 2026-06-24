import { writeSync } from 'node:fs';

import { ROLLDOWN_COMPAT_RESULT_PREFIX } from './compat-protocol.ts';
import { checkManualChunksCompat } from './compat.ts';
import { createMigrationReport } from './report.ts';

async function main(): Promise<void> {
  const rootDir = process.argv[2];
  if (!rootDir) {
    return;
  }

  try {
    const { resolveConfig } = await import('../index.js');
    // Use 'runner' configLoader to avoid Rolldown bundling the config file,
    // which prints UNRESOLVED_IMPORT warnings that cannot be suppressed via logLevel.
    const config = await resolveConfig(
      { root: rootDir, logLevel: 'silent', configLoader: 'runner' },
      'build',
    );
    const report = createMigrationReport();
    checkManualChunksCompat(config.build?.rollupOptions?.output, report);
    writeSync(
      process.stdout.fd,
      `${ROLLDOWN_COMPAT_RESULT_PREFIX}${JSON.stringify({ warnings: report.warnings })}\n`,
    );
  } catch {
    // Config resolution may fail — skip compatibility checking silently.
  }
}

// Config plugins may leave active handles behind. Once the result has been
// written synchronously, terminate this disposable worker without waiting for
// project-owned cleanup.
main().then(
  () => process.exit(0),
  () => process.exit(0),
);

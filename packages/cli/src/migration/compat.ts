import { addMigrationWarning, type MigrationReport } from './report.js';

/**
 * Check for Rolldown-incompatible manualChunks config patterns.
 */
export function checkManualChunksCompat(output: unknown, report: MigrationReport): void {
  const outputs = Array.isArray(output) ? output : output ? [output] : [];
  for (const out of outputs) {
    if (out.manualChunks != null && typeof out.manualChunks !== 'function') {
      addMigrationWarning(
        report,
        'Object-form `build.rollupOptions.output.manualChunks` is not supported by Rolldown. ' +
          'Convert it to function form or use `build.rolldownOptions.output.codeSplitting`. ' +
          'See: https://rolldown.rs/options/output#manualchunks and https://rolldown.rs/in-depth/manual-code-splitting',
      );
      break;
    }
  }
}

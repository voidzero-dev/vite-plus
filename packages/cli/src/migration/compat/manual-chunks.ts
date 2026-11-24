import type { MigrationReport } from '../report.ts';
import { addMigrationWarning } from '../report.ts';

/**
 * Check for Rolldown-incompatible manualChunks config patterns.
 */
export function checkManualChunksCompat(output: unknown, report: MigrationReport): void {
  const outputs: Array<unknown> = Array.isArray(output) ? output : output ? [output] : [];
  for (const out of outputs) {
    if (
      out !== null &&
      typeof out === 'object' &&
      'manualChunks' in out &&
      out.manualChunks !== undefined &&
      out.manualChunks !== null &&
      typeof out.manualChunks !== 'function'
    ) {
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

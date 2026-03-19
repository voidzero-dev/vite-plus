import { describe, expect, it } from 'vitest';

import { checkManualChunksCompat } from '../compat.js';
import { createMigrationReport } from '../report.js';

describe('checkManualChunksCompat', () => {
  it('should warn when manualChunks is an object', () => {
    const report = createMigrationReport();
    checkManualChunksCompat({ manualChunks: { react: ['react', 'react-dom'] } }, report);
    expect(report.warnings).toHaveLength(1);
    expect(report.warnings[0]).toContain('Object-form');
    expect(report.warnings[0]).toContain('codeSplitting');
  });

  it('should not warn when manualChunks is a function', () => {
    const report = createMigrationReport();
    checkManualChunksCompat({ manualChunks: () => undefined }, report);
    expect(report.warnings).toHaveLength(0);
  });

  it('should not warn when manualChunks is not set', () => {
    const report = createMigrationReport();
    checkManualChunksCompat({}, report);
    expect(report.warnings).toHaveLength(0);
  });

  it('should not warn when output is undefined', () => {
    const report = createMigrationReport();
    checkManualChunksCompat(undefined, report);
    expect(report.warnings).toHaveLength(0);
  });

  it('should handle array of outputs', () => {
    const report = createMigrationReport();
    checkManualChunksCompat(
      [{ manualChunks: () => undefined }, { manualChunks: { vendor: ['lodash'] } }],
      report,
    );
    expect(report.warnings).toHaveLength(1);
  });

  it('should only add one warning for multiple object-form outputs', () => {
    const report = createMigrationReport();
    checkManualChunksCompat(
      [{ manualChunks: { react: ['react'] } }, { manualChunks: { lodash: ['lodash'] } }],
      report,
    );
    expect(report.warnings).toHaveLength(1);
  });
});

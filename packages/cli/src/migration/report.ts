export interface MigrationReport {
  createdViteConfigCount: number;
  mergedConfigCount: number;
  mergedStagedConfigCount: number;
  inlinedLintStagedConfigCount: number;
  removedConfigCount: number;
  tsdownImportCount: number;
  rewrittenImportFileCount: number;
  rewrittenImportErrors: Array<{ path: string; message: string }>;
  eslintMigrated: boolean;
  prettierMigrated: boolean;
  nodeVersionFileMigrated: boolean;
  gitHooksConfigured: boolean;
  warnings: string[];
  manualSteps: string[];
}

export function createMigrationReport(): MigrationReport {
  return {
    createdViteConfigCount: 0,
    mergedConfigCount: 0,
    mergedStagedConfigCount: 0,
    inlinedLintStagedConfigCount: 0,
    removedConfigCount: 0,
    tsdownImportCount: 0,
    rewrittenImportFileCount: 0,
    rewrittenImportErrors: [],
    eslintMigrated: false,
    prettierMigrated: false,
    nodeVersionFileMigrated: false,
    gitHooksConfigured: false,
    warnings: [],
    manualSteps: [],
  };
}

export function addMigrationWarning(report: MigrationReport | undefined, warning: string) {
  if (!report || report.warnings.includes(warning)) {
    return;
  }
  report.warnings.push(warning);
}

export function addManualStep(report: MigrationReport | undefined, step: string) {
  if (!report || report.manualSteps.includes(step)) {
    return;
  }
  report.manualSteps.push(step);
}

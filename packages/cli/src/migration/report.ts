/**
 * A single toolchain dependency version change captured before the
 * existing-Vite+ reconcile mutates the manifest. `from` is the pre-migration
 * version (undefined when the package is freshly added); `to` is the version
 * Vite+ migrates it to. Rendered as an aligned table in the migrate summary.
 */
export interface DependencyVersionChange {
  name: string;
  from?: string;
  to: string;
}

export interface MigrationReport {
  createdViteConfigCount: number;
  mergedConfigCount: number;
  mergedStagedConfigCount: number;
  inlinedLintStagedConfigCount: number;
  removedConfigCount: number;
  tsdownImportCount: number;
  wrappedPluginConfigCount: number;
  rewrittenImportFileCount: number;
  preservedUpstreamVitestImportFileCount: number;
  rewrittenImportErrors: Array<{ path: string; message: string }>;
  eslintMigrated: boolean;
  prettierMigrated: boolean;
  nodeVersionFileMigrated: boolean;
  gitHooksConfigured: boolean;
  frameworkShimAdded: boolean;
  packageManagerBootstrapConfigured: boolean;
  dependencyUpgrades: DependencyVersionChange[];
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
    wrappedPluginConfigCount: 0,
    rewrittenImportFileCount: 0,
    preservedUpstreamVitestImportFileCount: 0,
    rewrittenImportErrors: [],
    eslintMigrated: false,
    prettierMigrated: false,
    nodeVersionFileMigrated: false,
    gitHooksConfigured: false,
    frameworkShimAdded: false,
    packageManagerBootstrapConfigured: false,
    dependencyUpgrades: [],
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

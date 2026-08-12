import fs from 'node:fs';
import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { PackageManager, type WorkspacePackage } from '../../types/index.ts';
import { runCommandSilently } from '../../utils/command.ts';
import { editJsonFile, readJsonFile } from '../../utils/json.ts';
import { displayRelative } from '../../utils/path.ts';
import { cancelAndExit } from '../../utils/prompts.ts';
import { getSilentSpinner, getSpinner } from '../../utils/spinner.ts';
import { detectConfigs, TSUP_CONFIG_FILES, TSUP_PACKAGE_JSON_CONFIG } from '../detector.ts';
import { type MigrationReport } from '../report.ts';

export function detectTsupProject(
  projectPath: string,
  packages?: WorkspacePackage[],
): {
  hasDependency: boolean;
  configFile?: string;
} {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return { hasDependency: false };
  }
  const pkg = readJsonFile(packageJsonPath) as {
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  };
  let hasDependency = !!(pkg.devDependencies?.tsup || pkg.dependencies?.tsup);
  const configs = detectConfigs(projectPath);
  const configFile = configs.tsupConfig;

  // If root doesn't have tsup dependency, check workspace packages
  if (!hasDependency && packages) {
    for (const wp of packages) {
      const pkgJsonPath = path.join(projectPath, wp.path, 'package.json');
      if (!fs.existsSync(pkgJsonPath)) {
        continue;
      }
      const wpPkg = readJsonFile(pkgJsonPath) as {
        devDependencies?: Record<string, string>;
        dependencies?: Record<string, string>;
      };
      if (wpPkg.devDependencies?.tsup || wpPkg.dependencies?.tsup) {
        hasDependency = true;
        break;
      }
    }
  }

  return { hasDependency, configFile };
}

/**
 * Run `vp dlx tsdown-migrate` in `cwd` with graceful error handling.
 * Returns true on success, false on failure (spawn error or non-zero exit).
 */
async function runTsdownMigrateStep(
  vpBin: string,
  cwd: string,
  spinner: ReturnType<typeof getSpinner>,
  failMessage: string,
  manualHint: string,
  packageManager: PackageManager,
): Promise<boolean> {
  try {
    const result = await runCommandSilently({
      command: vpBin,
      args: ['dlx', 'tsdown-migrate@rc', '--yes', `--package-manager ${packageManager}`], // remove pin to rc tag once it graduates to main version
      cwd,
      envs: process.env,
    });
    if (result.exitCode !== 0) {
      spinner.stop(failMessage);
      const stderr = result.stderr.toString().trim();
      if (stderr) {
        prompts.log.warn(`⚠ ${stderr}`);
      }
      prompts.log.info(manualHint);
      return false;
    }
    return true;
  } catch {
    spinner.stop(failMessage);
    prompts.log.info(manualHint);
    return false;
  }
}

export async function migrateTsupToTsdown(
  projectPath: string,
  interactive: boolean,
  packageManager: PackageManager,
  tsupConfigFile?: string,
  packages?: WorkspacePackage[],
  options?: { silent?: boolean; report?: MigrationReport },
): Promise<boolean> {
  const vpBin = process.env.VP_CLI_BIN ?? 'vp';
  const spinner = options?.silent ? getSilentSpinner() : getSpinner(interactive);

  // A tsup config isn't necessarily workspace-wide the way an ESLint flat
  // config usually is — a monorepo commonly builds each package
  // independently with its own `tsup.config.*`. Run `tsdown-migrate` in
  // every directory that has one (root and/or workspace packages) so each
  // gets its own `tsdown.config.*`, which `mergeTsdownConfigFile` then picks
  // up per-project — mirroring how `tsdown.config.*` itself is merged.
  const targets = [projectPath, ...(packages ?? []).map((p) => path.join(projectPath, p.path))];
  const tsupTargets = targets.filter((target) =>
    target === projectPath ? !!tsupConfigFile : !!detectConfigs(target).tsupConfig,
  );

  if (tsupTargets.length > 0) {
    spinner.start('Migrating tsup config to tsdown...');
    for (const target of tsupTargets) {
      const migrateOk = await runTsdownMigrateStep(
        vpBin,
        target,
        spinner,
        'tsup migration failed',
        `You can run \`vp dlx tsdown-migrate\` manually later in ${displayRelative(target)}`,
        packageManager,
      );
      if (!migrateOk) {
        return false;
      }
    }
    spinner.stop('tsup config migrated to tsdown.config');
  }

  if (options?.report) {
    options.report.tsupMigrated = true;
  }

  // Cleanup runs uniformly across the root and every workspace package —
  // delete tsup config files and remove the `tsup` dependency from
  // package.json. Mirrors the eslint/prettier cleanup pass.
  for (const target of targets) {
    if (!fs.existsSync(path.join(target, 'package.json'))) {
      continue;
    }
    deleteTsupConfigFiles(target, options?.report, options?.silent);
    rewriteTsupPackageJson(path.join(target, 'package.json'));
  }

  return true;
}

function deleteTsupConfigFiles(basePath: string, report?: MigrationReport, silent = false): void {
  const configs = detectConfigs(basePath);
  if (configs.tsupConfig && configs.tsupConfig !== TSUP_PACKAGE_JSON_CONFIG) {
    const configPath = path.join(basePath, configs.tsupConfig);
    if (fs.existsSync(configPath)) {
      fs.unlinkSync(configPath);
      if (report) {
        report.removedConfigCount++;
      }
      if (!silent) {
        prompts.log.success(`✔ Removed ${displayRelative(configPath)}`);
      }
    }
  }
  // Also clean up any stale tsup config files that detectConfigs didn't pick
  // (tsup only uses one config, but users may have leftover files).
  for (const file of TSUP_CONFIG_FILES) {
    if (file === configs.tsupConfig) {
      continue; // already handled above
    }
    const configPath = path.join(basePath, file);
    if (fs.existsSync(configPath)) {
      fs.unlinkSync(configPath);
      if (report) {
        report.removedConfigCount++;
      }
      if (!silent) {
        prompts.log.success(`✔ Removed ${displayRelative(configPath)}`);
      }
    }
  }
  // Remove "tsup" key from package.json if present — `tsdown-migrate`
  // reads it as a config source but never deletes it.
  editJsonFile<{ tsup?: unknown }>(path.join(basePath, 'package.json'), (pkg) => {
    if (pkg.tsup) {
      delete pkg.tsup;
      return pkg;
    }
    return undefined;
  });
}

function rewriteTsupPackageJson(packageJsonPath: string): void {
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }
  editJsonFile<{
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  }>(packageJsonPath, (pkg) => {
    let changed = false;
    // Remove the tsup dependency itself. Scripts (`"build": "tsup"`) are
    // already rewritten to `vp pack` generically by `rewriteScripts` (see
    // `replace-tsup` in rules/vite-tools.yml), and `tsdown` is a managed
    // vite-plus-bundled dependency (see `REMOVE_PACKAGES`), so neither needs
    // handling here.
    for (const field of ['devDependencies', 'dependencies'] as const) {
      if (pkg[field]?.tsup) {
        delete pkg[field].tsup;
        changed = true;
      }
    }
    return changed ? pkg : undefined;
  });
}

export function warnPackageLevelTsup() {
  prompts.log.warn(
    'tsup detected in workspace packages but no root config found. Package-level tsup must be migrated manually.',
  );
}

export async function confirmTsupMigration(interactive: boolean): Promise<boolean> {
  if (interactive) {
    const confirmed = await prompts.confirm({
      message:
        'Migrate tsup config to tsdown using tsdown-migrate?\n  ' +
        styleText(
          'gray',
          "tsdown is Vite+'s built-in bundler (exposed via `vp pack`) — a mostly drop-in tsup replacement powered by Rolldown. tsdown-migrate converts your existing config automatically.",
        ),
      initialValue: true,
    });
    if (prompts.isCancel(confirmed)) {
      cancelAndExit();
    }
    return confirmed;
  }
  prompts.log.info('tsup configuration detected. Auto-migrating to tsdown...');
  return true;
}

export async function promptTsupMigration(
  projectPath: string,
  interactive: boolean,
  packageManager: PackageManager,
  packages?: WorkspacePackage[],
): Promise<boolean> {
  const tsupProject = detectTsupProject(projectPath, packages);
  if (!tsupProject.hasDependency) {
    return false;
  }
  if (!tsupProject.configFile) {
    // Packages have tsup but no root config → warn and skip
    warnPackageLevelTsup();
    return false;
  }
  const confirmed = await confirmTsupMigration(interactive);
  if (!confirmed) {
    return false;
  }
  const ok = await migrateTsupToTsdown(projectPath, interactive, packageManager, tsupProject.configFile, packages);
  if (!ok) {
    cancelAndExit('tsup migration failed.', 1);
  }
  return true;
}

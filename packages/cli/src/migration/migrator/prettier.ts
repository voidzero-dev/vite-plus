import fs from 'node:fs';
import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { rewritePrettier } from '../../../binding/index.js';
import { type WorkspacePackage } from '../../types/index.ts';
import { runCommandSilently } from '../../utils/command.ts';
import { editJsonFile, readJsonFile } from '../../utils/json.ts';
import { displayRelative } from '../../utils/path.ts';
import { cancelAndExit } from '../../utils/prompts.ts';
import { getSpinner } from '../../utils/spinner.ts';
import { PRETTIER_CONFIG_FILES, PRETTIER_PACKAGE_JSON_CONFIG, detectConfigs } from '../detector.ts';
import { rewriteToolLintStagedConfigFiles } from '../migrator.ts';
import { type MigrationReport } from '../report.ts';
import { warnMigration } from './shared.ts';

export function detectPrettierProject(
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
  let hasDependency = !!(pkg.devDependencies?.prettier || pkg.dependencies?.prettier);
  const configs = detectConfigs(projectPath);
  const configFile = configs.prettierConfig;

  // If root doesn't have prettier dependency, check workspace packages
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
      if (wpPkg.devDependencies?.prettier || wpPkg.dependencies?.prettier) {
        hasDependency = true;
        break;
      }
    }
  }

  return { hasDependency, configFile };
}

/**
 * Run `vp fmt --migrate=prettier` step with graceful error handling.
 * Returns true on success, false on failure.
 */
async function runPrettierMigrateStep(
  vpBin: string,
  cwd: string,
  spinner: ReturnType<typeof getSpinner>,
  failMessage: string,
  manualHint: string,
): Promise<boolean> {
  try {
    const result = await runCommandSilently({
      command: vpBin,
      args: ['fmt', '--migrate=prettier'],
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

export async function migratePrettierToOxfmt(
  projectPath: string,
  interactive: boolean,
  prettierConfigFile?: string,
  packages?: WorkspacePackage[],
  options?: { silent?: boolean; report?: MigrationReport },
): Promise<boolean> {
  const vpBin = process.env.VP_CLI_BIN ?? 'vp';
  const spinner = options?.silent
    ? {
        start: () => {},
        stop: () => {},
        pause: () => {},
        resume: () => {},
        cancel: () => {},
        error: () => {},
        clear: () => {},
        message: () => {},
        isCancelled: false,
      }
    : getSpinner(interactive);

  // Step 1: Generate .oxfmtrc.json from Prettier config
  if (prettierConfigFile) {
    let tempPrettierConfig: string | undefined;

    // If config is in package.json, extract it to a temporary .prettierrc.json
    // so that `vp fmt --migrate=prettier` can read it
    if (prettierConfigFile === PRETTIER_PACKAGE_JSON_CONFIG) {
      const packageJsonPath = path.join(projectPath, 'package.json');
      const pkg = readJsonFile(packageJsonPath) as { prettier?: unknown };
      if (pkg.prettier) {
        tempPrettierConfig = path.join(projectPath, '.prettierrc.json');
        fs.writeFileSync(tempPrettierConfig, JSON.stringify(pkg.prettier, null, 2));
      } else {
        // Config disappeared between detection and migration — nothing to migrate
        return true;
      }
    }

    try {
      spinner.start('Migrating Prettier config to Oxfmt...');
      const migrateOk = await runPrettierMigrateStep(
        vpBin,
        projectPath,
        spinner,
        'Prettier migration failed',
        'You can run `vp fmt --migrate=prettier` manually later',
      );
      if (!migrateOk) {
        return false;
      }
      spinner.stop('Prettier config migrated to .oxfmtrc.json');
    } finally {
      if (tempPrettierConfig) {
        try {
          fs.unlinkSync(tempPrettierConfig);
        } catch {}
      }
    }
  }

  if (options?.report) {
    options.report.prettierMigrated = true;
  }

  // Step 2: Delete all prettier config files at root
  deletePrettierConfigFiles(projectPath, options?.report, options?.silent);

  // Step 3: Remove prettier dependency and rewrite prettier scripts (root)
  rewritePrettierPackageJson(path.join(projectPath, 'package.json'));

  // Step 3b: Rewrite prettier scripts in workspace packages
  if (packages) {
    for (const pkg of packages) {
      rewritePrettierPackageJson(path.join(projectPath, pkg.path, 'package.json'));
    }
  }

  // Step 4: Rewrite prettier references in lint-staged config files
  rewritePrettierLintStagedConfigFiles(projectPath, options?.report);

  // Step 5: Warn about .prettierignore if it exists
  const prettierIgnorePath = path.join(projectPath, '.prettierignore');
  if (fs.existsSync(prettierIgnorePath)) {
    warnMigration(
      `${displayRelative(prettierIgnorePath)} found — Oxfmt supports .prettierignore, but using the \`ignorePatterns\` option is recommended.`,
      options?.report,
    );
  }

  return true;
}

function deletePrettierConfigFiles(
  basePath: string,
  report?: MigrationReport,
  silent = false,
): void {
  // Delete detected prettier config file (like deleteEslintConfigFiles uses detectConfigs)
  const configs = detectConfigs(basePath);
  if (configs.prettierConfig && configs.prettierConfig !== PRETTIER_PACKAGE_JSON_CONFIG) {
    const configPath = path.join(basePath, configs.prettierConfig);
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
  // Also clean up any stale prettier config files that detectConfigs didn't pick
  // (prettier only uses one config, but users may have leftover files)
  for (const file of PRETTIER_CONFIG_FILES) {
    if (file === configs.prettierConfig) {
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
  // Remove "prettier" key from package.json if present
  editJsonFile<{ prettier?: unknown }>(path.join(basePath, 'package.json'), (pkg) => {
    if (pkg.prettier) {
      delete pkg.prettier;
      return pkg;
    }
    return undefined;
  });
}

function rewritePrettierPackageJson(packageJsonPath: string): void {
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }
  editJsonFile<{
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
    scripts?: Record<string, string>;
    'lint-staged'?: Record<string, string | string[]>;
  }>(packageJsonPath, (pkg) => {
    let changed = false;
    // Remove prettier and prettier-plugin-* dependencies
    if (pkg.devDependencies) {
      for (const dep of Object.keys(pkg.devDependencies)) {
        if (dep === 'prettier' || dep.startsWith('prettier-plugin-')) {
          delete pkg.devDependencies[dep];
          changed = true;
        }
      }
    }
    if (pkg.dependencies) {
      for (const dep of Object.keys(pkg.dependencies)) {
        if (dep === 'prettier' || dep.startsWith('prettier-plugin-')) {
          delete pkg.dependencies[dep];
          changed = true;
        }
      }
    }
    if (pkg.scripts) {
      const updated = rewritePrettier(JSON.stringify(pkg.scripts));
      if (updated) {
        pkg.scripts = JSON.parse(updated);
        changed = true;
      }
    }
    if (pkg['lint-staged']) {
      const updated = rewritePrettier(JSON.stringify(pkg['lint-staged']));
      if (updated) {
        pkg['lint-staged'] = JSON.parse(updated);
        changed = true;
      }
    }
    return changed ? pkg : undefined;
  });
}

function rewritePrettierLintStagedConfigFiles(projectPath: string, report?: MigrationReport): void {
  rewriteToolLintStagedConfigFiles(projectPath, rewritePrettier, 'prettier', report);
}

export function warnPackageLevelPrettier() {
  prompts.log.warn(
    'Prettier detected in workspace packages but no root config found. Package-level Prettier must be migrated manually.',
  );
}

export async function confirmPrettierMigration(interactive: boolean): Promise<boolean> {
  if (interactive) {
    const confirmed = await prompts.confirm({
      message:
        'Migrate Prettier to Oxfmt?\n  ' +
        styleText(
          'gray',
          "Oxfmt is Vite+'s built-in formatter that replaces Prettier with faster performance. Your configuration will be converted automatically.",
        ),
      initialValue: true,
    });
    if (prompts.isCancel(confirmed)) {
      cancelAndExit();
    }
    return confirmed;
  }
  prompts.log.info('Prettier configuration detected. Auto-migrating to Oxfmt...');
  return true;
}

export async function promptPrettierMigration(
  projectPath: string,
  interactive: boolean,
  packages?: WorkspacePackage[],
): Promise<boolean> {
  const prettierProject = detectPrettierProject(projectPath, packages);
  if (!prettierProject.hasDependency) {
    return false;
  }
  if (!prettierProject.configFile) {
    // Packages have prettier but no root config → warn and skip
    warnPackageLevelPrettier();
    return false;
  }
  const confirmed = await confirmPrettierMigration(interactive);
  if (!confirmed) {
    return false;
  }
  const ok = await migratePrettierToOxfmt(
    projectPath,
    interactive,
    prettierProject.configFile,
    packages,
  );
  if (!ok) {
    cancelAndExit('Prettier migration failed.', 1);
  }
  return true;
}

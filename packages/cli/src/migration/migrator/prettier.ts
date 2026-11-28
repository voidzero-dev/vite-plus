import fs from 'node:fs';
import path from 'node:path';
import { styleText } from 'node:util';

import {
  confirm as promptConfirm,
  isCancel,
  log as promptLog,
} from '@voidzero-dev/vite-plus-prompts';

import { hasConfigKey, removeConfigKey, rewritePrettier } from '../../../binding/index.js';
import type { WorkspacePackage } from '../../types/index.ts';
import { getVpCommand, runCommandSilently } from '../../utils/command.ts';
import { editJsonFile, readJsonFile } from '../../utils/json.ts';
import { displayRelative } from '../../utils/path.ts';
import { cancelAndExit } from '../../utils/prompts.ts';
import { getSilentSpinner, getSpinner } from '../../utils/spinner.ts';
import { PRETTIER_CONFIG_FILES, PRETTIER_PACKAGE_JSON_CONFIG, detectConfigs } from '../detector.ts';
import { hasOnlyInjectedDefaults, rewriteToolLintStagedConfigFiles } from '../migrator.ts';
import type { MigrationReport } from '../report.ts';
import type { DependencyBag } from './shared.ts';
import {
  removeReplacedDevDependency,
  rewritePackageScriptFields,
  warnMigration,
} from './shared.ts';

export function detectPrettierProject(projectPath: string, packages?: Array<WorkspacePackage>) {
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
 * Unsupported config formats are skipped; other errors still fail migration.
 */
async function runPrettierMigrateStep(
  cwd: string,
  spinner: ReturnType<typeof getSpinner>,
  failMessage: string,
  manualHint: string,
): Promise<'migrated' | 'skipped' | 'failed'> {
  try {
    const result = await runCommandSilently({
      ...getVpCommand(['fmt', '--migrate=prettier']),
      cwd,
      envs: process.env,
    });
    if (result.exitCode !== 0) {
      const stderr = result.stderr.toString().trim();
      // Oxfmt owns config loading. Keep Prettier when its config format is unsupported.
      const unsupportedConfig =
        stderr.includes('Failed to load Prettier config at:') &&
        (stderr.includes('YAML, JSON5 and TOML formats are not supported.') ||
          stderr.includes('Only JSON format is supported for `.prettierrc`.'));
      spinner.stop(unsupportedConfig ? 'Prettier migration skipped' : failMessage);
      if (stderr) {
        promptLog.warn(`⚠ ${stderr}`);
      }
      if (unsupportedConfig) {
        promptLog.info(
          'Keeping Prettier. Convert its config to JSON or JS and run `vp migrate --full` to migrate it later.',
        );
        return 'skipped';
      }
      promptLog.info(manualHint);
      return 'failed';
    }
    return 'migrated';
  } catch {
    spinner.stop(failMessage);
    promptLog.info(manualHint);
    return 'failed';
  }
}

export async function migratePrettierToOxfmt(
  projectPath: string,
  interactive: boolean,
  prettierConfigFile?: string,
  packages?: Array<WorkspacePackage>,
  options?: { silent?: boolean; report?: MigrationReport },
): Promise<'migrated' | 'skipped' | 'failed'> {
  const spinner = options?.silent ? getSilentSpinner() : getSpinner(interactive);

  // Step 1: Generate .oxfmtrc.json from Prettier config
  if (prettierConfigFile) {
    // `vp fmt --migrate` converts nothing when the config already has `fmt`,
    // and still exits successfully. The first `vp migrate` injects `fmt: {}`
    // even when Prettier stays, so remove that empty block to let the converted
    // options take its place. A block with user settings cannot be replaced.
    const { viteConfig } = detectConfigs(projectPath);
    const viteConfigPath = viteConfig && path.join(projectPath, viteConfig);
    let originalViteConfig: string | undefined;
    if (viteConfigPath && hasConfigKey(viteConfigPath, 'fmt')) {
      const withoutDefaults = hasOnlyInjectedDefaults(viteConfigPath, 'fmt')
        ? removeConfigKey(viteConfigPath, 'fmt')
        : undefined;
      if (!withoutDefaults?.updated) {
        warnMigration(
          `Kept Prettier because ${viteConfig} already has a fmt config. Move the Prettier options into it manually, then remove Prettier.`,
          options?.report,
        );
        return 'skipped';
      }
      originalViteConfig = fs.readFileSync(viteConfigPath, 'utf8');
      fs.writeFileSync(viteConfigPath, withoutDefaults.content);
    }

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
        return 'skipped';
      }
    }

    try {
      spinner.start('Migrating Prettier config to Oxfmt...');
      const result = await runPrettierMigrateStep(
        projectPath,
        spinner,
        'Prettier migration failed',
        'You can run `vp fmt --migrate=prettier` manually later',
      );
      if (result !== 'migrated') {
        if (viteConfigPath && originalViteConfig !== undefined) {
          fs.writeFileSync(viteConfigPath, originalViteConfig);
        }
        return result;
      }
      spinner.stop('Prettier config migrated to .oxfmtrc.json');
    } finally {
      if (tempPrettierConfig) {
        try {
          fs.unlinkSync(tempPrettierConfig);
        } catch {
          // A failed conversion may already have removed its temporary config.
        }
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

  return 'migrated';
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
        promptLog.success(`✔ Removed ${displayRelative(configPath)}`);
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
        promptLog.success(`✔ Removed ${displayRelative(configPath)}`);
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
  editJsonFile<
    DependencyBag & {
      scripts?: Record<string, string>;
      'lint-staged'?: Record<string, string | Array<string>>;
    }
  >(packageJsonPath, (pkg) => {
    let changed = false;
    for (const dep of Object.keys(pkg.devDependencies ?? {})) {
      if (
        (dep === 'prettier' || dep.startsWith('prettier-plugin-')) &&
        removeReplacedDevDependency(pkg, dep)
      ) {
        changed = true;
      }
    }
    const scriptsChanged = rewritePackageScriptFields(pkg, rewritePrettier);
    return changed || scriptsChanged ? pkg : undefined;
  });
}

function rewritePrettierLintStagedConfigFiles(projectPath: string, report?: MigrationReport): void {
  rewriteToolLintStagedConfigFiles(projectPath, rewritePrettier, 'prettier', report);
}

export function warnPackageLevelPrettier() {
  promptLog.warn(
    'Prettier detected in workspace packages but no root config found. Package-level Prettier must be migrated manually.',
  );
}

export async function confirmPrettierMigration(interactive: boolean): Promise<boolean> {
  if (interactive) {
    const confirmed = await promptConfirm({
      message:
        'Migrate Prettier to Oxfmt?\n  ' +
        styleText(
          'gray',
          "Oxfmt is Vite+'s built-in formatter that replaces Prettier with faster performance. Your configuration will be converted automatically.",
        ),
      initialValue: true,
    });
    if (isCancel(confirmed)) {
      cancelAndExit();
    }
    return confirmed;
  }
  promptLog.info('Prettier configuration detected. Auto-migrating to Oxfmt...');
  return true;
}

export async function promptPrettierMigration(
  projectPath: string,
  interactive: boolean,
  packages?: Array<WorkspacePackage>,
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
  const result = await migratePrettierToOxfmt(
    projectPath,
    interactive,
    prettierProject.configFile,
    packages,
  );
  if (result === 'failed') {
    cancelAndExit('Prettier migration failed.', 1);
  }
  return result === 'migrated';
}

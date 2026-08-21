import fs from 'node:fs';
import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { PackageManager, type WorkspacePackage } from '../../types/index.ts';
import { runCommandSilently } from '../../utils/command.ts';
import { TSDOWN_MIGRATE_VERSION, TSDOWN_MIGRATION_SKILL_URL } from '../../utils/constants.ts';
import { editJsonFile, readJsonFile } from '../../utils/json.ts';
import { displayRelative } from '../../utils/path.ts';
import { cancelAndExit } from '../../utils/prompts.ts';
import { getSilentSpinner, getSpinner } from '../../utils/spinner.ts';
import { detectConfigs, TSUP_CONFIG_FILES, TSUP_PACKAGE_JSON_CONFIG } from '../detector.ts';
import { addMigrationWarning, type MigrationReport } from '../report.ts';

function showTsdownMigrationOptions(
  targetLabel = 'the project root',
  automaticMigrationFailed = false,
): void {
  const lines = [
    'Choose one of these manual migration methods:',
    `  1. Run \`vp dlx tsdown-migrate\` in ${targetLabel}.`,
    '  2. Use the tsdown migration skill:',
    `     ${TSDOWN_MIGRATION_SKILL_URL}`,
  ];
  if (automaticMigrationFailed) {
    lines.unshift('Automatic tsup migration failed.', '');
    lines.push('');
  }
  prompts.log.info(lines.join('\n'));
}

function showTsdownMigrationSkillGuidance(instructions: string[]): void {
  prompts.log.info(
    [
      ...instructions,
      '',
      'Use the tsdown migration skill for guidance:',
      `  ${TSDOWN_MIGRATION_SKILL_URL}`,
    ].join('\n'),
  );
}

export function detectTsupProject(
  projectPath: string,
  packages?: WorkspacePackage[],
): {
  hasDependency: boolean;
  hasConfig: boolean;
  configFile?: string;
} {
  const packageJsonPath = path.join(projectPath, 'package.json');
  let hasDependency = false;
  if (fs.existsSync(packageJsonPath)) {
    const pkg = readJsonFile(packageJsonPath) as {
      devDependencies?: Record<string, string>;
      dependencies?: Record<string, string>;
    };
    hasDependency = !!(pkg.devDependencies?.tsup || pkg.dependencies?.tsup);
  }
  const configs = detectConfigs(projectPath);
  const configFile = configs.tsupConfig;
  let hasConfig = !!configFile;

  for (const wp of packages ?? []) {
    const workspacePath = path.join(projectPath, wp.path);
    hasConfig ||= !!detectConfigs(workspacePath).tsupConfig;

    if (!hasDependency) {
      const workspacePackageJsonPath = path.join(workspacePath, 'package.json');
      if (fs.existsSync(workspacePackageJsonPath)) {
        const workspacePackageJson = readJsonFile(workspacePackageJsonPath) as {
          devDependencies?: Record<string, string>;
          dependencies?: Record<string, string>;
        };
        hasDependency = !!(
          workspacePackageJson.devDependencies?.tsup || workspacePackageJson.dependencies?.tsup
        );
      }
    }
  }

  return { hasDependency, hasConfig, configFile };
}

const TSDOWN_MIGRATION_FILES = [
  'package.json',
  ...TSUP_CONFIG_FILES,
  ...TSUP_CONFIG_FILES.map((file) => file.replace('tsup', 'tsdown')),
];

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

const STANDARD_TSUP_CONFIG_FILE_PATTERN = TSUP_CONFIG_FILES.flatMap((file) => [
  file,
  file.replace('tsup', 'tsdown'),
])
  .map(escapeRegExp)
  .join('|');
const TSDOWN_STANDARD_CONFIG_OPTION_RE = new RegExp(
  String.raw`(\btsdown(?:-node)?\b(?:(?!&&|\|\||[;|]).)*?)\s+(?:--config(?:=|\s+)|-c\s+)["']?(?:\.[\\/])?(?:${STANDARD_TSUP_CONFIG_FILE_PATTERN})["']?(?=\s|$|[;&|])`,
  'g',
);
const TSUP_CONFIG_OPTION_RE =
  /\btsup(?:-node)?\b(?:(?!&&|\|\||[;|]).)*?\s+(?:--config(?:=|\s+)|-c\s+)(?:"([^"]+)"|'([^']+)'|([^\s;&|]+))(?=\s|$|[;&|])/g;

interface SharedTsupConsumer {
  label: string;
  packagePath: string;
  scriptNames: Set<string>;
}

interface SharedTsupConfig {
  consumers: Map<string, SharedTsupConsumer>;
}

interface TsupConfigLocation {
  configPath: string;
}

interface UnsupportedTsupConfigConsumer {
  configArgument: string;
  packagePath: string;
  scriptName: string;
}

function removeStandardTsdownConfigOption(script: string): string {
  return script.replace(TSDOWN_STANDARD_CONFIG_OPTION_RE, '$1');
}

function resolveScriptConfigPath(packagePath: string, configPath: string): string {
  return path.resolve(packagePath, configPath.replace(/[\\/]/g, path.sep));
}

function collectSharedTsupConfigs(
  projectPath: string,
  packages: WorkspacePackage[] | undefined,
  tsupTargets: string[],
): Map<string, SharedTsupConfig> {
  const migratedConfigPaths = new Set(
    tsupTargets.flatMap((target) =>
      TSUP_CONFIG_FILES.map((file) => path.join(target, file)).filter((file) =>
        fs.existsSync(file),
      ),
    ),
  );
  const sharedConfigs = new Map<string, SharedTsupConfig>();
  const projectRoot = path.resolve(projectPath);
  const consumers = [
    { label: 'the project root', packagePath: projectRoot },
    ...(packages ?? []).map((workspacePackage) => ({
      label: workspacePackage.path,
      packagePath: path.resolve(projectPath, workspacePackage.path),
    })),
  ];

  for (const { label, packagePath } of consumers) {
    const packageJsonPath = path.join(packagePath, 'package.json');
    if (!fs.existsSync(packageJsonPath)) {
      continue;
    }
    const packageJson = readJsonFile(packageJsonPath) as { scripts?: Record<string, string> };
    for (const [scriptName, script] of Object.entries(packageJson.scripts ?? {})) {
      for (const match of script.matchAll(TSUP_CONFIG_OPTION_RE)) {
        const configPath = resolveScriptConfigPath(packagePath, match[1] ?? match[2] ?? match[3]);
        if (!migratedConfigPaths.has(configPath) || path.dirname(configPath) === packagePath) {
          continue;
        }
        const sharedConfig = sharedConfigs.get(configPath) ?? { consumers: new Map() };
        const consumer = sharedConfig.consumers.get(packagePath) ?? {
          label,
          packagePath,
          scriptNames: new Set(),
        };
        consumer.scriptNames.add(scriptName);
        sharedConfig.consumers.set(packagePath, consumer);
        sharedConfigs.set(configPath, sharedConfig);
      }
    }
  }

  return sharedConfigs;
}

function collectTsupConfigCollisions(tsupTargets: string[]): TsupConfigLocation[] {
  const collisions: TsupConfigLocation[] = [];
  for (const target of tsupTargets) {
    const tsdownConfig = detectConfigs(target).tsdownConfig;
    if (tsdownConfig) {
      collisions.push({ configPath: path.join(target, tsdownConfig) });
    }
    const packageJsonPath = path.join(target, 'package.json');
    if (fs.existsSync(packageJsonPath)) {
      const packageJson = readJsonFile(packageJsonPath);
      if (Object.hasOwn(packageJson, 'tsdown')) {
        collisions.push({ configPath: `${packageJsonPath}#tsdown` });
      }
    }
  }
  return collisions;
}

function collectInlineTsupConfigs(tsupTargets: string[]): TsupConfigLocation[] {
  return tsupTargets.flatMap((target) => {
    const packageJsonPath = path.join(target, 'package.json');
    if (!fs.existsSync(packageJsonPath) || !Object.hasOwn(readJsonFile(packageJsonPath), 'tsup')) {
      return [];
    }
    return [{ configPath: `${packageJsonPath}#tsup` }];
  });
}

function isRemovableDefaultConfigArgument(configArgument: string, defaultConfig?: string): boolean {
  return (
    !!defaultConfig &&
    (configArgument === defaultConfig ||
      configArgument === `./${defaultConfig}` ||
      configArgument === `.\\${defaultConfig}`)
  );
}

function collectUnsupportedTsupConfigConsumers(
  tsupTargets: string[],
): UnsupportedTsupConfigConsumer[] {
  const migratedConfigPaths = new Set(
    tsupTargets.flatMap((target) =>
      TSUP_CONFIG_FILES.map((file) => path.join(target, file)).filter((file) =>
        fs.existsSync(file),
      ),
    ),
  );
  const unsupported: UnsupportedTsupConfigConsumer[] = [];

  for (const target of tsupTargets) {
    const packageJsonPath = path.join(target, 'package.json');
    if (!fs.existsSync(packageJsonPath)) {
      continue;
    }
    const detectedConfig = detectConfigs(target).tsupConfig;
    const defaultConfigPath =
      detectedConfig && detectedConfig !== TSUP_PACKAGE_JSON_CONFIG
        ? path.join(target, detectedConfig)
        : undefined;
    const packageJson = readJsonFile(packageJsonPath) as { scripts?: Record<string, string> };

    for (const [scriptName, script] of Object.entries(packageJson.scripts ?? {})) {
      for (const match of script.matchAll(TSUP_CONFIG_OPTION_RE)) {
        const configArgument = match[1] ?? match[2] ?? match[3];
        const configPath = resolveScriptConfigPath(target, configArgument);
        const isDefaultConfig =
          configPath === defaultConfigPath &&
          isRemovableDefaultConfigArgument(configArgument, detectedConfig);
        const isSharedMigratedConfig =
          migratedConfigPaths.has(configPath) && path.dirname(configPath) !== target;
        if (!isDefaultConfig && !isSharedMigratedConfig) {
          unsupported.push({ configArgument, packagePath: target, scriptName });
        }
      }
    }
  }

  return unsupported;
}

function restoreSharedTsupUsage(
  sharedConfigs: Map<string, SharedTsupConfig>,
  snapshots: Map<string, Buffer | undefined>,
): Set<string> {
  const preservedDependencyTargets = new Set<string>();
  const restoredScripts = new Map<string, Set<string>>();

  for (const [configPath, sharedConfig] of sharedConfigs) {
    const contents = snapshots.get(configPath);
    if (contents !== undefined) {
      fs.writeFileSync(configPath, contents);
    }
    preservedDependencyTargets.add(path.dirname(configPath));
    for (const consumer of sharedConfig.consumers.values()) {
      preservedDependencyTargets.add(consumer.packagePath);
      const scriptNames = restoredScripts.get(consumer.packagePath) ?? new Set();
      for (const scriptName of consumer.scriptNames) {
        scriptNames.add(scriptName);
      }
      restoredScripts.set(consumer.packagePath, scriptNames);
    }
  }

  for (const target of preservedDependencyTargets) {
    const packageJsonPath = path.join(target, 'package.json');
    const originalPackageJson = snapshots.get(packageJsonPath);
    if (!originalPackageJson || !fs.existsSync(packageJsonPath)) {
      continue;
    }
    const original = JSON.parse(originalPackageJson.toString()) as {
      scripts?: Record<string, string>;
      devDependencies?: Record<string, string>;
      dependencies?: Record<string, string>;
    };
    editJsonFile<{
      scripts?: Record<string, string>;
      devDependencies?: Record<string, string>;
      dependencies?: Record<string, string>;
    }>(packageJsonPath, (pkg) => {
      let changed = false;
      for (const field of ['devDependencies', 'dependencies'] as const) {
        const spec = original[field]?.tsup;
        if (spec && pkg[field]?.tsup !== spec) {
          pkg[field] ??= {};
          pkg[field].tsup = spec;
          changed = true;
        }
      }
      for (const scriptName of restoredScripts.get(target) ?? []) {
        const script = original.scripts?.[scriptName];
        if (script !== undefined && pkg.scripts?.[scriptName] !== script) {
          pkg.scripts ??= {};
          pkg.scripts[scriptName] = script;
          changed = true;
        }
      }
      return changed ? pkg : undefined;
    });
  }

  return preservedDependencyTargets;
}

function snapshotTsupMigrationTargets(targets: string[]): Map<string, Buffer | undefined> {
  const snapshots = new Map<string, Buffer | undefined>();
  for (const target of targets) {
    for (const file of TSDOWN_MIGRATION_FILES) {
      const filePath = path.join(target, file);
      snapshots.set(filePath, fs.existsSync(filePath) ? fs.readFileSync(filePath) : undefined);
    }
  }
  return snapshots;
}

function restoreTsupMigrationTargets(snapshots: Map<string, Buffer | undefined>): string[] {
  const failures: string[] = [];
  for (const [filePath, contents] of snapshots) {
    try {
      if (contents === undefined) {
        fs.rmSync(filePath, { force: true });
      } else {
        fs.writeFileSync(filePath, contents);
      }
    } catch {
      failures.push(displayRelative(filePath));
    }
  }
  return failures;
}

function extractTsdownMigrationWarnings(output: Buffer): string[] {
  return output
    .toString()
    .replaceAll('\r\n', '\n')
    .split(/\n\s*\n/)
    .map((block) =>
      block
        .trim()
        .match(/^WARN\s+([\s\S]+)$/)?.[1]
        ?.trim(),
    )
    .filter((warning): warning is string => !!warning);
}

/** Run `vp dlx tsdown-migrate` in `cwd` with graceful error handling. */
async function runTsdownMigrateStep(
  vpBin: string,
  cwd: string,
  packageManager: PackageManager,
): Promise<{ ok: boolean; warnings: string[] }> {
  try {
    const result = await runCommandSilently({
      command: vpBin,
      args: [
        'dlx',
        `tsdown-migrate@${TSDOWN_MIGRATE_VERSION}`,
        '--yes',
        '--package-manager',
        packageManager,
        '--no-install',
      ],
      cwd,
      envs: process.env,
    });
    if (result.exitCode !== 0) {
      const stderr = result.stderr.toString().trim();
      if (stderr) {
        prompts.log.warn(`⚠ ${stderr}`);
      }
      return { ok: false, warnings: [] };
    }
    return {
      ok: true,
      warnings: [
        ...extractTsdownMigrationWarnings(result.stdout),
        ...extractTsdownMigrationWarnings(result.stderr),
      ],
    };
  } catch {
    return { ok: false, warnings: [] };
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
  const rootPath = path.resolve(projectPath);
  const targets = [rootPath, ...(packages ?? []).map((p) => path.resolve(rootPath, p.path))];
  const tsupTargets = targets.filter((target) =>
    target === rootPath ? !!tsupConfigFile : !!detectConfigs(target).tsupConfig,
  );
  const configCollisions = collectTsupConfigCollisions(tsupTargets);
  if (configCollisions.length > 0) {
    prompts.log.warn(
      `Automatic tsup migration was skipped because these tsdown configs already exist:\n${configCollisions
        .map(({ configPath }) => `  ${displayRelative(configPath, projectPath)}`)
        .join('\n')}`,
    );
    showTsdownMigrationSkillGuidance([
      'Resolve this configuration conflict manually:',
      '  1. Merge the tsup and tsdown configurations into `pack` in `vite.config.*`.',
      '  2. Do not run `tsdown-migrate`. It can overwrite the existing tsdown configuration.',
    ]);
    return false;
  }
  const inlineTsupConfigs = collectInlineTsupConfigs(tsupTargets);
  if (inlineTsupConfigs.length > 0) {
    prompts.log.warn(
      `Automatic tsup migration was skipped because these inline tsup configs cannot be migrated automatically:\n${inlineTsupConfigs
        .map(({ configPath }) => `  ${displayRelative(configPath, projectPath)}`)
        .join('\n')}`,
    );
    showTsdownMigrationSkillGuidance([
      'Resolve this inline configuration manually:',
      '  1. Move each `package.json#tsup` configuration into `pack` in `vite.config.*`.',
      '  2. Do not run `tsdown-migrate`. Vite+ Pack does not read `package.json#tsdown`.',
    ]);
    return false;
  }
  const unsupportedConfigConsumers = collectUnsupportedTsupConfigConsumers(tsupTargets);
  if (unsupportedConfigConsumers.length > 0) {
    prompts.log.warn(
      `Automatic tsup migration was skipped because these scripts use configs that cannot be migrated automatically:\n${unsupportedConfigConsumers
        .map(
          ({ configArgument, packagePath, scriptName }) =>
            `  ${displayRelative(path.join(packagePath, 'package.json'), projectPath)}#${scriptName} -> ${configArgument}`,
        )
        .join('\n')}`,
    );
    showTsdownMigrationSkillGuidance([
      'Resolve these config paths manually:',
      '  1. Migrate each listed config into `pack` in `vite.config.*`.',
      '  2. Update each listed script.',
      '  3. Do not run `tsdown-migrate`. It cannot safely resolve these config paths.',
    ]);
    return false;
  }
  const sharedConfigs = collectSharedTsupConfigs(projectPath, packages, tsupTargets);

  let preservedDependencyTargets = new Set<string>();

  if (tsupTargets.length > 0) {
    // tsdown-migrate rewrites package.json and renames every tsup config it
    // finds. Preserve those files across all targets so a later failure can
    // roll the complete workspace back to its pre-migration state.
    const snapshots = snapshotTsupMigrationTargets(tsupTargets);
    const migrationWarnings: { targetLabel: string; warning: string }[] = [];
    spinner.start('Migrating tsup config to tsdown...');
    for (const target of tsupTargets) {
      const targetLabel =
        target === rootPath ? 'the project root' : displayRelative(target, projectPath);
      const migrateResult = await runTsdownMigrateStep(vpBin, target, packageManager);
      if (!migrateResult.ok) {
        spinner.stop();
        const restoreFailures = restoreTsupMigrationTargets(snapshots);
        if (restoreFailures.length > 0) {
          prompts.log.warn(
            `Could not restore these files after the failed migration:\n${restoreFailures
              .map((file) => `  ${file}`)
              .join('\n')}`,
          );
        }
        showTsdownMigrationOptions(targetLabel, true);
        return false;
      }
      for (const warning of migrateResult.warnings) {
        migrationWarnings.push({ targetLabel, warning });
      }
    }
    spinner.stop('tsup config migrated to tsdown.config');

    // A root or workspace package can explicitly load a config owned by a
    // different migration target. Keep that tsup config, its dependency, and
    // each affected script so no consumer points to a renamed file.
    preservedDependencyTargets = restoreSharedTsupUsage(sharedConfigs, snapshots);

    for (const { targetLabel, warning } of migrationWarnings) {
      const message =
        targetLabel === 'the project root'
          ? `tsdown-migrate: ${warning}`
          : `tsdown-migrate (${targetLabel}): ${warning}`;
      if (options?.report) {
        addMigrationWarning(options.report, message);
      } else {
        prompts.log.warn(message);
      }
    }
    for (const [configPath, sharedConfig] of sharedConfigs) {
      const message = `${displayRelative(configPath, projectPath)} is shared by ${[
        ...sharedConfig.consumers.values(),
      ]
        .map((consumer) => consumer.label)
        .toSorted()
        .join(', ')}. It was preserved and must be migrated manually.`;
      if (options?.report) {
        addMigrationWarning(options.report, message);
      } else {
        prompts.log.warn(message);
      }
    }
  }

  if (options?.report) {
    options.report.tsupMigrated = true;
  }

  // Only clean packages that tsdown-migrate processed. Other packages can
  // still use tsup when they do not have a config that can be migrated.
  for (const target of tsupTargets) {
    if (!fs.existsSync(path.join(target, 'package.json'))) {
      continue;
    }
    const preservedConfigs = new Set(
      [...sharedConfigs.keys()].filter(
        (configPath) => path.dirname(configPath) === path.resolve(target),
      ),
    );
    deleteTsupConfigFiles(target, preservedConfigs, options?.report, options?.silent);
    rewriteTsupPackageJson(
      path.join(target, 'package.json'),
      preservedDependencyTargets.has(path.resolve(target)),
    );
  }

  return true;
}

function deleteTsupConfigFiles(
  basePath: string,
  preservedConfigs: ReadonlySet<string>,
  report?: MigrationReport,
  silent = false,
): void {
  const configs = detectConfigs(basePath);
  if (configs.tsupConfig && configs.tsupConfig !== TSUP_PACKAGE_JSON_CONFIG) {
    const configPath = path.join(basePath, configs.tsupConfig);
    if (!preservedConfigs.has(configPath) && fs.existsSync(configPath)) {
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
    if (!preservedConfigs.has(configPath) && fs.existsSync(configPath)) {
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

function rewriteTsupPackageJson(packageJsonPath: string, preserveTsupDependency = false): void {
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }
  editJsonFile<{
    scripts?: Record<string, string>;
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  }>(packageJsonPath, (pkg) => {
    let changed = false;
    // tsdown-migrate rewrites the command to tsdown. Vite+ Pack automatically
    // loads standard tsdown config files and does not accept `--config`, so
    // remove that redundant option before the generic tsdown -> vp pack rule.
    for (const [name, script] of Object.entries(pkg.scripts ?? {})) {
      const rewritten = removeStandardTsdownConfigOption(script);
      if (rewritten !== script) {
        pkg.scripts![name] = rewritten;
        changed = true;
      }
    }

    // Remove any tsup dependency left by the external migrator. The tsdown
    // dependency is removed later because vite-plus bundles it.
    if (!preserveTsupDependency) {
      for (const field of ['devDependencies', 'dependencies'] as const) {
        if (pkg[field]?.tsup) {
          delete pkg[field].tsup;
          changed = true;
        }
      }
    }
    return changed ? pkg : undefined;
  });
}

export function warnMissingTsupConfig() {
  prompts.log.warn(
    'tsup detected, but no tsup config was found. The tsup setup must be migrated manually.',
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
    if (!confirmed) {
      showTsdownMigrationOptions();
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
  if (!tsupProject.hasConfig) {
    warnMissingTsupConfig();
    return false;
  }
  const confirmed = await confirmTsupMigration(interactive);
  if (!confirmed) {
    return false;
  }
  const ok = await migrateTsupToTsdown(
    projectPath,
    interactive,
    packageManager,
    tsupProject.configFile,
    packages,
  );
  if (!ok) {
    cancelAndExit('Complete the tsup migration manually, then re-run `vp migrate`.', 1);
  }
  return true;
}

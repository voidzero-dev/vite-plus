import * as prompts from '@voidzero-dev/vite-plus-prompts';
import semver from 'semver';

import { downloadPackageManager as downloadPackageManagerBinding } from '../../binding/index.js';
import { PackageManager } from '../types/index.ts';
import { isPnpmIgnoredBuildsError, parseInstallGatedBuilds } from './approve-builds.ts';
import { runCommandSilently } from './command.ts';
import { getSilentSpinner, getSpinner } from './spinner.ts';
import { accent } from './terminal.ts';

export interface CommandRunSummary {
  durationMs: number;
  exitCode?: number;
  status: 'installed' | 'formatted' | 'failed' | 'skipped';
  /**
   * pnpm packages whose build (install/postinstall) scripts were gated during
   * the install. Only populated when `runViteInstall` is called with
   * `detectIgnoredBuilds`. See {@link runViteInstall}.
   */
  pendingBuilds?: string[];
}

/**
 * pnpm v11 promoted `ERR_PNPM_IGNORED_BUILDS` from a warning to a hard
 * exit-1. Auto-installs run by `vp migrate` / `vp create` happen before the
 * user has a chance to approve build scripts via `pnpm.onlyBuiltDependencies`,
 * so transitive deps like `esbuild` would fail the install. Pass
 * `--ignore-scripts` in that window so the orchestration succeeds; the user's
 * own subsequent `vp install` keeps default pnpm behavior.
 */
export function shouldIgnoreScriptsForAutoInstall(
  packageManager: PackageManager | undefined,
  packageManagerVersion: string | undefined,
): boolean {
  if (packageManager !== PackageManager.pnpm) {
    return false;
  }
  const coerced = packageManagerVersion ? semver.coerce(packageManagerVersion)?.version : undefined;
  if (!coerced) {
    return false;
  }
  return semver.gte(coerced, '11.0.0');
}

export function cancelAndExit(message = 'Operation cancelled', exitCode = 0): never {
  prompts.cancel(message);
  process.exit(exitCode);
}

export async function selectPackageManager(interactive?: boolean, silent = false) {
  if (interactive) {
    const selected = await prompts.select({
      message: 'Which package manager would you like to use?',
      options: [
        { value: PackageManager.pnpm, hint: 'recommended' },
        { value: PackageManager.yarn },
        { value: PackageManager.npm },
        { value: PackageManager.bun },
      ],
      initialValue: PackageManager.pnpm,
    });

    if (prompts.isCancel(selected)) {
      cancelAndExit();
    }

    return selected;
  } else {
    // --no-interactive: use pnpm as default
    if (!silent) {
      prompts.log.info(`Using default package manager: ${accent(PackageManager.pnpm)}`);
    }
    return PackageManager.pnpm;
  }
}

export async function downloadPackageManager(
  packageManager: PackageManager,
  version: string,
  interactive?: boolean,
  silent = false,
) {
  const spinner = silent ? getSilentSpinner() : getSpinner(interactive);
  spinner.start(`${packageManager}@${version} installing...`);
  const downloadResult = await downloadPackageManagerBinding({
    name: packageManager,
    version,
  });
  spinner.stop(`${packageManager}@${downloadResult.version} installed`);
  return downloadResult;
}

export async function runViteInstall(
  cwd: string,
  interactive?: boolean,
  extraArgs?: string[],
  options?: {
    silent?: boolean;
    packageManager?: PackageManager;
    packageManagerVersion?: string;
    /**
     * Surface pnpm's gated build scripts instead of suppressing them. When set,
     * the auto `--ignore-scripts` workaround is skipped (so pnpm records which
     * packages need approval), a pnpm >= 11 `ERR_PNPM_IGNORED_BUILDS` exit is
     * treated as a successful install (deps are on disk), and the gated package
     * names are returned in `pendingBuilds`.
     */
    detectIgnoredBuilds?: boolean;
  },
) {
  // install dependencies on non-CI environment
  if (process.env.VP_SKIP_INSTALL) {
    return { durationMs: 0, status: 'skipped' } satisfies CommandRunSummary;
  }

  const detectIgnoredBuilds = options?.detectIgnoredBuilds === true;
  const installArgs = [...(extraArgs ?? [])];
  // `--ignore-scripts` keeps auto-installs from hard-failing on pnpm >= 11, but
  // it also makes the gated builds unrecoverable (`approve-builds` then reports
  // nothing pending). When the caller wants to act on those builds, leave the
  // flag off and instead tolerate the resulting `ERR_PNPM_IGNORED_BUILDS` exit
  // below.
  if (
    !detectIgnoredBuilds &&
    shouldIgnoreScriptsForAutoInstall(options?.packageManager, options?.packageManagerVersion) &&
    !installArgs.includes('--ignore-scripts')
  ) {
    installArgs.push('--ignore-scripts');
  }

  const spinner = options?.silent ? getSilentSpinner() : getSpinner(interactive);
  const startTime = Date.now();
  spinner.start(`Installing dependencies...`);
  const { exitCode, stderr, stdout } = await runCommandSilently({
    command: process.env.VP_CLI_BIN ?? 'vp',
    args: ['install', ...installArgs],
    cwd,
    envs: process.env,
  });
  const combinedOutput = `${stdout.toString()}\n${stderr.toString()}`;
  const pendingBuilds = detectIgnoredBuilds
    ? parseInstallGatedBuilds(combinedOutput, options?.packageManager)
    : undefined;
  // pnpm >= 11 exits 1 when it gates a build script, but the install itself
  // completed (deps are on disk). Treat that one case as success so callers can
  // offer to approve the builds rather than report a broken install.
  const ignoredBuildsOnly =
    exitCode !== 0 && detectIgnoredBuilds && isPnpmIgnoredBuildsError(combinedOutput);
  if (exitCode === 0 || ignoredBuildsOnly) {
    spinner.stop(`Dependencies installed`);
    return {
      durationMs: Date.now() - startTime,
      exitCode,
      status: 'installed',
      pendingBuilds,
    } satisfies CommandRunSummary;
  } else {
    spinner.stop(`Install failed`);
    prompts.log.info(stdout.toString());
    prompts.log.error(stderr.toString());
    prompts.log.info(`You may need to run "vp install" manually in ${cwd}`);
    return {
      durationMs: Date.now() - startTime,
      exitCode,
      status: 'failed',
    } satisfies CommandRunSummary;
  }
}

export async function runViteFmt(
  cwd: string,
  interactive?: boolean,
  paths?: string[],
  options?: { silent?: boolean },
) {
  const spinner = options?.silent ? getSilentSpinner() : getSpinner(interactive);
  const startTime = Date.now();
  spinner.start(`Formatting code...`);

  const { exitCode, stderr, stdout } = await runCommandSilently({
    command: process.env.VP_CLI_BIN ?? 'vp',
    args: ['fmt', '--write', ...(paths ?? [])],
    cwd,
    envs: process.env,
  });

  if (exitCode === 0) {
    spinner.stop(`Code formatted`);
    return {
      durationMs: Date.now() - startTime,
      exitCode,
      status: 'formatted',
    } satisfies CommandRunSummary;
  } else {
    spinner.stop(`Format failed`);
    prompts.log.info(stdout.toString());
    prompts.log.error(stderr.toString());
    const relativePaths = (paths ?? []).length > 0 ? ` ${(paths ?? []).join(' ')}` : '';
    prompts.log.info(`You may need to run "vp fmt --write${relativePaths}" manually in ${cwd}`);
    return {
      durationMs: Date.now() - startTime,
      exitCode,
      status: 'failed',
    } satisfies CommandRunSummary;
  }
}

export async function upgradeYarn(cwd: string, interactive?: boolean, silent = false) {
  const spinner = silent ? getSilentSpinner() : getSpinner(interactive);
  spinner.start(`Running yarn set version stable...`);
  const { exitCode, stderr, stdout } = await runCommandSilently({
    command: 'yarn',
    args: ['set', 'version', 'stable'],
    cwd,
    envs: process.env,
  });
  if (exitCode === 0) {
    spinner.stop(`Yarn upgraded to stable version`);
  } else {
    spinner.stop(`yarn upgrade failed`);
    prompts.log.info(stdout.toString());
    prompts.log.error(stderr.toString());
  }
}

export async function promptGitHooks(options: {
  hooks?: boolean;
  interactive: boolean;
}): Promise<boolean> {
  if (options.hooks === false) {
    return false;
  }
  if (options.hooks === true) {
    return true;
  }
  if (options.interactive) {
    const selected = await prompts.confirm({
      message:
        'Set up pre-commit hooks to run formatting, linting, and type checking with auto-fixes?',
      initialValue: true,
    });
    if (prompts.isCancel(selected)) {
      cancelAndExit();
      return false;
    }
    return selected;
  }
  return true; // non-interactive default
}

export async function promptGitInit(options: {
  git?: boolean;
  interactive: boolean;
}): Promise<boolean> {
  if (options.git === false) {
    return false;
  }
  if (options.git === true) {
    return true;
  }
  if (options.interactive) {
    const selected = await prompts.confirm({
      message: 'Initialize a git repository?',
      initialValue: true,
    });
    if (prompts.isCancel(selected)) {
      cancelAndExit();
      return false;
    }
    return selected;
  }
  return false; // non-interactive default
}

// Git initialization only applies to a brand-new standalone project or
// monorepo. A package added to an existing monorepo shares that monorepo's
// repository, so git setup (and its prompt) is skipped.
export async function resolveGitInit(
  options: { git?: boolean; interactive: boolean },
  isMonorepo: boolean,
): Promise<boolean> {
  if (isMonorepo) {
    return false;
  }
  return promptGitInit(options);
}

export function defaultInteractive() {
  // If CI environment, use non-interactive mode by default
  return !process.env.CI && process.stdin.isTTY;
}

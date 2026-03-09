import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { downloadPackageManager as downloadPackageManagerBinding } from '../../binding/index.js';
import { fmt as resolveFmt } from '../resolve-fmt.js';
import { PackageManager } from '../types/index.js';
import { runCommandSilently } from './command.js';
import { accent } from './terminal.js';

export interface CommandRunSummary {
  durationMs: number;
  exitCode?: number;
  status: 'installed' | 'formatted' | 'failed' | 'skipped';
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
  options?: { silent?: boolean },
) {
  // install dependencies on non-CI environment
  if (process.env.VITE_PLUS_SKIP_INSTALL) {
    return { durationMs: 0, status: 'skipped' } satisfies CommandRunSummary;
  }

  const spinner = options?.silent ? getSilentSpinner() : getSpinner(interactive);
  const startTime = Date.now();
  spinner.start(`Installing dependencies...`);
  const { exitCode, stderr, stdout } = await runCommandSilently({
    command: process.env.VITE_PLUS_CLI_BIN ?? 'vp',
    args: ['install', ...(extraArgs ?? [])],
    cwd,
    envs: process.env,
  });
  if (exitCode === 0) {
    spinner.stop(`Dependencies installed`);
    return {
      durationMs: Date.now() - startTime,
      exitCode,
      status: 'installed',
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

  const { binPath, envs } = await resolveFmt();
  const { exitCode, stderr, stdout } = await runCommandSilently({
    command: binPath,
    args: ['--write', ...(paths ?? [])],
    cwd,
    envs: {
      ...process.env,
      ...envs,
    },
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

export async function upgradeYarn(cwd: string, interactive?: boolean) {
  const spinner = getSpinner(interactive);
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

export function defaultInteractive() {
  // If CI environment, use non-interactive mode by default
  return !process.env.CI && process.stdin.isTTY;
}

export function getSpinner(interactive?: boolean) {
  if (interactive) {
    return prompts.spinner();
  }
  return {
    start: (msg?: string) => {
      if (msg) {
        prompts.log.info(msg);
      }
    },
    stop: (msg?: string) => {
      if (msg) {
        prompts.log.info(msg);
      }
    },
    message: (msg?: string) => {
      if (msg) {
        prompts.log.info(msg);
      }
    },
  };
}

function getSilentSpinner() {
  return {
    start: () => {},
    stop: () => {},
    message: () => {},
  };
}

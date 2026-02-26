import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { downloadPackageManager as downloadPackageManagerBinding } from '../../binding/index.js';
import { PackageManager } from '../types/index.js';
import { runCommandSilently } from './command.js';
import { accent } from './terminal.js';

export function cancelAndExit(message = 'Operation cancelled', exitCode = 0): never {
  prompts.cancel(message);
  process.exit(exitCode);
}

export async function selectPackageManager(interactive?: boolean) {
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
    prompts.log.info(`Using default package manager: ${accent(PackageManager.pnpm)}`);
    return PackageManager.pnpm;
  }
}

export async function downloadPackageManager(
  packageManager: PackageManager,
  version: string,
  interactive?: boolean,
) {
  const spinner = getSpinner(interactive);
  spinner.start(`${packageManager}@${version} installing...`);
  const downloadResult = await downloadPackageManagerBinding({
    name: packageManager,
    version,
  });
  spinner.stop(`${packageManager}@${downloadResult.version} installed`);
  return downloadResult;
}

export async function runViteInstall(cwd: string, interactive?: boolean, extraArgs?: string[]) {
  // install dependencies on non-CI environment
  if (process.env.CI) {
    return;
  }

  const spinner = getSpinner(interactive);
  spinner.start(`Installing dependencies...`);
  const { exitCode, stderr, stdout } = await runCommandSilently({
    command: process.env.VITE_PLUS_CLI_BIN ?? 'vp',
    args: ['install', ...(extraArgs ?? [])],
    cwd,
    envs: process.env,
  });
  if (exitCode === 0) {
    spinner.stop(`Dependencies installed`);
  } else {
    spinner.stop(`Install failed`);
    prompts.log.info(stdout.toString());
    prompts.log.error(stderr.toString());
    prompts.log.info(`You may need to run "vp install" manually in ${cwd}`);
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

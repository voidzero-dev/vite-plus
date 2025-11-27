import * as prompts from '@clack/prompts';
import { downloadPackageManager as downloadPackageManagerBinding } from '@voidzero-dev/vite-plus/binding';
import colors from 'picocolors';

import { PackageManager } from '../types/index.ts';
import { runCommandSilently } from './command.ts';

const { cyan } = colors;

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
    prompts.log.info(`Using default package manager: ${cyan(PackageManager.pnpm)}`);
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

export async function runViteInstall(cwd: string, interactive?: boolean) {
  // install dependencies on non-CI environment
  if (process.env.CI) {
    return;
  }

  const spinner = getSpinner(interactive);
  spinner.start(`Running vite install...`);
  const { exitCode, stderr, stdout } = await runCommandSilently({
    command: 'vite',
    args: ['install'],
    cwd,
    envs: process.env,
  });
  if (exitCode === 0) {
    spinner.stop(`Dependencies installed`);
  } else {
    spinner.stop(`Install failed`);
    prompts.log.info(stdout.toString());
    prompts.log.error(stderr.toString());
    prompts.log.info(`You may need to run "vite install" manually in ${cwd}`);
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

function getSpinner(interactive?: boolean) {
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

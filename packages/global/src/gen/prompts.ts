import fs from 'node:fs';

import * as prompts from '@clack/prompts';
import colors from 'picocolors';
import validateNpmPackageName from 'validate-npm-package-name';

import { emptyDir, getProjectDirFromPackageName, isEmpty } from './utils.ts';

const { cyan } = colors;

export async function promptPackageNameAndTargetDir(
  defaultPackageName: string,
  interactive?: boolean,
) {
  let packageName: string;
  let targetDir: string;

  if (interactive) {
    const selected = await prompts.text({
      message: 'Package name:',
      placeholder: defaultPackageName,
      defaultValue: defaultPackageName,
      validate: (value) => {
        if (value.length === 0) return;

        const result = validateNpmPackageName(value);
        if (result.validForNewPackages) return;
        return (
          result.errors?.[0] ?? result.warnings?.[0] ?? 'Invalid package name'
        );
      },
    });
    if (prompts.isCancel(selected)) {
      cancelAndExit();
    }
    packageName = selected;
    targetDir = getProjectDirFromPackageName(packageName);
  } else {
    // --no-interactive: use default
    packageName = defaultPackageName;
    targetDir = getProjectDirFromPackageName(packageName);
    prompts.log.info(`Using default package name: ${cyan(packageName)}`);
  }

  return { packageName, targetDir };
}

export async function checkProjectDirExists(
  projectDirFullPath: string,
  interactive?: boolean,
) {
  if (!fs.existsSync(projectDirFullPath) || isEmpty(projectDirFullPath)) {
    return;
  }
  if (!interactive) {
    prompts.log.info(
      'Use --directory to specify a different location or remove the directory first',
    );
    cancelAndExit(`Target directory "${projectDirFullPath}" is not empty`, 1);
  }

  // Handle directory if it exists and is not empty
  const overwrite = await prompts.select({
    message: `Target directory "${projectDirFullPath}" is not empty. Please choose how to proceed:`,
    options: [
      {
        label: 'Cancel operation',
        value: 'no',
      },
      {
        label: 'Remove existing files and continue',
        value: 'yes',
      },
    ],
  });

  if (prompts.isCancel(overwrite)) {
    cancelAndExit();
  }

  switch (overwrite) {
    case 'yes':
      emptyDir(projectDirFullPath);
      break;
    case 'no':
      cancelAndExit();
  }
}

export function cancelAndExit(
  message = 'Operation cancelled',
  exitCode = 0,
): never {
  prompts.cancel(message);
  process.exit(exitCode);
}

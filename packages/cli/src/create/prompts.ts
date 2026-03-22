import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import validateNpmPackageName from 'validate-npm-package-name';

import { accent } from '../utils/terminal.js';
import { getRandomProjectName } from './random-name.js';
import { getProjectDirFromPackageName } from './utils.js';

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
        if (value == null || value.length === 0) {
          return;
        }
        const result = value ? validateNpmPackageName(value) : null;
        if (result?.validForNewPackages) {
          return;
        }
        return result?.errors?.[0] ?? result?.warnings?.[0] ?? 'Invalid package name';
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
    prompts.log.info(`Using default package name: ${accent(packageName)}`);
  }

  return { packageName, targetDir };
}

export async function promptTargetDir(
  defaultTargetDir: string,
  interactive?: boolean,
  options?: { cwd?: string },
) {
  let targetDir: string;

  if (interactive) {
    const selected = await prompts.text({
      message: 'Target directory:',
      placeholder: defaultTargetDir,
      defaultValue: defaultTargetDir,
      validate: (value) => validateTargetDir(value ?? defaultTargetDir, options?.cwd).error,
    });
    if (prompts.isCancel(selected)) {
      cancelAndExit();
    }
    targetDir = validateTargetDir(selected ?? defaultTargetDir, options?.cwd).directory;
  } else {
    targetDir = validateTargetDir(defaultTargetDir, options?.cwd).directory;
    prompts.log.info(`Using default target directory: ${accent(targetDir)}`);
  }

  return targetDir;
}

export function suggestAvailableTargetDir(defaultTargetDir: string, cwd: string) {
  let suggestedTargetDir = defaultTargetDir;
  let attempt = 1;

  while (!isTargetDirAvailable(path.join(cwd, suggestedTargetDir))) {
    suggestedTargetDir = getRandomProjectName({ fallbackName: `${defaultTargetDir}-${attempt}` });
    attempt++;
  }

  return suggestedTargetDir;
}

export async function checkProjectDirExists(projectDirFullPath: string, interactive?: boolean) {
  if (isTargetDirAvailable(projectDirFullPath)) {
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

export function cancelAndExit(message = 'Operation cancelled', exitCode = 0): never {
  prompts.cancel(message);
  process.exit(exitCode);
}

function isEmpty(path: string) {
  const files = fs.readdirSync(path);
  return files.length === 0 || (files.length === 1 && files[0] === '.git');
}

function emptyDir(dir: string) {
  if (!fs.existsSync(dir)) {
    return;
  }
  for (const file of fs.readdirSync(dir)) {
    if (file === '.git') {
      continue;
    }
    fs.rmSync(path.resolve(dir, file), { recursive: true, force: true });
  }
}

export function isTargetDirAvailable(projectDirFullPath: string) {
  return !fs.existsSync(projectDirFullPath) || isEmpty(projectDirFullPath);
}

function validateTargetDir(input?: string, cwd?: string): { directory: string; error?: string } {
  const value = input?.trim() ?? '';
  if (!value) {
    return { directory: '', error: 'Target directory is required' };
  }

  const targetDir = path.normalize(value);
  if (!targetDir || targetDir === '.') {
    return { directory: '', error: 'Target directory is required' };
  }
  if (path.isAbsolute(targetDir)) {
    return { directory: '', error: 'Absolute path is not allowed' };
  }
  if (targetDir.includes('..')) {
    return { directory: '', error: 'Relative path contains ".." which is not allowed' };
  }
  if (cwd && !isTargetDirAvailable(path.join(cwd, targetDir))) {
    return { directory: '', error: `Target directory "${targetDir}" already exists` };
  }
  return { directory: targetDir };
}

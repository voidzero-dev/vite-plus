import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import validateNpmPackageName from 'validate-npm-package-name';

import { cancelAndExit } from '../utils/prompts.ts';
import { accent } from '../utils/terminal.ts';
import { getRandomProjectName } from './random-name.ts';
import { getProjectDirFromPackageName } from './utils.ts';

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
          return undefined;
        }
        const result = value ? validateNpmPackageName(value) : null;
        if (result?.validForNewPackages) {
          return undefined;
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

function describeExistingTarget(projectDirFullPath: string, stats: fs.Stats) {
  if (stats.isSymbolicLink()) {
    return {
      description: `Target path "${projectDirFullPath}" is a symbolic link`,
      removeLabel: 'Remove symbolic link and continue',
    };
  }
  if (stats.isDirectory()) {
    return {
      description: `Target directory "${projectDirFullPath}" is not empty`,
      removeLabel: 'Remove existing files and continue',
    };
  }
  return {
    description: `Target path "${projectDirFullPath}" already exists`,
    removeLabel: 'Remove existing path and continue',
  };
}

function stripTrailingPathSeparators(targetPath: string) {
  const root = path.parse(targetPath).root;
  let end = targetPath.length;
  while (end > root.length && targetPath[end - 1] === path.sep) {
    end--;
  }
  return targetPath.slice(0, end);
}

export async function checkProjectDirExists(projectDirFullPath: string, interactive?: boolean) {
  const targetPath = stripTrailingPathSeparators(projectDirFullPath);
  const stats = fs.lstatSync(targetPath, { throwIfNoEntry: false });
  if (!stats || (stats.isDirectory() && isEmpty(targetPath))) {
    return;
  }
  const { description, removeLabel } = describeExistingTarget(targetPath, stats);
  if (!interactive) {
    prompts.log.info(
      'Use --directory to specify a different location or remove the directory first',
    );
    cancelAndExit(description, 1);
  }

  // Handle an existing target that cannot be reused as-is.
  const overwrite = await prompts.select({
    message: `${description}. Please choose how to proceed:`,
    options: [
      {
        label: 'Cancel operation',
        value: 'no',
      },
      {
        label: removeLabel,
        value: 'yes',
      },
    ],
  });

  if (prompts.isCancel(overwrite)) {
    cancelAndExit();
  }

  switch (overwrite) {
    case 'yes':
      clearTargetPath(targetPath);
      break;
    case 'no':
      cancelAndExit();
  }
}

function isEmpty(path: string) {
  const files = fs.readdirSync(path);
  return files.length === 0 || (files.length === 1 && files[0] === '.git');
}

function clearTargetPath(targetPath: string) {
  const strippedTargetPath = stripTrailingPathSeparators(targetPath);
  const stats = fs.lstatSync(strippedTargetPath, { throwIfNoEntry: false });
  if (!stats) {
    return;
  }
  if (!stats.isDirectory()) {
    fs.rmSync(strippedTargetPath, { force: true });
    return;
  }
  for (const file of fs.readdirSync(strippedTargetPath)) {
    if (file === '.git') {
      continue;
    }
    fs.rmSync(path.resolve(strippedTargetPath, file), { recursive: true, force: true });
  }
}

export function isTargetDirAvailable(projectDirFullPath: string) {
  const targetPath = stripTrailingPathSeparators(projectDirFullPath);
  const stats = fs.lstatSync(targetPath, { throwIfNoEntry: false });
  return !stats || (stats.isDirectory() && isEmpty(targetPath));
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

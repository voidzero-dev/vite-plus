import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

import * as prompts from '@clack/prompts';

import { detectWorkspace as detectWorkspaceBinding } from '../../binding/index.js';
import {
  defaultInteractive,
  detectPackageMetadata,
  readNearestPackageJson,
  VITE_PLUS_NAME,
  cancelAndExit,
  runViteInstall,
  runCommand,
} from '../utils/index.js';

const cwd = process.cwd();
const interactive = defaultInteractive();
let localCliMetadata = detectPackageMetadata(cwd, VITE_PLUS_NAME);
let startPrompts = false;

// check local CLI already added to devDependencies but not installed
if (!localCliMetadata) {
  const pkg = readNearestPackageJson<{
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  }>(cwd);
  if (pkg?.devDependencies?.[VITE_PLUS_NAME] || pkg?.dependencies?.[VITE_PLUS_NAME]) {
    prompts.intro(`Vite+ Local CLI "${VITE_PLUS_NAME}" not found`);
    startPrompts = true;
    // run vite install and detect package metadata again
    await runViteInstall(cwd, interactive);
    localCliMetadata = detectPackageMetadata(cwd, VITE_PLUS_NAME);
    if (localCliMetadata) {
      prompts.outro(`Using Vite+ Local CLI`);
    }
  }
}

if (!localCliMetadata) {
  let autoInstall = true;
  if (!startPrompts) {
    prompts.intro(`Vite+ Local CLI "${VITE_PLUS_NAME}" not found`);
  }
  if (interactive) {
    const selected = await prompts.confirm({
      message: `Do you want to add ${VITE_PLUS_NAME} to devDependencies?`,
      initialValue: true,
    });

    if (prompts.isCancel(selected) || !selected) {
      autoInstall = false;
    }
  }
  if (!autoInstall) {
    cancelAndExit(`Please add ${VITE_PLUS_NAME} to devDependencies first`, 1);
  }

  const args = ['add', '-D', VITE_PLUS_NAME];
  // add -w if cwd is root workspace under monorepo
  const workspaceInfo = await detectWorkspaceBinding(cwd);
  if (workspaceInfo.isMonorepo) {
    args.push('-w');
  }
  const exitCode = await runCommand({
    command: 'vite',
    args,
    cwd,
    envs: process.env,
  });
  if (exitCode === 0) {
    prompts.log.success(`${VITE_PLUS_NAME} added`);
  } else {
    prompts.log.error(`Add ${VITE_PLUS_NAME} failed`);
  }

  localCliMetadata = detectPackageMetadata(cwd, VITE_PLUS_NAME);
  if (!localCliMetadata) {
    prompts.log.info(`You may need to run "vite ${args.join(' ')}" manually in ${cwd}`);
    cancelAndExit(`Failed to locate Vite+ Local CLI`, 2);
  }
  prompts.outro(`Using Vite+ Local CLI`);
}

// delegate to local CLI
import(pathToFileURL(join(localCliMetadata.path, 'dist', 'bin.js')).href);

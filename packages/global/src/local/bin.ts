import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

import * as prompts from '@clack/prompts';

import { detectWorkspace as detectWorkspaceBinding } from '../../binding/index.js';
import { runCommand } from '../utils/command.js';
import { VITE_PLUS_NAME } from '../utils/constants.js';
import {
  detectPackageMetadata,
  hasVitePlusDependency,
  readNearestPackageJson,
} from '../utils/package.js';
import { cancelAndExit, defaultInteractive, runViteInstall } from '../utils/prompts.js';
import type { PackageDependencies } from '../utils/types.js';

const cwd = process.cwd();
const interactive = defaultInteractive();
let localCliMetadata = detectPackageMetadata(cwd, VITE_PLUS_NAME);
let startPrompts = false;

if (!localCliMetadata) {
  if (hasVitePlusDependency(readNearestPackageJson<PackageDependencies>(cwd))) {
    prompts.intro(`Installing "${VITE_PLUS_NAME}"…`);
    startPrompts = true;
    await runViteInstall(cwd, interactive);
    localCliMetadata = detectPackageMetadata(cwd, VITE_PLUS_NAME);
  }
}

if (!localCliMetadata) {
  let autoInstall = true;
  if (!startPrompts) {
    prompts.intro(`Local "${VITE_PLUS_NAME}" package was not found`);
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
    // use VITE_PLUS_CLI_BIN environment variable if set, otherwise use 'vp'
    command: process.env.VITE_PLUS_CLI_BIN ?? 'vp',
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
    cancelAndExit(`Failed to locate local Vite+ CLI`, 2);
  }
  prompts.outro(`Using local Vite+ CLI`);
}

await import(pathToFileURL(join(localCliMetadata.path, 'dist', 'bin.js')).href);

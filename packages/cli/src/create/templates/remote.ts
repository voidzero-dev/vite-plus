import * as prompts from '@voidzero-dev/vite-plus-prompts';
import colors from 'picocolors';

import type { WorkspaceInfo } from '../../types/index.ts';
import { runCommand, runCommandSilently } from '../../utils/command.ts';
import { checkNpmPackageExists } from '../../utils/package.ts';
import {
  type ExecutionWithProjectDir,
  formatDlxCommand,
  runCommandAndDetectProjectDir,
} from '../command.ts';
import type { TemplateInfo } from './types.ts';

const { gray, yellow } = colors;

export async function executeRemoteTemplate(
  workspaceInfo: WorkspaceInfo,
  templateInfo: TemplateInfo,
  options?: { silent?: boolean },
): Promise<ExecutionWithProjectDir> {
  const silent = options?.silent ?? false;
  if (!silent) {
    prompts.log.step('Generating project…');
  }

  let isGitHubTemplate = templateInfo.command === 'degit';
  let result: ExecutionWithProjectDir;
  if (templateInfo.command === 'node') {
    // Template found locally - execute directly
    const command = templateInfo.command;
    const args = templateInfo.args;
    const envs = templateInfo.envs;
    if (!silent) {
      prompts.log.info(`Running: ${gray(`${command} ${args.join(' ')}`)}`);
    }
    result = await runCommandAndDetectProjectDir(
      { command, args, cwd: workspaceInfo.rootDir, envs },
      templateInfo.parentDir,
    );
  } else {
    // TODO: prompt for project name if not provided for degit
    // Template not found - use package manager runner (npx/pnpm dlx/etc.)
    if (!isGitHubTemplate) {
      // templateInfo.command is the npm package name (e.g. "create-vite", "@tanstack/cli")
      const packageExists = await checkNpmPackageExists(templateInfo.command);
      if (!packageExists) {
        if (!silent) {
          prompts.log.error(
            `Template "${templateInfo.command}" not found on npm. Run ${yellow('vp create --list')} to see available templates.`,
          );
        }
        return { exitCode: 1 };
      }
    }
    result = await runRemoteTemplateCommand(
      workspaceInfo,
      workspaceInfo.rootDir,
      templateInfo,
      true,
      silent,
    );
  }

  const exitCode = result.exitCode;
  // Provide troubleshooting tips
  if (exitCode === 127) {
    prompts.log.info(yellow('\nTroubleshooting:'));
    prompts.log.info(`  ${gray('•')} Command not found. Make sure Node.js is installed`);
    // prompts.log.info(`  ${gray('•')} Check if ${command} is available in PATH`);
  } else if (isGitHubTemplate && exitCode !== 0) {
    prompts.log.info(yellow('\nTroubleshooting:'));
    prompts.log.info(`  ${gray('•')} Make sure the GitHub repository exists`);
    prompts.log.info(`  ${gray('•')} Check your internet connection`);
    prompts.log.info(`  ${gray('•')} Repository might be private (requires authentication)`);
  }
  return result;
}

// Run a remote template command and support detect the created project directory
export async function runRemoteTemplateCommand(
  workspaceInfo: WorkspaceInfo,
  cwd: string,
  templateInfo: TemplateInfo,
  detectCreatedProjectDir?: boolean,
  silent = false,
): Promise<ExecutionWithProjectDir> {
  autoFixRemoteTemplateCommand(templateInfo, workspaceInfo);
  const remotePackageName = templateInfo.command;
  const execArgs = [...templateInfo.args];
  const envs = templateInfo.envs;
  const { command, args } = formatDlxCommand(remotePackageName, execArgs, workspaceInfo);
  if (!silent) {
    prompts.log.info(`Running: ${gray(`${command} ${args.join(' ')}`)}`);
  }
  if (detectCreatedProjectDir) {
    return await runCommandAndDetectProjectDir(
      { command, args, cwd, envs },
      templateInfo.parentDir,
    );
  }
  if (silent) {
    return await runCommandSilently({ command, args, cwd, envs });
  }
  return await runCommand({ command, args, cwd, envs });
}

function autoFixRemoteTemplateCommand(templateInfo: TemplateInfo, workspaceInfo: WorkspaceInfo) {
  // @tanstack/cli@latest, create-vite@latest
  let packageName = templateInfo.command;
  const indexOfAt = packageName.indexOf('@', 2);
  if (indexOfAt !== -1) {
    packageName = packageName.substring(0, indexOfAt);
  }
  if (packageName === 'create-vite') {
    // don't run dev server after installation
    // https://github.com/vitejs/vite/blob/main/packages/create-vite/src/index.ts#L46
    templateInfo.args.push('--no-immediate');
    // don't present rolldown option to users
    templateInfo.args.push('--no-rolldown');
  } else if (packageName === '@tanstack/cli') {
    // ensure create command is used
    if (templateInfo.args[0] !== 'create') {
      templateInfo.args.unshift('create');
    }
    // don't run npm install after project creation
    templateInfo.args.push('--no-install');
    // don't setup toolchain automatically
    templateInfo.args.push('--no-toolchain');
  } else if (packageName === 'sv') {
    // ensure create command is used
    if (templateInfo.args[0] !== 'create') {
      templateInfo.args.unshift('create');
    }
    // don't run npm install after project creation
    templateInfo.args.push('--no-install');
  }

  if (workspaceInfo.isMonorepo) {
    // don't run git init on monorepo
    if (packageName === 'create-nuxt') {
      templateInfo.args.push('--no-gitInit');
    } else if (packageName === '@tanstack/cli') {
      templateInfo.args.push('--no-git');
    }
  }
}

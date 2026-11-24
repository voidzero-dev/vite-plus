import { log } from '@voidzero-dev/vite-plus-prompts';
import colors from 'picocolors';

import type { WorkspaceInfo } from '../../types/index.ts';
import { getLocalVpCommand, runCommand, runCommandSilently } from '../../utils/command.ts';
import { checkNpmPackageExists } from '../../utils/package.ts';
import type { ExecutionWithProjectDir } from '../command.ts';
import { runCommandAndDetectProjectDir } from '../command.ts';
import type { TemplateInfo } from './types.ts';

const { gray, yellow } = colors;

export async function executeRemoteTemplate(
  workspaceInfo: WorkspaceInfo,
  templateInfo: TemplateInfo,
  options?: { silent?: boolean },
): Promise<ExecutionWithProjectDir> {
  const silent = options?.silent ?? false;
  if (!silent) {
    log.step('Generating project…');
  }

  const isGitHubTemplate = templateInfo.command === 'degit';
  let result: ExecutionWithProjectDir;
  if (templateInfo.command === 'node') {
    // Template found locally - execute directly
    const command = templateInfo.command;
    const args = templateInfo.args;
    const envs = templateInfo.envs;
    if (!silent) {
      log.info(`Running: ${gray(`${command} ${args.join(' ')}`)}`);
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
          log.error(
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
    log.info(yellow('\nTroubleshooting:'));
    log.info(`  ${gray('•')} Command not found. Make sure Node.js is installed`);
    // log.info(`  ${gray('•')} Check if ${command} is available in PATH`);
  } else if (isGitHubTemplate && exitCode !== 0) {
    log.info(yellow('\nTroubleshooting:'));
    log.info(`  ${gray('•')} Make sure the GitHub repository exists`);
    log.info(`  ${gray('•')} Check your internet connection`);
    log.info(`  ${gray('•')} Repository might be private (requires authentication)`);
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
  // `--` keeps the global CLI from reading a template's `--help` as its own.
  const dlxArgs = ['dlx', '--', templateInfo.command, ...templateInfo.args];
  // `vp dlx` detects the package manager from cwd and defaults to pnpm outside a project.
  // Pass the exact version so a tag such as `latest` is not resolved again from the new workspace's
  // registry, and keep an override's hash so `dlx` reuses the verified install create downloaded.
  const envs = {
    ...templateInfo.envs,
    VP_PACKAGE_MANAGER:
      workspaceInfo.packageManagerOverride ??
      `${workspaceInfo.packageManager}@${workspaceInfo.packageManagerVersion}`,
  };
  // The global `vp` may be an older release and would pick its own Node.js outside a project.
  const { command, args } = getLocalVpCommand(dlxArgs);
  if (!silent) {
    log.info(`Running: ${gray(`vp ${dlxArgs.join(' ')}`)}`);
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

export function autoFixRemoteTemplateCommand(
  templateInfo: TemplateInfo,
  workspaceInfo: WorkspaceInfo,
) {
  // @tanstack/cli@latest, create-vite@latest
  let packageName = templateInfo.command;
  const indexOfAt = packageName.indexOf('@', 2);
  if (indexOfAt !== -1) {
    packageName = packageName.slice(0, indexOfAt);
  }
  if (packageName === 'create-vite') {
    // don't run dev server after installation
    // https://github.com/vitejs/vite/blob/main/packages/create-vite/src/index.ts#L46
    // Don't present the Rolldown option.
    templateInfo.args.push('--no-immediate', '--no-rolldown');
  } else if (packageName === '@tanstack/cli') {
    // ensure create command is used
    if (templateInfo.args[0] !== 'create') {
      templateInfo.args.unshift('create');
    }
    // don't run npm install after project creation
    // Skip automatic toolchain setup.
    templateInfo.args.push('--no-install', '--no-toolchain');
  } else if (packageName === 'sv') {
    // ensure create command is used
    if (templateInfo.args[0] !== 'create') {
      templateInfo.args.unshift('create');
    }
    // don't run npm install after project creation
    templateInfo.args.push('--no-install');
  } else if (packageName === 'create-react-router') {
    // Vite+ owns repository initialization and hooks. create-react-router also
    // creates an initial commit, which can fail when Git identity is unset and
    // abort the rest of the Vite+ integration.
    templateInfo.args.push('--no-git-init');
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

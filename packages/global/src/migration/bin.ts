import path from 'node:path';

import * as prompts from '@clack/prompts';
import mri from 'mri';
import colors from 'picocolors';
import semver from 'semver';

import { PackageManager, type WorkspaceInfo } from '../types/index.js';
import {
  defaultInteractive,
  detectWorkspace,
  selectPackageManager,
  downloadPackageManager,
  runViteInstall,
  upgradeYarn,
  cancelAndExit,
  selectAgentTargetPath,
  writeAgentInstructions,
} from '../utils/index.js';
import { getVitePlusHeader } from '../utils/terminal.js';
import {
  checkVitestVersion,
  checkViteVersion,
  rewriteMonorepo,
  rewriteStandaloneProject,
} from './migrator.js';

const { green, gray } = colors;

// prettier-ignore
const helpMessage = `\
Usage: vite migration [PATH] [OPTIONS]

Migrate standalone vite, vitest, oxlint, and oxfmt to unified vite-plus.

Arguments:
  PATH                       Target directory to migrate (default: current directory)

Options:
  --agent NAME               Write agent instructions file into the project (e.g. chatgpt, claude, opencode).
  --no-agent                 Skip writing agent instructions file
  --no-interactive           Run in non-interactive mode (skip prompts and use defaults)
  -h, --help                 Show this help message

Examples:
  ${gray('# Migrate current package')}
  vite migration

  ${gray('# Migrate specific directory')}
  vite migration my-app

  ${gray('# Non-interactive mode')}
  vite migration --no-interactive

Aliases: ${gray('migrate')}
`;

export interface MigrationOptions {
  interactive: boolean;
  help?: boolean;
  agent?: string | false;
}

function parseArgs() {
  const args = process.argv.slice(3); // Skip 'node', 'vite', 'migration'

  const parsed = mri<{
    help?: boolean;
    interactive?: boolean;
    agent?: string | false;
  }>(args, {
    alias: { h: 'help' },
    boolean: ['help', 'interactive'],
    default: { interactive: defaultInteractive() },
  });

  let projectPath = parsed._[0] as string;
  if (projectPath) {
    projectPath = path.resolve(process.cwd(), projectPath);
  } else {
    projectPath = process.cwd();
  }

  return {
    projectPath,
    options: {
      interactive: parsed.interactive,
      help: parsed.help,
      agent: parsed.agent,
    } as MigrationOptions,
  };
}

async function main() {
  const { projectPath, options } = parseArgs();

  if (options.help) {
    console.log(helpMessage);
    return;
  }

  prompts.intro(await getVitePlusHeader());

  const workspaceInfoOptional = await detectWorkspace(projectPath);
  const packageManager =
    workspaceInfoOptional.packageManager ?? (await selectPackageManager(options.interactive));

  // ensure the package manager is installed by vite-plus
  const downloadResult = await downloadPackageManager(
    packageManager,
    workspaceInfoOptional.packageManagerVersion,
    options.interactive,
  );
  const workspaceInfo: WorkspaceInfo = {
    ...workspaceInfoOptional,
    packageManager,
    downloadPackageManager: downloadResult,
  };

  // support catalog require yarn@>=4.10.0 https://yarnpkg.com/features/catalogs
  // if `yarn<4.10.0 && yarn>=4.0.0`, upgrade yarn to stable version
  if (
    packageManager === PackageManager.yarn &&
    semver.satisfies(downloadResult.version, '>=4.0.0 <4.10.0')
  ) {
    await upgradeYarn(workspaceInfo.rootDir, options.interactive);
  } else if (
    packageManager === PackageManager.pnpm &&
    semver.satisfies(downloadResult.version, '< 9.5.0')
  ) {
    // required pnpm@>=9.5.0 to support catalog https://pnpm.io/9.x/catalogs
    prompts.log.error(
      `❌ pnpm@${downloadResult.version} is not supported by auto migration, please upgrade pnpm to >=9.5.0 first`,
    );
    cancelAndExit('The project is not supported by auto migration', 1);
  } else if (
    packageManager === PackageManager.npm &&
    semver.satisfies(downloadResult.version, '< 8.3.0')
  ) {
    // required npm@>=8.3.0 to support overrides https://github.com/npm/cli/releases/tag/v8.3.0
    prompts.log.error(
      `❌ npm@${downloadResult.version} is not supported by auto migration, please upgrade npm to >=8.3.0 first`,
    );
    cancelAndExit('The project is not supported by auto migration', 1);
  }

  // run vite install first to ensure the project is ready
  await runViteInstall(workspaceInfo.rootDir, options.interactive);
  // check vite and vitest version is supported by migration
  const isViteSupported = checkViteVersion(workspaceInfo.rootDir);
  const isVitestSupported = checkVitestVersion(workspaceInfo.rootDir);
  if (!isViteSupported || !isVitestSupported) {
    cancelAndExit('The project is not supported by auto migration', 1);
  }

  if (workspaceInfo.isMonorepo) {
    rewriteMonorepo(workspaceInfo);
  } else {
    rewriteStandaloneProject(workspaceInfo.rootDir, workspaceInfo);
  }

  const selectedAgentTargetPath = await selectAgentTargetPath({
    interactive: options.interactive,
    agent: options.agent,
    onCancel: () => cancelAndExit(),
  });

  await writeAgentInstructions({
    projectRoot: workspaceInfo.rootDir,
    targetPath: selectedAgentTargetPath,
    interactive: options.interactive,
  });

  // reinstall after migration
  await runViteInstall(workspaceInfo.rootDir, options.interactive);
  prompts.outro(green('✨ Migration completed!'));
}

main().catch((err) => {
  prompts.log.error(err.message);
  console.error(err);
  process.exit(1);
});

import path from 'node:path';

import * as prompts from '@clack/prompts';
import mri from 'mri';
import colors from 'picocolors';

import type { WorkspaceInfo } from '../types/index.ts';
import {
  defaultInteractive,
  detectWorkspace,
  selectPackageManager,
  downloadPackageManager,
  runViteInstall,
} from '../utils/index.ts';
import { rewriteMonorepo, rewriteStandaloneProject } from './migrator.ts';

const { cyan, green, gray } = colors;

// prettier-ignore
const helpMessage = `\
Usage: vite migration [PATH] [OPTIONS]

Migrate standalone vite, vitest, tsdown, oxlint, and oxfmt to unified vite-plus.

Arguments:
  PATH                       Target directory to migrate (default: current directory)

Options:
  --no-interactive           Run in non-interactive mode (skip prompts and use defaults)
  -h, --help                 Show this help message

Examples:
  ${gray('# Migrate current package')}
  vite migration

  ${gray('# Migrate specific directory')}
  vite migration path/to/my-app

  ${gray('# Non-interactive mode')}
  vite migration --no-interactive

Aliases: ${gray('migrate')}
`;

export interface MigrationOptions {
  interactive: boolean;
  help?: boolean;
}

function parseArgs() {
  const args = process.argv.slice(3); // Skip 'node', 'vite', 'migration'

  const parsed = mri<{
    help?: boolean;
    interactive?: boolean;
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
    } as MigrationOptions,
  };
}

async function main() {
  const { projectPath, options } = parseArgs();

  // Handle help flag
  if (options.help) {
    console.log(helpMessage);
    return;
  }

  // Start migration
  prompts.intro(cyan('Vite+ Migration'));

  const workspaceInfoOptional = await detectWorkspace(projectPath);
  // Prompt for package manager or use default
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

  if (workspaceInfo.isMonorepo) {
    rewriteMonorepo(workspaceInfo);
  } else {
    rewriteStandaloneProject(projectPath, workspaceInfo);
  }

  await runViteInstall(projectPath, options.interactive);

  prompts.outro(green('✨ Migration completed!'));
}

main().catch((err) => {
  prompts.log.error(err.message);
  console.error(err);
  process.exit(1);
});

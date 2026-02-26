import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import mri from 'mri';
import colors from 'picocolors';
import semver from 'semver';

import { PackageManager, type WorkspaceInfo } from '../types/index.js';
import { selectAgentTargetPath, writeAgentInstructions } from '../utils/agent.js';
import { hasVitePlusDependency, readNearestPackageJson } from '../utils/package.js';
import {
  cancelAndExit,
  defaultInteractive,
  downloadPackageManager,
  runViteInstall,
  selectPackageManager,
  upgradeYarn,
} from '../utils/prompts.js';
import { accent, getVitePlusHeader, headline, log, muted } from '../utils/terminal.js';
import type { PackageDependencies } from '../utils/types.js';
import { detectWorkspace } from '../utils/workspace.js';
import {
  checkVitestVersion,
  checkViteVersion,
  rewriteMonorepo,
  rewriteStandaloneProject,
} from './migrator.js';

const { green } = colors;

// prettier-ignore
const helpMessage = `\
${headline(`Usage:`)} ${styleText('bold', `vp migrate [PATH] [OPTIONS]`)}

Migrate standalone Vite, Vitest, Oxlint, and Oxfmt projects to unified Vite+.

${headline(`Arguments:`)}
  PATH                       Target directory to migrate (default: current directory)

${headline(`Options:`)}
  --agent NAME               Write agent instructions file into the project (e.g. chatgpt, claude, opencode).
  --no-agent                 Skip writing agent instructions file
  --no-interactive           Run in non-interactive mode (skip prompts and use defaults)
  --non-interactive          Alias for --no-interactive
  -h, --help                 Show this help message

${headline(`Examples:`)}
  ${muted('# Migrate current package')}
  ${accent(`vp migrate`)}

  ${muted('# Migrate specific directory')}
  ${accent(`vp migrate my-app`)}

  ${muted('# Non-interactive mode')}
  ${accent(`vp migrate --no-interactive`)}

`;

export interface MigrationOptions {
  interactive: boolean;
  help?: boolean;
  agent?: string | false;
}

function parseArgs() {
  const args = process.argv.slice(3); // Skip 'node', 'vite', 'migrate'

  const parsed = mri<{
    help?: boolean;
    interactive?: boolean;
    nonInteractive?: boolean;
    'non-interactive'?: boolean;
    agent?: string | false;
  }>(args, {
    alias: { h: 'help' },
    boolean: ['help', 'interactive', 'non-interactive', 'nonInteractive'],
    default: { interactive: defaultInteractive() },
  });
  const nonInteractive = parsed['non-interactive'] ?? parsed.nonInteractive;
  const interactive = nonInteractive ? false : parsed.interactive;

  let projectPath = parsed._[0];
  if (projectPath) {
    projectPath = path.resolve(process.cwd(), projectPath);
  } else {
    projectPath = process.cwd();
  }

  return {
    projectPath,
    options: {
      interactive,
      help: parsed.help,
      agent: parsed.agent,
    } as MigrationOptions,
  };
}

async function main() {
  const { projectPath, options } = parseArgs();

  if (options.help) {
    log((await getVitePlusHeader()) + '\n');
    log(helpMessage);
    return;
  }

  prompts.intro(await getVitePlusHeader());

  const workspaceInfoOptional = await detectWorkspace(projectPath);
  if (
    hasVitePlusDependency(
      readNearestPackageJson<PackageDependencies>(workspaceInfoOptional.rootDir),
    )
  ) {
    prompts.outro(`This project is already using Vite+! ${accent(`Happy coding!`)}`);
    return;
  }

  if (options.interactive) {
    prompts.log.info(
      [
        styleText('bold', 'Migration plan:'),
        '- Inspect workspace and package manager',
        `- Run ${accent('vp install')} to prepare dependencies`,
        '- Rewrite configs and dependencies for Vite+',
      ].join('\n'),
    );
    const approved = await prompts.confirm({
      message: 'Migrate this project to Vite+?',
      initialValue: true,
    });
    if (prompts.isCancel(approved) || !approved) {
      cancelAndExit('Migration cancelled');
    }
  }

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
      `✘ pnpm@${downloadResult.version} is not supported by auto migration, please upgrade pnpm to >=9.5.0 first`,
    );
    cancelAndExit('Vite+ cannot automatically migrate this project yet.', 1);
  } else if (
    packageManager === PackageManager.npm &&
    semver.satisfies(downloadResult.version, '< 8.3.0')
  ) {
    // required npm@>=8.3.0 to support overrides https://github.com/npm/cli/releases/tag/v8.3.0
    prompts.log.error(
      `✘ npm@${downloadResult.version} is not supported by auto migration, please upgrade npm to >=8.3.0 first`,
    );
    cancelAndExit('Vite+ cannot automatically migrate this project yet.', 1);
  }

  // run vp install first to ensure the project is ready
  await runViteInstall(workspaceInfo.rootDir, options.interactive);
  // check vite and vitest version is supported by migration
  const isViteSupported = checkViteVersion(workspaceInfo.rootDir);
  const isVitestSupported = checkVitestVersion(workspaceInfo.rootDir);
  if (!isViteSupported || !isVitestSupported) {
    cancelAndExit('Vite+ cannot automatically migrate this project yet.', 1);
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
  // npm needs --force to re-resolve packages with newly added overrides,
  // otherwise the stale lockfile prevents override resolution.
  const installArgs = packageManager === PackageManager.npm ? ['--force'] : undefined;
  await runViteInstall(workspaceInfo.rootDir, options.interactive, installArgs);
  prompts.outro(green('✔ Migration completed!'));
}

main().catch((err) => {
  prompts.log.error(err.message);
  console.error(err);
  process.exit(1);
});

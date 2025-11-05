import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@clack/prompts';
import { downloadPackageManager } from '@voidzero-dev/vite-plus/binding';
import mri from 'mri';
import colors from 'picocolors';

import { runCommandSilently } from './gen/command.ts';
import { discoverTemplate } from './gen/discovery.ts';
import { performAutoMigration } from './gen/migration.ts';
import { cancelAndExit, checkProjectDirExists, promptPackageNameAndTargetDir } from './gen/prompts.ts';
import { executeBuiltinTemplate, executeMonorepoTemplate, executeRemoteTemplate } from './gen/templates/index.ts';
import {
  BuiltinTemplate,
  DependencyType,
  type ExecutionResult,
  PackageManager,
  TemplateType,
  type ViteOptions,
  type WorkspaceInfo,
} from './gen/types.ts';
import { formatTargetDir, setPackageManager, templatesDir } from './gen/utils.ts';
import { detectWorkspace, updatePackageJsonWithDeps, updateWorkspaceConfig } from './gen/workspace.ts';

const { blue, cyan, green, gray, blueBright } = colors;

// prettier-ignore
const helpMessage = `\
Usage: vite gen [TEMPLATE] [OPTIONS] [-- TEMPLATE_OPTIONS]

Run any template (builtin, remote, or local) with automatic vite+ integration.

Arguments:
  TEMPLATE                   Template name (optional; you will be prompted if omitted)
                            - Built-in: vite:monorepo, vite:application, vite:library, vite:generator
                            - Remote: create-vite, @tanstack/create-start, create-next-app, create-nuxt,
                                      github:user/repo, https://github.com/user/template-repo
                            - Local: @company/generator-*, ./tools/create-ui-component

Options (before --):
  --directory DIR           Target directory for the generated project.
                            Only works for built-in templates; auto-detected for remote templates.
  --no-interactive          Run in non-interactive mode (skip prompts and use defaults)
  --list                    List all available templates
  -h, --help                Show this help message

Template options (after --):
  All arguments after -- are passed directly through to the template command.

Examples:
  ${gray('# Interactive mode - choose template via prompt')}
  vite gen

  ${gray('# Run any existing template (npx / pnpm dlx / yarn dlx / bunx auto-detected)')}
  vite gen create-vite
  vite gen create-next-app
  vite gen @tanstack/create-start

  ${gray('# With template-specific options (after --)')}
  vite gen create-vite -- --template react-ts
  vite gen create-next-app -- --typescript --app

  ${gray('# Create vite+ monorepo, application, library, or generator scaffolds')}
  vite gen vite:monorepo
  vite gen vite:application
  vite gen vite:library
  vite gen vite:generator

  ${gray('# Monorepo: specify target directory')}
  vite gen vite:application --directory=packages/my-app

  ${gray('# Combine gen command options and template options')}
  vite gen vite:application --directory=apps/my-app -- --template vue-ts

  ${gray('# Run templates from GitHub (via degit)')}
  vite gen github:user/repo
  vite gen https://github.com/user/template-repo

Note: Templates are executed via npx / pnpm dlx / yarn dlx / bunx,
      based on the detected package manager.
      No global installation required - always uses the latest version.

Aliases: ${gray('g, generate, new')}
`;

// Parse CLI arguments: split on '--' separator
function parseArgs() {
  const args = process.argv.slice(3); // Skip 'node', 'vite', 'gen'
  const separatorIndex = args.indexOf('--');

  // Arguments before -- are vite+ options
  const viteArgs = separatorIndex >= 0 ? args.slice(0, separatorIndex) : args;

  // Arguments after -- are template options
  const templateArgs = separatorIndex >= 0 ? args.slice(separatorIndex + 1) : [];

  const parsed = mri<{
    directory?: string;
    interactive?: boolean;
    list?: boolean;
    help?: boolean;
  }>(viteArgs, {
    alias: { h: 'help' },
    boolean: ['help', 'list', 'all', 'interactive'],
    string: ['directory'],
    default: { interactive: process.stdin.isTTY },
  });

  const templateName = parsed._[0] as string | undefined;

  return {
    templateName,
    viteOptions: {
      directory: parsed.directory,
      interactive: parsed.interactive,
      list: parsed.list || false,
      help: parsed.help || false,
    } as ViteOptions,
    templateArgs,
  };
}

async function main() {
  const { templateName, viteOptions, templateArgs } = parseArgs();

  // #region Handle help flag
  if (viteOptions.help) {
    console.log(helpMessage);
    return;
  }
  // #endregion

  // #region Handle list flag
  if (viteOptions.list) {
    showAvailableTemplates();
    return;
  }
  // #endregion

  // #region Handle required arguments
  if (!templateName && !viteOptions.interactive) {
    console.error(`
Template name is required when running in non-interactive mode

Usage: vite gen [TEMPLATE] [OPTIONS] [-- TEMPLATE_OPTIONS]

Example: 
  ${gray('# Create a new application in non-interactive mode with a custom target directory')}
  vite gen vite:application --no-interactive --directory=apps/my-app

Use \`vite gen --list\` to list all available templates, or run \`vite gen --help\` for more information.
`);
    process.exit(1);
  }
  // #endregion

  // #region Prepare Stage
  if (viteOptions.interactive) {
    const logo = fs.readFileSync(path.join(templatesDir, 'vite-plus-logo.txt'), 'utf-8');
    console.log(blueBright(logo));
  }
  prompts.intro(`${blueBright('Vite+')} - The Unified Toolchain for the Web`);

  // check --directory option is valid
  let targetDir = '';
  let packageName = '';
  if (viteOptions.directory) {
    const formatted = formatTargetDir(viteOptions.directory);
    if (formatted.error) {
      prompts.log.error(formatted.error);
      cancelAndExit('The --directory option is invalid', 1);
    }
    targetDir = formatted.directory;
    packageName = formatted.packageName;
  }

  const workspaceInfoOptional = await detectWorkspace(process.cwd());
  const isMonorepo = workspaceInfoOptional.isMonorepo;

  // Interactive mode: prompt for template if not provided
  let selectedTemplateName = templateName as string;
  let selectedTemplateArgs = [...templateArgs];

  if (!selectedTemplateName) {
    const templates: { label: string; value: string; hint: string }[] = [];
    if (isMonorepo) {
      // find local templates in the monorepo
      for (const pkg of workspaceInfoOptional.packages) {
        if (pkg.isTemplatePackage) {
          templates.push({
            label: pkg.name,
            value: pkg.name,
            hint: pkg.description ?? pkg.path,
          });
        }
      }
    } else {
      templates.push({
        label: 'Vite+ Monorepo',
        value: BuiltinTemplate.monorepo,
        hint: 'Create a new vite+ monorepo project',
      });
    }
    const template = await prompts.select({
      message: 'Which template would you like to use?',
      options: [
        ...templates,
        {
          label: 'Vite+ Application',
          value: BuiltinTemplate.application,
          hint: 'Create vite applications',
        },
        {
          label: 'Vite+ Library',
          value: BuiltinTemplate.library,
          hint: 'Create vite libraries',
        },
        // TODO: only support builtin generator template in monorepo for now
        ...(isMonorepo
          ? [{
            label: 'Vite+ Generator',
            value: BuiltinTemplate.generator,
            hint: 'Scaffold a new code generator',
          }]
          : []),
        {
          label: 'TanStack Start',
          value: '@tanstack/create-start@latest',
          hint: 'Create TanStack applications and libraries',
        },
        {
          label: 'Others',
          value: 'other',
          hint: 'Enter a custom template package name',
        },
      ],
    });

    if (prompts.isCancel(template)) {
      cancelAndExit();
    }

    // Handle custom template input
    if (template === 'other') {
      const customTemplate = await prompts.text({
        message: 'Enter the template package name:',
        placeholder: 'e.g., create-next-app, create-nuxt',
        validate: (value) => {
          if (!value || value.trim().length === 0) {
            return 'Template name is required';
          }
        },
      });

      if (prompts.isCancel(customTemplate)) {
        cancelAndExit();
      }

      selectedTemplateName = customTemplate;
    } else {
      selectedTemplateName = template;
    }
  }

  // Prompt for package manager or use default
  let packageManager: PackageManager = workspaceInfoOptional.packageManager as PackageManager;
  if (!packageManager) {
    if (viteOptions.interactive) {
      const selected = await prompts.select({
        message: 'Which package manager would you like to use?',
        options: [
          { value: PackageManager.pnpm, hint: 'recommended' },
          { value: PackageManager.yarn },
          { value: PackageManager.npm },
        ],
        initialValue: PackageManager.pnpm,
      });

      if (prompts.isCancel(selected)) {
        cancelAndExit();
      }

      packageManager = selected;
    } else {
      // --no-interactive: use pnpm as default
      packageManager = PackageManager.pnpm;
      prompts.log.info(`Using default package manager: ${cyan(packageManager)}`);
    }
  }

  // ensure the package manager is installed by vite-plus
  const spinner = prompts.spinner();
  spinner.start(`${packageManager}@${workspaceInfoOptional.packageManagerVersion} installing...`);
  const downloadResult = await downloadPackageManager({
    name: packageManager,
    version: workspaceInfoOptional.packageManagerVersion,
  });
  spinner.stop(`${packageManager}@${downloadResult.version} installed`);
  const workspaceInfo: WorkspaceInfo = {
    ...workspaceInfoOptional,
    packageManager,
    downloadPackageManager: downloadResult,
  };

  // Discover template
  const templateInfo = discoverTemplate(
    selectedTemplateName,
    selectedTemplateArgs,
    workspaceInfo,
    viteOptions.interactive,
  );

  // only for builtin templates
  if (targetDir) {
    if (templateInfo.type !== TemplateType.builtin) {
      cancelAndExit('The --directory option is only available for builtin templates', 1);
    }
    // reset auto detect parent directory
    templateInfo.parentDir = undefined;
  }

  // #endregion

  // #region Handle monorepo template
  if (templateInfo.command === BuiltinTemplate.monorepo) {
    // Validate: cannot create monorepo inside an existing monorepo
    if (isMonorepo) {
      prompts.log.info(
        'You are already in a monorepo workspace.\nUse a different template or run this command outside the monorepo',
      );
      cancelAndExit('Cannot create a monorepo inside an existing monorepo', 1);
    }

    if (!packageName) {
      const selected = await promptPackageNameAndTargetDir('vite-plus-monorepo', viteOptions.interactive);
      packageName = selected.packageName;
      targetDir = selected.targetDir;
    }

    prompts.log.info(`Target directory: ${cyan(targetDir)}`);
    await checkProjectDirExists(path.join(workspaceInfo.rootDir, targetDir), viteOptions.interactive);
    const result = await executeMonorepoTemplate(
      workspaceInfo,
      { ...templateInfo, packageName, targetDir },
      viteOptions.interactive,
    );
    if (result.exitCode !== 0) {
      cancelAndExit(`Failed to create monorepo, exit code: ${result.exitCode}`, result.exitCode);
    }

    await runViteInstall(path.join(workspaceInfo.rootDir, result.projectDir!));
    prompts.outro(green('✨ Generation completed!'));
    showNextSteps(result.projectDir!);
    return;
  }
  // #endregion

  // #region Handle single project template

  if (isMonorepo && viteOptions.interactive) {
    if (!targetDir) {
      // no custom target directory provided, prompt for parent directory
      let parentDir: string | undefined;
      if (workspaceInfo.parentDirs.length > 0) {
        const defaultParentDir = templateInfo.parentDir ?? workspaceInfo.parentDirs[0];
        const selected = await prompts.select({
          message: 'Where should the new package be added to the monorepo:',
          options: workspaceInfo.parentDirs.map((dir) => ({
            label: `${dir}/`,
            value: dir,
            hint: ``,
          })).concat([{
            label: 'other',
            value: 'other',
            hint: 'Enter a custom target directory',
          }]),
          initialValue: defaultParentDir,
        });

        if (prompts.isCancel(selected)) {
          cancelAndExit();
        }

        if (selected !== 'other') {
          parentDir = selected;
        }
      }

      if (!parentDir) {
        const customTargetDir = await prompts.text({
          message: 'Where should the new package be added to the monorepo:',
          placeholder: 'e.g., packages/',
          validate: (value) => {
            return formatTargetDir(value).error;
          },
        });

        if (prompts.isCancel(customTargetDir)) {
          cancelAndExit();
        }

        parentDir = customTargetDir;
      }

      templateInfo.parentDir = parentDir;
    }
  }

  let result: ExecutionResult;
  if (templateInfo.type === TemplateType.builtin) {
    // prompt for package name if not provided
    if (!targetDir) {
      let defaultPackageName = `vite-plus-${templateInfo.command.split(':')[1]}`;
      if (workspaceInfo.monorepoScope) {
        defaultPackageName = `${workspaceInfo.monorepoScope}/${defaultPackageName}`;
      }
      const selected = await promptPackageNameAndTargetDir(defaultPackageName, viteOptions.interactive);
      packageName = selected.packageName;
      targetDir = templateInfo.parentDir ? path.join(templateInfo.parentDir, selected.targetDir) : selected.targetDir;
    }
    await checkProjectDirExists(targetDir, viteOptions.interactive);
    prompts.log.info(`Target directory: ${cyan(targetDir)}`);
    result = await executeBuiltinTemplate(workspaceInfo, { ...templateInfo, packageName, targetDir });
  } else {
    result = await executeRemoteTemplate(
      workspaceInfo,
      templateInfo,
    );
  }

  if (result.exitCode !== 0) {
    process.exit(result.exitCode);
  }
  const projectDir = result.projectDir;
  if (!projectDir) {
    process.exit(0);
  }

  // Show detected project directory
  prompts.log.success(`Detected project directory: ${green(projectDir)}`);
  const fullPath = path.join(workspaceInfo.rootDir, projectDir);

  // Auto-migration to vite-plus
  await performAutoMigration(
    workspaceInfo,
    projectDir,
  );

  // Monorepo integration
  if (isMonorepo) {
    prompts.log.step('Monorepo integration...');

    if (workspaceInfo.packages.length > 0) {
      if (viteOptions.interactive) {
        const selectedDepTypeOptions = await prompts.multiselect({
          message: `Add workspace dependencies to the ${green(projectDir)}?`,
          options: [
            {
              value: DependencyType.dependencies,
            },
            {
              value: DependencyType.devDependencies,
            },
            {
              value: DependencyType.peerDependencies,
            },
            {
              value: DependencyType.optionalDependencies,
            },
          ],
          required: false,
        });

        let selectedDepTypes: DependencyType[] = [];
        if (!prompts.isCancel(selectedDepTypeOptions)) {
          selectedDepTypes = selectedDepTypeOptions;
        }

        for (const selectedDepType of selectedDepTypes) {
          const selected = await prompts.multiselect({
            message: `Which packages should be added as ${selectedDepType} to ${green(projectDir)}?`,
            // FIXME: ignore itself as dependency
            options: workspaceInfo.packages.map(pkg => ({
              value: pkg.name,
              label: pkg.path,
            })),
            required: false,
          });
          let selectedDeps: string[] = [];
          if (!prompts.isCancel(selected)) {
            selectedDeps = selected;
          }

          if (selectedDeps.length > 0) {
            // FIXME: should use `vite add` command instead
            updatePackageJsonWithDeps(workspaceInfo.rootDir, projectDir, selectedDeps, selectedDepType);
          }
        }
      }
    }

    updateWorkspaceConfig(projectDir, workspaceInfo);
    // install dependencies in the root of the monorepo
    await runViteInstall(workspaceInfo.rootDir);
  } else {
    // single project
    // set package manager in the project directory
    setPackageManager(fullPath, workspaceInfo.downloadPackageManager);
    // install dependencies in the project directory
    await runViteInstall(fullPath);
  }

  // Show comprehensive summary
  prompts.outro(green('✨ Generation completed!'));

  // Display summary
  console.log(`\n${blue('Summary:')}`);
  console.log(
    `  ${gray('•')} Template: ${cyan(selectedTemplateName)}`,
  );
  console.log(`  ${gray('•')} Created: ${green(projectDir)}`);

  // Show next steps
  showNextSteps(projectDir);
  // #endregion
}

function showNextSteps(projectDir: string) {
  console.log(`\n${gray('Next steps:')}`);
  console.log(`  ${cyan(`cd ${projectDir}`)}`);
  console.log(`  ${cyan('vite run dev')}`);
  console.log('');
}

function showAvailableTemplates() {
  console.log('');
  console.log(cyan('📦 Available Templates'));
  console.log('');

  console.log(blue('Vite+ Built-in Templates:'));
  console.log('  • vite:monorepo                 ' + gray('- Create a new monorepo'));
  console.log('  • vite:application              ' + gray('- Create a new application'));
  console.log('  • vite:library                  ' + gray('- Create a new library'));
  console.log('  • vite:generator                ' + gray('- Scaffold a new code generator'));
  console.log('');

  console.log(green('Popular Remote Templates:'));
  console.log('  • create-vite                   ' + gray('- Official Vite templates'));
  console.log('  • @tanstack/create-start        ' + gray('- TanStack applications'));
  console.log('  • create-next-app               ' + gray('- Next.js application'));
  console.log('  • create-nuxt                   ' + gray('- Nuxt application'));
  console.log('  • create-typescript-app         ' + gray('- TypeScript application'));
  console.log('  • create-react-router           ' + gray('- React Router application'));
  console.log('  • create-vue                    ' + gray('- Vue application'));

  console.log(
    '\n' +
      gray('Run ') +
      cyan('vite gen') +
      gray(' for interactive mode'),
  );
  console.log(
    gray('Run ') +
      cyan('vite gen <template>') +
      gray(' to use any template'),
  );
  console.log(
    gray('Run ') +
      cyan('vite gen <template> -- <options>') +
      gray(' to pass options to the template'),
  );

  console.log('');
  console.log('✨ Tip: You can use ANY npm template with vite gen!');
  console.log('');
}

async function runViteInstall(cwd: string) {
  // install dependencies on non-CI environment
  if (process.env.CI) {
    return;
  }

  const spinner = prompts.spinner();
  spinner.start(`Running vite install...`);
  const { exitCode, stderr, stdout } = await runCommandSilently({
    command: 'vite',
    args: ['install'],
    cwd,
    envs: process.env,
  });
  if (exitCode === 0) {
    spinner.stop(`Dependencies installed`);
  } else {
    spinner.stop(`Install failed`);
    prompts.log.info(stdout.toString());
    prompts.log.error(stderr.toString());
    prompts.log.info(`You may need to run it manually in ${cwd}`);
  }
}

main().catch((err) => {
  prompts.log.error(err.message);
  console.error(err);
  cancelAndExit(`Failed to generate code: ${err.message}`, 1);
});

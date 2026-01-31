import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@clack/prompts';
import mri from 'mri';

import {
  rewriteMonorepo,
  rewriteMonorepoProject,
  rewriteStandaloneProject,
} from '../migration/migrator.js';
import { DependencyType, type WorkspaceInfo } from '../types/index.js';
import {
  detectExistingAgentTargetPath,
  selectAgentTargetPath,
  writeAgentInstructions,
} from '../utils/agent.js';
import {
  defaultInteractive,
  downloadPackageManager,
  runViteInstall,
  selectPackageManager,
} from '../utils/prompts.js';
import { accent, getVitePlusHeader, headline, muted, log, success } from '../utils/terminal.js';
import {
  detectWorkspace,
  updatePackageJsonWithDeps,
  updateWorkspaceConfig,
} from '../utils/workspace.js';
import type { ExecutionResult } from './command.js';
import { discoverTemplate, inferParentDir } from './discovery.js';
import { cancelAndExit, checkProjectDirExists, promptPackageNameAndTargetDir } from './prompts.js';
import {
  executeBuiltinTemplate,
  executeMonorepoTemplate,
  executeRemoteTemplate,
} from './templates/index.js';
import { InitialMonorepoAppDir } from './templates/monorepo.js';
import { BuiltinTemplate, TemplateType } from './templates/types.js';
import { formatTargetDir } from './utils.js';

const helpMessage = `\
${headline(`Usage:`)} ${styleText('bold', `vp new [TEMPLATE] [OPTIONS] [-- TEMPLATE_OPTIONS]`)}

Use any builtin, local or remote template with Vite+.

${headline(`Arguments:`)}
  TEMPLATE            Template name. Run \`vp new --list\` to see available templates.
                      - Default: vite:monorepo, vite:application, vite:library, vite:generator
                      - Remote: create-vite, @tanstack/create-start, create-next-app,
                        create-nuxt, github:user/repo, https://github.com/user/template-repo, etc.
                      - Local: @company/generator-*, ./tools/create-ui-component

${headline(`Options:`)}
  --directory DIR     Target directory for the generated project.
  --agent NAME        Create an agent instructions file for the specified agent.
  --no-interactive    Run in non-interactive mode
  --list              List all available templates
  -h, --help          Show this help message

${headline(`Template options:`)}
  Any arguments after -- are passed directly to the template.

${headline(`Examples:`)}
  ${muted('# Interactive mode')}
  ${accent(`vp new`)}

  ${muted('# Use existing templates')}
  ${accent(`vp new create-vite`)}
  ${accent(`vp new create-next-app`)}
  ${accent(`vp new @tanstack/create-start`)}
  ${accent(`vp new create-vite -- --template react-ts`)}

  ${muted('# Create Vite+ monorepo, application, library, or generator scaffolds')}
  ${accent(`vp new vite:monorepo`)}
  ${accent(`vp new vite:application`)}
  ${accent(`vp new vite:library`)}
  ${accent(`vp new vite:generator`)}

  ${muted('# Use templates from GitHub (via degit)')}
  ${accent(`vp new github:user/repo`)}
  ${accent(`vp new https://github.com/user/template-repo`)}
`;

export interface Options {
  directory?: string;
  interactive: boolean;
  list: boolean;
  help: boolean;
  agent?: string;
}

// Parse CLI arguments: split on '--' separator
function parseArgs() {
  const args = process.argv.slice(3); // Skip 'node', 'vite'
  const separatorIndex = args.indexOf('--');

  // Arguments before -- are Vite+ options
  const viteArgs = separatorIndex >= 0 ? args.slice(0, separatorIndex) : args;

  // Arguments after -- are template options
  const templateArgs = separatorIndex >= 0 ? args.slice(separatorIndex + 1) : [];

  const parsed = mri<{
    directory?: string;
    interactive?: boolean;
    list?: boolean;
    help?: boolean;
    agent?: string;
  }>(viteArgs, {
    alias: { h: 'help' },
    boolean: ['help', 'list', 'all', 'interactive'],
    string: ['directory', 'agent'],
    default: { interactive: defaultInteractive() },
  });

  const templateName = parsed._[0] as string | undefined;

  return {
    templateName,
    options: {
      directory: parsed.directory,
      interactive: parsed.interactive,
      list: parsed.list || false,
      help: parsed.help || false,
      agent: parsed.agent,
    } as Options,
    templateArgs,
  };
}

async function main() {
  const { templateName, options, templateArgs } = parseArgs();

  // #region Handle help flag
  if (options.help) {
    log((await getVitePlusHeader()) + '\n');
    log(helpMessage);
    return;
  }
  // #endregion

  // #region Handle list flag
  if (options.list) {
    await showAvailableTemplates();
    return;
  }
  // #endregion

  // #region Handle required arguments
  if (!templateName && !options.interactive) {
    console.error(`
A template name is required when running in non-interactive mode

Usage: vp new [TEMPLATE] [OPTIONS] [-- TEMPLATE_OPTIONS]

Example: 
  ${muted('# Create a new application in non-interactive mode with a custom target directory')}
  vp new vite:application --no-interactive --directory=apps/my-app

Use \`vp new --list\` to list all available templates, or run \`vp new --help\` for more information.
`);
    process.exit(1);
  }
  // #endregion

  // #region Prepare Stage
  prompts.intro(await getVitePlusHeader());

  // check --directory option is valid
  let targetDir = '';
  let packageName = '';
  if (options.directory) {
    const formatted = formatTargetDir(options.directory);
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
  let selectedAgentTargetPath: string | undefined;
  let selectedParentDir: string | undefined;

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
        hint: 'Create a new Vite+ monorepo project',
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
          ? [
              {
                label: 'Vite+ Generator',
                value: BuiltinTemplate.generator,
                hint: 'Scaffold a new code generator',
              },
            ]
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

  const isBuiltinTemplate = selectedTemplateName.startsWith('vite:');
  if (targetDir && !isBuiltinTemplate) {
    cancelAndExit('The --directory option is only available for builtin templates', 1);
  }
  if (selectedTemplateName === BuiltinTemplate.monorepo && isMonorepo) {
    prompts.log.info(
      'You are already in a monorepo workspace.\nUse a different template or run this command outside the monorepo',
    );
    cancelAndExit('Cannot create a monorepo inside an existing monorepo', 1);
  }

  if (isMonorepo && options.interactive && !targetDir) {
    let parentDir: string | undefined;
    if (workspaceInfoOptional.parentDirs.length > 0) {
      const defaultParentDir =
        inferParentDir(selectedTemplateName, workspaceInfoOptional) ??
        workspaceInfoOptional.parentDirs[0];
      const selected = await prompts.select({
        message: 'Where should the new package be added to the monorepo:',
        options: workspaceInfoOptional.parentDirs
          .map((dir) => ({
            label: `${dir}/`,
            value: dir,
            hint: ``,
          }))
          .concat([
            {
              label: 'other',
              value: 'other',
              hint: 'Enter a custom target directory',
            },
          ]),
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

    selectedParentDir = parentDir;
  }
  if (isMonorepo && !options.interactive && !targetDir) {
    const inferredParentDir =
      inferParentDir(selectedTemplateName, workspaceInfoOptional) ??
      workspaceInfoOptional.parentDirs[0];
    selectedParentDir = inferredParentDir;
  }

  if (isBuiltinTemplate && !targetDir) {
    if (selectedTemplateName === BuiltinTemplate.monorepo) {
      const selected = await promptPackageNameAndTargetDir(
        'vite-plus-monorepo',
        options.interactive,
      );
      packageName = selected.packageName;
      targetDir = selected.targetDir;
    } else {
      let defaultPackageName = `vite-plus-${selectedTemplateName.split(':')[1]}`;
      if (workspaceInfoOptional.monorepoScope) {
        defaultPackageName = `${workspaceInfoOptional.monorepoScope}/${defaultPackageName}`;
      }
      const selected = await promptPackageNameAndTargetDir(defaultPackageName, options.interactive);
      packageName = selected.packageName;
      targetDir = selectedParentDir
        ? path.join(selectedParentDir, selected.targetDir)
        : selected.targetDir;
    }
  }

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

  const existingAgentTargetPath =
    options.agent || !options.interactive
      ? undefined
      : detectExistingAgentTargetPath(workspaceInfoOptional.rootDir);
  selectedAgentTargetPath =
    existingAgentTargetPath ??
    (await selectAgentTargetPath({
      interactive: options.interactive,
      agent: options.agent,
      onCancel: () => cancelAndExit(),
    }));

  // Discover template
  const templateInfo = discoverTemplate(
    selectedTemplateName,
    selectedTemplateArgs,
    workspaceInfo,
    options.interactive,
  );

  if (selectedParentDir) {
    templateInfo.parentDir = selectedParentDir;
  }

  // only for builtin templates
  if (targetDir) {
    // reset auto detect parent directory
    templateInfo.parentDir = undefined;
  }

  // #endregion

  // #region Handle monorepo template
  if (templateInfo.command === BuiltinTemplate.monorepo) {
    prompts.log.info(`Target directory: ${accent(targetDir)}`);
    await checkProjectDirExists(path.join(workspaceInfo.rootDir, targetDir), options.interactive);
    const result = await executeMonorepoTemplate(
      workspaceInfo,
      { ...templateInfo, packageName, targetDir },
      options.interactive,
    );
    const { projectDir } = result;
    if (result.exitCode !== 0 || !projectDir) {
      cancelAndExit(`Failed to create monorepo, exit code: ${result.exitCode}`, result.exitCode);
    }

    // rewrite monorepo to add vite-plus dependencies
    const fullPath = path.join(workspaceInfo.rootDir, projectDir);
    await writeAgentInstructions({
      projectRoot: fullPath,
      targetPath: selectedAgentTargetPath,
      interactive: options.interactive,
    });
    workspaceInfo.rootDir = fullPath;
    rewriteMonorepo(workspaceInfo);
    await runViteInstall(fullPath, options.interactive);
    prompts.outro(`✔ Created ${accent(projectDir)}!`);
    log(`${styleText('bold', 'Next steps:')}`);
    log(`  ${accent(`cd ${projectDir}`)}`);
    log(`  ${accent(`vp dev ${InitialMonorepoAppDir}`)}`);
    return;
  }
  // #endregion

  // #region Handle single project template

  let result: ExecutionResult;
  if (templateInfo.type === TemplateType.builtin) {
    // prompt for package name if not provided
    if (!targetDir) {
      let defaultPackageName = `vite-plus-${templateInfo.command.split(':')[1]}`;
      if (workspaceInfo.monorepoScope) {
        defaultPackageName = `${workspaceInfo.monorepoScope}/${defaultPackageName}`;
      }
      const selected = await promptPackageNameAndTargetDir(defaultPackageName, options.interactive);
      packageName = selected.packageName;
      targetDir = templateInfo.parentDir
        ? path.join(templateInfo.parentDir, selected.targetDir)
        : selected.targetDir;
    }
    await checkProjectDirExists(targetDir, options.interactive);
    prompts.log.info(`Target directory: ${accent(targetDir)}`);
    result = await executeBuiltinTemplate(workspaceInfo, {
      ...templateInfo,
      packageName,
      targetDir,
    });
  } else {
    result = await executeRemoteTemplate(workspaceInfo, templateInfo);
  }

  if (result.exitCode !== 0) {
    process.exit(result.exitCode);
  }
  const projectDir = result.projectDir;
  if (!projectDir) {
    process.exit(0);
  }

  prompts.log.success(`Project directory: ${accent(projectDir)}`);
  const fullPath = path.join(workspaceInfo.rootDir, projectDir);
  await writeAgentInstructions({
    projectRoot: fullPath,
    targetPath: selectedAgentTargetPath,
    interactive: options.interactive,
  });

  if (isMonorepo) {
    prompts.log.step('Monorepo integration...');
    rewriteMonorepoProject(fullPath, workspaceInfo.packageManager);

    if (workspaceInfo.packages.length > 0) {
      if (options.interactive) {
        const selectedDepTypeOptions = await prompts.multiselect({
          message: `Add workspace dependencies to ${accent(projectDir)}?`,
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
            message: `Which packages should be added as ${selectedDepType} to ${success(
              projectDir,
            )}?`,
            // FIXME: ignore itself as dependency
            options: workspaceInfo.packages.map((pkg) => ({
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
            // FIXME: should use `vp add` command instead
            updatePackageJsonWithDeps(
              workspaceInfo.rootDir,
              projectDir,
              selectedDeps,
              selectedDepType,
            );
          }
        }
      }
    }

    updateWorkspaceConfig(projectDir, workspaceInfo);
    await runViteInstall(workspaceInfo.rootDir, options.interactive);
  } else {
    rewriteStandaloneProject(fullPath, workspaceInfo);
    await runViteInstall(fullPath, options.interactive);
  }

  prompts.outro(`✔ Created ${accent(projectDir)}!`);

  showNextSteps(projectDir, isMonorepo);
  // #endregion
}

function showNextSteps(projectDir: string, isMonorepo: boolean) {
  log(`${styleText('bold', 'Next steps:')}`);
  if (isMonorepo) {
    log(`  ${accent(`vp dev ${projectDir}`)}`);
  } else {
    log(`  ${accent(`cd ${projectDir}`)}`);
    log(`  ${accent('vp dev')}`);
  }
  log('');
}

async function showAvailableTemplates() {
  log((await getVitePlusHeader()) + '\n');

  log(headline('Vite+ Built-in Templates:'));
  log(`  vite:monorepo            ${muted('Create a new monorepo')}`);
  log(`  vite:application         ${muted('Create a new application')}`);
  log(`  vite:library             ${muted('Create a new library')}`);
  log(`  vite:generator           ${muted('Scaffold a new code generator')}`);
  log('');
  log(headline('Popular Templates:'));
  log(`  create-vite              ${muted('Official Vite templates')}`);
  log(`  create-typescript-app    ${muted('TypeScript application')}`);
  log(`  @tanstack/create-start   ${muted('TanStack applications')}`);
  log(`  create-next-app          ${muted('Next.js application')}`);
  log(`  create-nuxt              ${muted('Nuxt application')}`);
  log(`  create-react-router      ${muted('React Router application')}`);
  log(`  create-vue               ${muted('Vue application')}`);
  log('');
  log(headline(`Examples:`));
  log(`  ${accent('vp new')} ${muted('# interactive mode')}`);
  log(`  ${accent('vp new <template>')} ${muted('# use any template')}`);
  log(`  ${accent('vp new <template> -- <options>')} ${muted('# pass options to the template')}\n`);
  log(`✨ Tip: You can use any npm template or git repo with ${accent('vp new')}!\n`);
}

main().catch((err) => {
  prompts.log.error(err.message);
  console.error(err);
  cancelAndExit(`Failed to generate code: ${err.message}`, 1);
});

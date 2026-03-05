import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import mri from 'mri';

import { vitePlusHeader } from '../../binding/index.js';
import {
  installGitHooks,
  rewriteMonorepo,
  rewriteMonorepoProject,
  rewriteStandaloneProject,
} from '../migration/migrator.js';
import { DependencyType, type WorkspaceInfo } from '../types/index.js';
import {
  detectExistingAgentTargetPath,
  selectAgentTargetPaths,
  writeAgentInstructions,
} from '../utils/agent.js';
import { detectExistingEditor, selectEditor, writeEditorConfigs } from '../utils/editor.js';
import { renderCliDoc } from '../utils/help.js';
import { displayRelative } from '../utils/path.js';
import {
  defaultInteractive,
  downloadPackageManager,
  promptGitHooks,
  runViteFmt,
  runViteInstall,
  selectPackageManager,
} from '../utils/prompts.js';
import { accent, muted, log, success } from '../utils/terminal.js';
import {
  detectWorkspace,
  updatePackageJsonWithDeps,
  updateWorkspaceConfig,
} from '../utils/workspace.js';
import type { ExecutionResult } from './command.js';
import { discoverTemplate, inferParentDir } from './discovery.js';
import { cancelAndExit, checkProjectDirExists, promptPackageNameAndTargetDir } from './prompts.js';
import { getRandomProjectName } from './random-name.js';
import {
  executeBuiltinTemplate,
  executeMonorepoTemplate,
  executeRemoteTemplate,
} from './templates/index.js';
import { InitialMonorepoAppDir } from './templates/monorepo.js';
import { BuiltinTemplate, TemplateType } from './templates/types.js';
import { formatDisplayTargetDir, formatTargetDir } from './utils.js';

const helpMessage = renderCliDoc({
  usage: 'vp create [TEMPLATE] [OPTIONS] [-- TEMPLATE_OPTIONS]',
  summary: 'Use any builtin, local or remote template with Vite+.',
  sections: [
    {
      title: 'Arguments',
      rows: [
        {
          label: 'TEMPLATE',
          description: [
            `Template name. Run \`${accent('vp create --list')}\` to see available templates.`,
            `- Default: ${accent('vite:monorepo')}, ${accent('vite:application')}, ${accent('vite:library')}, ${accent('vite:generator')}`,
            '- Remote: vite, @tanstack/start, create-next-app,',
            '  create-nuxt, github:user/repo, https://github.com/user/template-repo, etc.',
            '- Local: @company/generator-*, ./tools/create-ui-component',
          ],
        },
      ],
    },
    {
      title: 'Options',
      rows: [
        { label: '--directory DIR', description: 'Target directory for the generated project.' },
        {
          label: '--agent NAME',
          description: 'Create an agent instructions file for the specified agent.',
        },
        {
          label: '--editor NAME',
          description: 'Write editor config files for the specified editor.',
        },
        {
          label: '--hooks',
          description: 'Set up pre-commit hooks (default in non-interactive mode)',
        },
        { label: '--no-hooks', description: 'Skip pre-commit hooks setup' },
        { label: '--no-interactive', description: 'Run in non-interactive mode' },
        { label: '--list', description: 'List all available templates' },
        { label: '-h, --help', description: 'Show this help message' },
      ],
    },
    {
      title: 'Template Options',
      lines: ['  Any arguments after -- are passed directly to the template.'],
    },
    {
      title: 'Examples',
      lines: [
        `  ${muted('# Interactive mode')}`,
        `  ${accent('vp create')}`,
        '',
        `  ${muted('# Use existing templates (shorthand expands to create-* packages)')}`,
        `  ${accent('vp create vite')}`,
        `  ${accent('vp create @tanstack/start')}`,
        `  ${accent('vp create vite -- --template react-ts')}`,
        '',
        `  ${muted('# Full package names also work')}`,
        `  ${accent('vp create create-vite')}`,
        `  ${accent('vp create create-next-app')}`,
        '',
        `  ${muted('# Create Vite+ monorepo, application, library, or generator scaffolds')}`,
        `  ${accent('vp create vite:monorepo')}`,
        `  ${accent('vp create vite:application')}`,
        `  ${accent('vp create vite:library')}`,
        `  ${accent('vp create vite:generator')}`,
        '',
        `  ${muted('# Use templates from GitHub (via degit)')}`,
        `  ${accent('vp create github:user/repo')}`,
        `  ${accent('vp create https://github.com/user/template-repo')}`,
      ],
    },
  ],
});

const listTemplatesMessage = renderCliDoc({
  usage: 'vp create --list',
  summary: 'List available builtin and popular project templates.',
  sections: [
    {
      title: 'Vite+ Built-in Templates',
      rows: [
        { label: 'vite:monorepo', description: 'Create a new monorepo' },
        { label: 'vite:application', description: 'Create a new application' },
        { label: 'vite:library', description: 'Create a new library' },
        { label: 'vite:generator', description: 'Scaffold a new code generator' },
      ],
    },
    {
      title: 'Popular Templates (shorthand)',
      rows: [
        { label: 'vite', description: 'Official Vite templates (create-vite)' },
        {
          label: '@tanstack/start',
          description: 'TanStack applications (@tanstack/create-start)',
        },
        { label: 'next-app', description: 'Next.js application (create-next-app)' },
        { label: 'nuxt', description: 'Nuxt application (create-nuxt)' },
        { label: 'react-router', description: 'React Router application (create-react-router)' },
        { label: 'vue', description: 'Vue application (create-vue)' },
      ],
    },
    {
      title: 'Examples',
      lines: [
        `  ${accent('vp create')} ${muted('# interactive mode')}`,
        `  ${accent('vp create vite')} ${muted('# shorthand for create-vite')}`,
        `  ${accent('vp create @tanstack/start')} ${muted('# shorthand for @tanstack/create-start')}`,
        `  ${accent('vp create <template> -- <options>')} ${muted('# pass options to the template')}`,
      ],
    },
    {
      title: 'Tip',
      lines: [`  You can use any npm template or git repo with ${accent('vp create')}.`],
    },
  ],
});

export interface Options {
  directory?: string;
  interactive: boolean;
  list: boolean;
  help: boolean;
  agent?: string | string[] | false;
  editor?: string;
  hooks?: boolean;
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
    agent?: string | string[] | false;
    editor?: string;
    hooks?: boolean;
  }>(viteArgs, {
    alias: { h: 'help' },
    boolean: ['help', 'list', 'all', 'interactive', 'hooks'],
    string: ['directory', 'agent', 'editor'],
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
      editor: parsed.editor,
      hooks: parsed.hooks,
    } as Options,
    templateArgs,
  };
}

async function main() {
  const { templateName, options, templateArgs } = parseArgs();

  // #region Handle help flag
  if (options.help) {
    log(vitePlusHeader() + '\n');
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

Usage: vp create [TEMPLATE] [OPTIONS] [-- TEMPLATE_OPTIONS]

Example:
  ${muted('# Create a new application in non-interactive mode with a custom target directory')}
  vp create vite:application --no-interactive --directory=apps/my-app

Use \`vp create --list\` to list all available templates, or run \`vp create --help\` for more information.
`);
    process.exit(1);
  }
  // #endregion

  // #region Prepare Stage
  prompts.intro(vitePlusHeader());

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

  const cwd = process.cwd();
  const workspaceInfoOptional = await detectWorkspace(cwd);
  const isMonorepo = workspaceInfoOptional.isMonorepo;

  // For non-monorepo, always use cwd as rootDir.
  // detectWorkspace walks up to find the nearest package.json, but for `vp create`
  // in standalone mode, the project should be created relative to where the user is.
  if (!isMonorepo) {
    workspaceInfoOptional.rootDir = cwd;
  }
  const cwdRelativeToRoot =
    isMonorepo && workspaceInfoOptional.rootDir !== cwd
      ? displayRelative(cwd, workspaceInfoOptional.rootDir)
      : '';
  const isInSubdirectory = cwdRelativeToRoot !== '';
  const cwdUnderParentDir = isInSubdirectory
    ? workspaceInfoOptional.parentDirs.some(
        (dir) => cwdRelativeToRoot === dir || cwdRelativeToRoot.startsWith(`${dir}/`),
      )
    : true;
  const shouldOfferCwdOption = isInSubdirectory && !cwdUnderParentDir;

  // Interactive mode: prompt for template if not provided
  let selectedTemplateName = templateName as string;
  let selectedTemplateArgs = [...templateArgs];
  let selectedAgentTargetPaths: string[] | undefined;
  let selectedEditor: Awaited<ReturnType<typeof selectEditor>>;
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
      message: '',
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

  if (isInSubdirectory) {
    prompts.log.info(`Detected monorepo root at ${accent(workspaceInfoOptional.rootDir)}`);
  }

  if (isMonorepo && options.interactive && !targetDir) {
    let parentDir: string | undefined;
    const hasParentDirs = workspaceInfoOptional.parentDirs.length > 0;

    if (hasParentDirs || isInSubdirectory) {
      const dirOptions: { label: string; value: string; hint: string }[] =
        workspaceInfoOptional.parentDirs.map((dir) => ({
          label: `${dir}/`,
          value: dir,
          hint: '',
        }));

      if (shouldOfferCwdOption) {
        dirOptions.push({
          label: `${cwdRelativeToRoot}/ (current directory)`,
          value: cwdRelativeToRoot,
          hint: '',
        });
      }

      dirOptions.push({
        label: 'other directory',
        value: 'other',
        hint: 'Enter a custom target directory',
      });

      const defaultParentDir = shouldOfferCwdOption
        ? cwdRelativeToRoot
        : (inferParentDir(selectedTemplateName, workspaceInfoOptional) ??
          workspaceInfoOptional.parentDirs[0]);

      const selected = await prompts.select({
        message: 'Where should the new package be added to the monorepo:',
        options: dirOptions,
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
          return value ? formatTargetDir(value).error : 'Target directory is required';
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
    if (isInSubdirectory) {
      prompts.log.info(`Use ${accent('--directory')} to specify a different target location.`);
    }
    const inferredParentDir =
      inferParentDir(selectedTemplateName, workspaceInfoOptional) ??
      workspaceInfoOptional.parentDirs[0];
    selectedParentDir = inferredParentDir;
  }

  if (isBuiltinTemplate && !targetDir) {
    if (selectedTemplateName === BuiltinTemplate.monorepo) {
      const selected = await promptPackageNameAndTargetDir(
        getRandomProjectName({ fallbackName: 'vite-plus-monorepo' }),
        options.interactive,
      );
      packageName = selected.packageName;
      targetDir = selected.targetDir;
    } else {
      const defaultPackageName = getRandomProjectName({
        scope: workspaceInfoOptional.monorepoScope,
        fallbackName: `vite-plus-${selectedTemplateName.split(':')[1]}`,
      });
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
    options.agent !== undefined || !options.interactive
      ? undefined
      : detectExistingAgentTargetPath(workspaceInfoOptional.rootDir);
  selectedAgentTargetPaths =
    existingAgentTargetPath !== undefined
      ? [existingAgentTargetPath]
      : await selectAgentTargetPaths({
          interactive: options.interactive,
          agent: options.agent,
          onCancel: () => cancelAndExit(),
        });

  const existingEditor =
    options.editor || !options.interactive
      ? undefined
      : detectExistingEditor(workspaceInfoOptional.rootDir);
  selectedEditor =
    existingEditor ??
    (await selectEditor({
      interactive: options.interactive,
      editor: options.editor,
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
      targetPaths: selectedAgentTargetPaths,
      interactive: options.interactive,
    });
    await writeEditorConfigs({
      projectRoot: fullPath,
      editorId: selectedEditor,
      interactive: options.interactive,
    });
    workspaceInfo.rootDir = fullPath;
    rewriteMonorepo(workspaceInfo);
    const shouldSetupHooks = await promptGitHooks(options);
    if (shouldSetupHooks) {
      installGitHooks(fullPath);
    }
    await runViteInstall(fullPath, options.interactive);
    await runViteFmt(fullPath, options.interactive);
    prompts.outro(`✔ Created ${accent(projectDir)}!`);
    log(styleText('bold', 'Next steps:'));
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
      const defaultPackageName = getRandomProjectName({
        scope: workspaceInfo.monorepoScope,
        fallbackName: `vite-plus-${templateInfo.command.split(':')[1]}`,
      });
      const selected = await promptPackageNameAndTargetDir(defaultPackageName, options.interactive);
      packageName = selected.packageName;
      targetDir = templateInfo.parentDir
        ? path.join(templateInfo.parentDir, selected.targetDir)
        : selected.targetDir;
    }
    await checkProjectDirExists(targetDir, options.interactive);
    prompts.log.info(`Target directory: ${accent(formatDisplayTargetDir(targetDir))}`);
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
    targetPaths: selectedAgentTargetPaths,
    interactive: options.interactive,
  });
  await writeEditorConfigs({
    projectRoot: fullPath,
    editorId: selectedEditor,
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
    await runViteFmt(workspaceInfo.rootDir, options.interactive, [projectDir]);
  } else {
    rewriteStandaloneProject(fullPath, workspaceInfo);
    const shouldSetupHooks = await promptGitHooks(options);
    if (shouldSetupHooks) {
      installGitHooks(fullPath);
    }
    await runViteInstall(fullPath, options.interactive);
    await runViteFmt(fullPath, options.interactive);
  }

  prompts.outro(`✔ Created ${accent(projectDir)}!`);

  showNextSteps(projectDir, isMonorepo);
  // #endregion
}

function showNextSteps(projectDir: string, isMonorepo: boolean) {
  log(styleText('bold', 'Next steps:'));
  if (isMonorepo) {
    log(`  ${accent(`vp dev ${projectDir}`)}`);
  } else {
    log(`  ${accent(`cd ${projectDir}`)}`);
    log(`  ${accent('vp dev')}`);
  }
  log('');
}

async function showAvailableTemplates() {
  log(vitePlusHeader() + '\n');
  log(listTemplatesMessage);
}

main().catch((err) => {
  prompts.log.error(err.message);
  console.error(err);
  cancelAndExit(`Failed to generate code: ${err.message}`, 1);
});

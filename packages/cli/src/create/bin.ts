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
} from '../migration/migrator.ts';
import { DependencyType, PackageManager, type WorkspaceInfo } from '../types/index.ts';
import {
  detectExistingAgentTargetPaths,
  selectAgentTargetPaths,
  writeAgentInstructions,
} from '../utils/agent.ts';
import { detectExistingEditor, selectEditor, writeEditorConfigs } from '../utils/editor.ts';
import { renderCliDoc } from '../utils/help.ts';
import { displayRelative } from '../utils/path.ts';
import {
  type CommandRunSummary,
  defaultInteractive,
  downloadPackageManager,
  promptGitHooks,
  runViteFmt,
  runViteInstall,
  selectPackageManager,
} from '../utils/prompts.ts';
import { accent, muted, log, success } from '../utils/terminal.ts';
import {
  detectWorkspace,
  updatePackageJsonWithDeps,
  updateWorkspaceConfig,
} from '../utils/workspace.ts';
import type { ExecutionResult } from './command.ts';
import { discoverTemplate, inferGitHubRepoName, inferParentDir, isGitHubUrl } from './discovery.ts';
import { getInitialTemplateOptions } from './initial-template-options.ts';
import {
  cancelAndExit,
  checkProjectDirExists,
  promptPackageNameAndTargetDir,
  promptTargetDir,
  suggestAvailableTargetDir,
} from './prompts.ts';
import { getRandomProjectName } from './random-name.ts';
import {
  executeBuiltinTemplate,
  executeMonorepoTemplate,
  executeRemoteTemplate,
} from './templates/index.ts';
import { BuiltinTemplate, TemplateType } from './templates/types.ts';
import { deriveDefaultPackageName, formatTargetDir } from './utils.ts';

const helpMessage = renderCliDoc({
  usage: 'vp create [TEMPLATE] [OPTIONS] [-- TEMPLATE_OPTIONS]',
  summary: 'Use any builtin, local or remote template with Vite+.',
  documentationUrl: 'https://viteplus.dev/guide/create',
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
          description: 'Write coding agent instructions to AGENTS.md, CLAUDE.md, etc.',
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
        {
          label: '--package-manager NAME',
          description: 'Use specified package manager (pnpm, npm, yarn, bun)',
        },
        { label: '--verbose', description: 'Show detailed scaffolding output' },
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
        `  ${accent('vp create svelte')}`,
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
  documentationUrl: 'https://viteplus.dev/guide/create',
  sections: [
    {
      title: 'Vite+ Built-in Templates',
      rows: [
        { label: 'vite:monorepo', description: 'Create a new monorepo' },
        { label: 'vite:application', description: 'Create a new application' },
        { label: 'vite:library', description: 'Create a new library' },
        { label: 'vite:generator', description: 'Scaffold a new code generator (monorepo only)' },
      ],
    },
    {
      title: 'Popular Templates (shorthand)',
      rows: [
        { label: 'vite', description: 'Official Vite templates (create-vite)' },
        {
          label: '@tanstack/start',
          description: 'TanStack applications (@tanstack/cli create)',
        },
        { label: 'next-app', description: 'Next.js application (create-next-app)' },
        { label: 'nuxt', description: 'Nuxt application (create-nuxt)' },
        { label: 'react-router', description: 'React Router application (create-react-router)' },
        { label: 'svelte', description: 'Svelte application (sv create)' },
        { label: 'vue', description: 'Vue application (create-vue)' },
      ],
    },
    {
      title: 'Examples',
      lines: [
        `  ${accent('vp create')} ${muted('# interactive mode')}`,
        `  ${accent('vp create vite')} ${muted('# shorthand for create-vite')}`,
        `  ${accent('vp create @tanstack/start')} ${muted('# shorthand for @tanstack/cli create')}`,
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
  verbose: boolean;
  agent?: string | string[] | false;
  editor?: string;
  hooks?: boolean;
  packageManager?: string;
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
    verbose?: boolean;
    agent?: string | string[] | false;
    editor?: string;
    hooks?: boolean;
    'package-manager'?: string;
  }>(viteArgs, {
    alias: { h: 'help' },
    boolean: ['help', 'list', 'all', 'interactive', 'hooks', 'verbose'],
    string: ['directory', 'agent', 'editor', 'package-manager'],
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
      verbose: parsed.verbose || false,
      agent: parsed.agent,
      editor: parsed.editor,
      hooks: parsed.hooks,
      packageManager: parsed['package-manager'],
    } as Options,
    templateArgs,
  };
}

function describeScaffold(templateName: string, templateArgs: string[]) {
  if (templateName === BuiltinTemplate.monorepo) {
    return 'Vite+ monorepo';
  }
  if (templateName === BuiltinTemplate.generator) {
    return 'generator scaffold';
  }
  if (templateName === BuiltinTemplate.library) {
    return 'TypeScript library';
  }

  const selectedTemplate = getTemplateOption(templateArgs);
  if (selectedTemplate) {
    return formatTemplateName(selectedTemplate);
  }

  if (templateName === BuiltinTemplate.application) {
    return 'Vite application';
  }

  return undefined;
}

function getTemplateOption(args: string[]) {
  for (let index = 0; index < args.length; index++) {
    const arg = args[index];
    if (arg === '--template' || arg === '-t') {
      return args[index + 1];
    }
    if (arg.startsWith('--template=')) {
      return arg.slice('--template='.length);
    }
  }
  return undefined;
}

function hasExplicitTargetDir(args: string[]) {
  return args[0] !== undefined && !args[0].startsWith('-');
}

function formatTemplateName(templateName: string) {
  const templateAliases: Record<string, string> = {
    lit: 'Lit',
    preact: 'Preact',
    react: 'React',
    'react-router': 'React Router',
    solid: 'Solid',
    svelte: 'Svelte',
    vanilla: 'Vanilla',
    vue: 'Vue',
  };
  const isTypeScript = templateName.endsWith('-ts');
  const baseName = isTypeScript ? templateName.slice(0, -3) : templateName;
  const frameworkName =
    templateAliases[baseName] ??
    baseName
      .split(/[-_]/)
      .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
      .join(' ');

  return `${frameworkName} + ${isTypeScript ? 'TypeScript' : 'JavaScript'}`;
}

function formatDuration(durationMs: number) {
  if (durationMs < 1000) {
    return `${Math.max(1, durationMs)}ms`;
  }
  const durationSeconds = durationMs / 1000;
  if (durationSeconds < 10) {
    return `${durationSeconds.toFixed(1)}s`;
  }
  return `${Math.round(durationSeconds)}s`;
}

function getNextCommand(projectDir: string, command: string) {
  if (!projectDir || projectDir === '.') {
    return command;
  }
  return `cd ${projectDir} && ${command}`;
}

function showCreateSummary(options: {
  description?: string;
  installSummary?: CommandRunSummary;
  nextCommand: string;
  packageManager: string;
  packageManagerVersion: string;
  projectDir: string;
}) {
  const {
    description,
    installSummary,
    nextCommand,
    packageManager,
    packageManagerVersion,
    projectDir,
  } = options;

  log(
    `${styleText('magenta', '◇')} Scaffolded ${accent(projectDir)}${
      description ? ` with ${description}` : ''
    }`,
  );
  log(
    `${styleText('gray', '•')} Node ${process.versions.node}  ${packageManager} ${packageManagerVersion}`,
  );
  if (installSummary?.status === 'installed') {
    log(
      `${styleText('green', '✓')} Dependencies installed in ${formatDuration(
        installSummary.durationMs,
      )}`,
    );
  }
  log(`${styleText('blue', '→')} Next: ${accent(nextCommand)}`);
}

async function main() {
  const { templateName, options, templateArgs } = parseArgs();
  let compactOutput = !options.verbose;

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
  if (options.interactive) {
    prompts.intro(vitePlusHeader());
  }

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
  let remoteTargetDir: string | undefined;
  let shouldSetupHooks = false;
  const installArgs = process.env.CI ? ['--no-frozen-lockfile'] : undefined;

  if (!selectedTemplateName) {
    const template = await prompts.select({
      message: '',
      options: getInitialTemplateOptions(isMonorepo),
    });

    if (prompts.isCancel(template)) {
      cancelAndExit();
    }

    selectedTemplateName = template;
  }

  const isBuiltinTemplate = selectedTemplateName.startsWith('vite:');

  // Remote templates (e.g., @tanstack/cli, custom templates) run their own
  // interactive CLI, so verbose mode is needed to show their output.
  if (!isBuiltinTemplate) {
    compactOutput = false;
  }

  if (targetDir && !isBuiltinTemplate) {
    cancelAndExit('The --directory option is only available for builtin templates', 1);
  }
  if (selectedTemplateName === BuiltinTemplate.monorepo && isMonorepo) {
    prompts.log.info(
      'You are already in a monorepo workspace.\nUse a different template or run this command outside the monorepo',
    );
    cancelAndExit('Cannot create a monorepo inside an existing monorepo', 1);
  }
  if (selectedTemplateName === BuiltinTemplate.generator && !isMonorepo) {
    prompts.log.info(
      'The vite:generator template requires a monorepo workspace.\nRun this command inside a Vite+ monorepo, or create one first with `vp create vite:monorepo`',
    );
    cancelAndExit('Cannot create a generator outside a monorepo', 1);
  }

  if (isInSubdirectory && !compactOutput) {
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
    if (isInSubdirectory && !compactOutput) {
      prompts.log.info(`Use ${accent('--directory')} to specify a different target location.`);
    }
    const inferredParentDir =
      inferParentDir(selectedTemplateName, workspaceInfoOptional) ??
      workspaceInfoOptional.parentDirs[0];
    selectedParentDir = inferredParentDir;
  }

  if (isGitHubUrl(selectedTemplateName)) {
    if (hasExplicitTargetDir(selectedTemplateArgs)) {
      remoteTargetDir = selectedTemplateArgs[0];
    } else {
      const inferredTargetDir = inferGitHubRepoName(selectedTemplateName) ?? 'template';
      const remoteTargetBaseDir = selectedParentDir
        ? path.join(workspaceInfoOptional.rootDir, selectedParentDir)
        : workspaceInfoOptional.rootDir;
      const defaultTargetDir = suggestAvailableTargetDir(inferredTargetDir, remoteTargetBaseDir);
      if (defaultTargetDir !== inferredTargetDir && options.interactive) {
        prompts.log.info(
          `  Target directory "${inferredTargetDir}" already exists. Suggested: ${accent(defaultTargetDir)}`,
        );
      }
      remoteTargetDir = await promptTargetDir(defaultTargetDir, options.interactive, {
        cwd: remoteTargetBaseDir,
      });
      selectedTemplateArgs = [remoteTargetDir, ...selectedTemplateArgs];
    }
  }

  if (isBuiltinTemplate && (!targetDir || targetDir === '.')) {
    if (targetDir === '.') {
      // Current directory: auto-derive package name from cwd, no prompt
      const fallbackName =
        selectedTemplateName === BuiltinTemplate.monorepo
          ? 'vite-plus-monorepo'
          : `vite-plus-${selectedTemplateName.split(':')[1]}`;
      packageName = deriveDefaultPackageName(
        cwd,
        workspaceInfoOptional.monorepoScope,
        fallbackName,
      );
      if (isMonorepo) {
        if (!cwdRelativeToRoot) {
          // At monorepo root: scaffolding here would overwrite the entire workspace
          cancelAndExit(
            'Cannot scaffold into the monorepo root directory. Use --directory to specify a target directory',
            1,
          );
        }
        // Check if cwd is inside an existing workspace package
        const enclosingPackage = workspaceInfoOptional.packages.find(
          (pkg) => cwdRelativeToRoot === pkg.path || cwdRelativeToRoot.startsWith(`${pkg.path}/`),
        );
        if (enclosingPackage) {
          cancelAndExit(
            `Cannot scaffold inside existing package "${enclosingPackage.name}" (${enclosingPackage.path}). Use --directory to specify a different location`,
            1,
          );
        }
        // Resolve '.' to the path relative to rootDir
        // so that scaffolding happens in cwd, not at the workspace root
        targetDir = cwdRelativeToRoot;
      }
      prompts.log.info(`Using package name: ${accent(packageName)}`);
    } else if (selectedTemplateName === BuiltinTemplate.monorepo) {
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
        ? path.join(selectedParentDir, selected.targetDir).split(path.sep).join('/')
        : selected.targetDir;
    }
  }

  // Resolve package manager: workspace detection > CLI flag > interactive prompt/default
  if (
    options.packageManager &&
    !Object.values(PackageManager).includes(options.packageManager as PackageManager)
  ) {
    const valid = Object.values(PackageManager).join(', ');
    prompts.log.error(
      `Invalid package manager: ${options.packageManager}. Must be one of: ${valid}`,
    );
    cancelAndExit('Invalid --package-manager value', 1);
  }
  const packageManager =
    workspaceInfoOptional.packageManager ??
    (options.packageManager as PackageManager | undefined) ??
    (await selectPackageManager(options.interactive, compactOutput));
  const shouldSilencePackageManagerInstallLog =
    compactOutput || (isMonorepo && workspaceInfoOptional.packageManager !== undefined);
  // ensure the package manager is installed by vite-plus
  const downloadResult = await downloadPackageManager(
    packageManager,
    workspaceInfoOptional.packageManagerVersion,
    options.interactive,
    shouldSilencePackageManagerInstallLog,
  );
  const workspaceInfo: WorkspaceInfo = {
    ...workspaceInfoOptional,
    packageManager,
    downloadPackageManager: downloadResult,
  };

  const existingAgentTargetPaths =
    options.agent !== undefined || !options.interactive
      ? undefined
      : detectExistingAgentTargetPaths(workspaceInfoOptional.rootDir);
  selectedAgentTargetPaths =
    existingAgentTargetPaths !== undefined
      ? existingAgentTargetPaths
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

  if (!isMonorepo) {
    shouldSetupHooks = await promptGitHooks(options);
  }

  const createProgress =
    options.interactive && compactOutput ? prompts.spinner({ indicator: 'timer' }) : undefined;
  let createProgressStarted = false;
  let createProgressMessage = 'Scaffolding project';
  const updateCreateProgress = (message: string) => {
    createProgressMessage = message;
    if (!createProgress) {
      return;
    }
    if (createProgressStarted) {
      createProgress.message(message);
      return;
    }
    createProgress.start(message);
    createProgressStarted = true;
  };
  const clearCreateProgress = () => {
    if (createProgress && createProgressStarted) {
      createProgress.clear();
      createProgressStarted = false;
    }
  };
  const failCreateProgress = (message: string) => {
    if (createProgress && createProgressStarted) {
      createProgress.error(message);
      createProgressStarted = false;
    }
  };
  const pauseCreateProgress = () => {
    if (createProgress && createProgressStarted) {
      createProgress.pause();
      createProgressStarted = false;
    }
  };
  const resumeCreateProgress = () => {
    if (createProgress && !createProgressStarted) {
      createProgress.resume(createProgressMessage);
      createProgressStarted = true;
    }
  };
  updateCreateProgress('Scaffolding project');

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

  if (remoteTargetDir) {
    const projectDir = templateInfo.parentDir
      ? path.join(templateInfo.parentDir, remoteTargetDir)
      : remoteTargetDir;
    pauseCreateProgress();
    await checkProjectDirExists(path.join(workspaceInfo.rootDir, projectDir), options.interactive);
    resumeCreateProgress();
  }

  // #endregion

  // #region Handle monorepo template
  if (templateInfo.command === BuiltinTemplate.monorepo) {
    updateCreateProgress('Creating monorepo');
    await checkProjectDirExists(path.join(workspaceInfo.rootDir, targetDir), options.interactive);
    const result = await executeMonorepoTemplate(
      workspaceInfo,
      { ...templateInfo, packageName, targetDir },
      options.interactive,
      { silent: compactOutput },
    );
    const { projectDir } = result;
    if (result.exitCode !== 0 || !projectDir) {
      failCreateProgress('Scaffolding failed');
      cancelAndExit(`Failed to create monorepo, exit code: ${result.exitCode}`, result.exitCode);
    }

    // rewrite monorepo to add vite-plus dependencies
    const fullPath = path.join(workspaceInfo.rootDir, projectDir);
    updateCreateProgress('Writing agent instructions');
    pauseCreateProgress();
    await writeAgentInstructions({
      projectRoot: fullPath,
      targetPaths: selectedAgentTargetPaths,
      interactive: options.interactive,
      silent: compactOutput,
    });
    resumeCreateProgress();
    updateCreateProgress('Writing editor configs');
    pauseCreateProgress();
    await writeEditorConfigs({
      projectRoot: fullPath,
      editorId: selectedEditor,
      interactive: options.interactive,
      silent: compactOutput,
      extraVsCodeSettings: { 'npm.scriptRunner': 'vp' },
    });
    resumeCreateProgress();
    workspaceInfo.rootDir = fullPath;
    updateCreateProgress('Integrating monorepo');
    rewriteMonorepo(workspaceInfo, undefined, compactOutput);
    if (shouldSetupHooks) {
      installGitHooks(fullPath, compactOutput);
    }
    updateCreateProgress('Installing dependencies');
    const installSummary = await runViteInstall(fullPath, options.interactive, installArgs, {
      silent: compactOutput,
    });
    updateCreateProgress('Formatting code');
    await runViteFmt(fullPath, options.interactive, undefined, { silent: compactOutput });
    clearCreateProgress();
    showCreateSummary({
      description: describeScaffold(selectedTemplateName, selectedTemplateArgs),
      installSummary,
      nextCommand: getNextCommand(projectDir, 'vp run'),
      packageManager: workspaceInfo.packageManager,
      packageManagerVersion: workspaceInfo.downloadPackageManager.version,
      projectDir,
    });
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
        ? path.join(templateInfo.parentDir, selected.targetDir).split(path.sep).join('/')
        : selected.targetDir;
    }
    pauseCreateProgress();
    await checkProjectDirExists(path.join(workspaceInfo.rootDir, targetDir), options.interactive);
    resumeCreateProgress();
    updateCreateProgress('Generating project');
    result = await executeBuiltinTemplate(
      workspaceInfo,
      {
        ...templateInfo,
        packageName,
        targetDir,
      },
      { silent: compactOutput },
    );
  } else {
    updateCreateProgress('Generating project');
    result = await executeRemoteTemplate(workspaceInfo, templateInfo, { silent: compactOutput });
  }

  if (result.exitCode !== 0) {
    failCreateProgress('Scaffolding failed');
    process.exit(result.exitCode);
  }
  const projectDir = result.projectDir;
  if (!projectDir) {
    clearCreateProgress();
    process.exit(0);
  }

  const fullPath = path.join(workspaceInfo.rootDir, projectDir);
  const agentInstructionsRoot = isMonorepo ? workspaceInfo.rootDir : fullPath;
  updateCreateProgress('Writing agent instructions');
  pauseCreateProgress();
  await writeAgentInstructions({
    projectRoot: agentInstructionsRoot,
    targetPaths: selectedAgentTargetPaths,
    interactive: options.interactive,
    silent: compactOutput,
  });
  resumeCreateProgress();
  updateCreateProgress('Writing editor configs');
  pauseCreateProgress();
  await writeEditorConfigs({
    projectRoot: fullPath,
    editorId: selectedEditor,
    interactive: options.interactive,
    silent: compactOutput,
    extraVsCodeSettings: { 'npm.scriptRunner': 'vp' },
  });
  resumeCreateProgress();

  let installSummary: CommandRunSummary | undefined;
  if (isMonorepo) {
    if (!compactOutput) {
      prompts.log.step('Monorepo integration...');
    }
    updateCreateProgress('Integrating into monorepo');
    rewriteMonorepoProject(fullPath, workspaceInfo.packageManager, undefined, compactOutput);

    if (workspaceInfo.packages.length > 0) {
      if (options.interactive) {
        pauseCreateProgress();
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
        resumeCreateProgress();
      }
    }

    updateWorkspaceConfig(projectDir, workspaceInfo);
    updateCreateProgress('Installing dependencies');
    installSummary = await runViteInstall(workspaceInfo.rootDir, options.interactive, installArgs, {
      silent: compactOutput,
    });
    updateCreateProgress('Formatting code');
    await runViteFmt(workspaceInfo.rootDir, options.interactive, [projectDir], {
      silent: compactOutput,
    });
  } else {
    updateCreateProgress('Applying Vite+ project setup');
    rewriteStandaloneProject(fullPath, workspaceInfo, undefined, compactOutput);
    if (shouldSetupHooks) {
      installGitHooks(fullPath, compactOutput);
    }
    updateCreateProgress('Installing dependencies');
    installSummary = await runViteInstall(fullPath, options.interactive, installArgs, {
      silent: compactOutput,
    });
    updateCreateProgress('Formatting code');
    await runViteFmt(fullPath, options.interactive, undefined, { silent: compactOutput });
  }

  clearCreateProgress();
  showCreateSummary({
    description: describeScaffold(selectedTemplateName, selectedTemplateArgs),
    installSummary,
    nextCommand: getNextCommand(projectDir, 'vp run'),
    packageManager: workspaceInfo.packageManager,
    packageManagerVersion: workspaceInfo.downloadPackageManager.version,
    projectDir,
  });
  // #endregion
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

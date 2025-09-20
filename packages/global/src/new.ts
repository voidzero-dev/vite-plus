import fs from 'node:fs';
import path from 'node:path';
import { scheduler } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';

import * as prompts from '@clack/prompts';
import spawn from 'cross-spawn';
import mri from 'mri';
import colors from 'picocolors';

// refactor from https://github.com/vitejs/vite/blob/main/packages/create-vite/src/index.ts

const {
  blue,
  blueBright,
  cyan,
  green,
  greenBright,
  gray,
  magenta,
  red,
  redBright,
  reset,
  yellow,
} = colors;

const argv = mri<{
  monorepo?: boolean;
  template?: string;
  help?: boolean;
  overwrite?: boolean;
  app?: boolean;
  lib?: boolean;
  git?: boolean;
  pm?: string;
}>(process.argv.slice(3), {
  alias: { h: 'help', t: 'template' },
  boolean: ['help', 'overwrite', 'monorepo', 'app', 'lib', 'git'],
  string: ['template', 'pm'],
});
const cwd = process.cwd();
const pkgRoot = path.dirname(import.meta.dirname);

// prettier-ignore
const helpMessage = `\
Usage: vite new [OPTION]... [DIRECTORY]

Create a new Vite+ project in TypeScript.
With no arguments, start the CLI in interactive mode.

Options:
  --monorepo                 create a monorepo project
  --app                      add an app to existing monorepo, e.g.: vite new --app apps/website
  --lib                      add a library to existing monorepo, e.g.: vite new --lib packages/utils
  -t, --template NAME        use a specific template
  --pm NAME                  use a specific package manager, e.g.: vite new --pm pnpm
  --git                      init git repository
  --overwrite                overwrite existing files

Available templates:
${yellow('vanilla-ts     vanilla')}
${green('vue-ts         vue')}
${cyan('react-ts       react')}
${cyan('react-swc-ts   react-swc')}
${magenta('preact-ts      preact')}
${redBright('lit-ts         lit')}
${red('svelte-ts      svelte')}
${blue('solid-ts       solid')}
${blueBright('qwik-ts        qwik')}`;

type ColorFunc = (str: string | number) => string;
type Framework = {
  name: string;
  display: string;
  color: ColorFunc;
  variants: FrameworkVariant[];
};
type FrameworkVariant = {
  name: string;
  display: string;
  color: ColorFunc;
  customCommand?: string;
};

const FRAMEWORKS: Framework[] = [
  {
    name: 'vanilla',
    display: 'Vanilla',
    color: yellow,
    variants: [
      {
        name: 'vanilla-ts',
        display: 'TypeScript',
        color: blue,
      },
      {
        name: 'vanilla',
        display: 'JavaScript',
        color: yellow,
      },
    ],
  },
  {
    name: 'vue',
    display: 'Vue',
    color: green,
    variants: [
      {
        name: 'vue-ts',
        display: 'TypeScript',
        color: blue,
      },
      {
        name: 'vue',
        display: 'JavaScript',
        color: yellow,
      },
      {
        name: 'custom-create-vue',
        display: 'Official Vue Starter ↗',
        color: green,
        customCommand: 'npm create vue@latest TARGET_DIR',
      },
      {
        name: 'custom-nuxt',
        display: 'Nuxt ↗',
        color: greenBright,
        customCommand: 'npm exec nuxi init TARGET_DIR',
      },
    ],
  },
  {
    name: 'react',
    display: 'React',
    color: cyan,
    variants: [
      {
        name: 'react-ts',
        display: 'TypeScript',
        color: blue,
      },
      {
        name: 'react-swc-ts',
        display: 'TypeScript + SWC',
        color: blue,
      },
      {
        name: 'react',
        display: 'JavaScript',
        color: yellow,
      },
      {
        name: 'react-swc',
        display: 'JavaScript + SWC',
        color: yellow,
      },
      {
        name: 'custom-react-router',
        display: 'React Router v7 ↗',
        color: cyan,
        customCommand: 'npm create react-router@latest TARGET_DIR',
      },
      {
        name: 'custom-tanstack-router-react',
        display: 'TanStack Router ↗',
        color: cyan,
        customCommand: 'npm create -- tsrouter-app@latest TARGET_DIR --framework React --interactive',
      },
      {
        name: 'redwoodsdk-standard',
        display: 'RedwoodSDK ↗',
        color: red,
        customCommand: 'npm exec degit redwoodjs/sdk/starters/standard TARGET_DIR',
      },
      {
        name: 'rsc',
        display: 'RSC ↗',
        color: magenta,
        customCommand: 'npm exec degit vitejs/vite-plugin-react/packages/plugin-rsc/examples/starter TARGET_DIR',
      },
    ],
  },
  {
    name: 'preact',
    display: 'Preact',
    color: magenta,
    variants: [
      {
        name: 'preact-ts',
        display: 'TypeScript',
        color: blue,
      },
      {
        name: 'preact',
        display: 'JavaScript',
        color: yellow,
      },
      {
        name: 'custom-create-preact',
        display: 'Official Preact Starter ↗',
        color: magenta,
        customCommand: 'npm create preact@latest TARGET_DIR',
      },
    ],
  },
  {
    name: 'lit',
    display: 'Lit',
    color: redBright,
    variants: [
      {
        name: 'lit-ts',
        display: 'TypeScript',
        color: blue,
      },
      {
        name: 'lit',
        display: 'JavaScript',
        color: yellow,
      },
    ],
  },
  {
    name: 'svelte',
    display: 'Svelte',
    color: red,
    variants: [
      {
        name: 'svelte-ts',
        display: 'TypeScript',
        color: blue,
      },
      {
        name: 'svelte',
        display: 'JavaScript',
        color: yellow,
      },
      {
        name: 'custom-svelte-kit',
        display: 'SvelteKit ↗',
        color: red,
        customCommand: 'npm exec sv create TARGET_DIR',
      },
    ],
  },
  {
    name: 'solid',
    display: 'Solid',
    color: blue,
    variants: [
      {
        name: 'solid-ts',
        display: 'TypeScript',
        color: blue,
      },
      {
        name: 'solid',
        display: 'JavaScript',
        color: yellow,
      },
      {
        name: 'custom-tanstack-router-solid',
        display: 'TanStack Router ↗',
        color: cyan,
        customCommand: 'npm create -- tsrouter-app@latest TARGET_DIR --framework Solid --interactive',
      },
    ],
  },
  {
    name: 'qwik',
    display: 'Qwik',
    color: blueBright,
    variants: [
      {
        name: 'qwik-ts',
        display: 'TypeScript',
        color: blueBright,
      },
      {
        name: 'qwik',
        display: 'JavaScript',
        color: yellow,
      },
      {
        name: 'custom-qwik-city',
        display: 'QwikCity ↗',
        color: blueBright,
        customCommand: 'npm create qwik@latest basic TARGET_DIR',
      },
    ],
  },
  {
    name: 'angular',
    display: 'Angular',
    color: red,
    variants: [
      {
        name: 'custom-angular',
        display: 'Angular ↗',
        color: red,
        customCommand: 'npm exec @angular/cli@latest new TARGET_DIR',
      },
      {
        name: 'custom-analog',
        display: 'Analog ↗',
        color: yellow,
        customCommand: 'npm create analog@latest TARGET_DIR',
      },
    ],
  },
  {
    name: 'marko',
    display: 'Marko',
    color: magenta,
    variants: [
      {
        name: 'marko-run',
        display: 'Marko Run ↗',
        color: magenta,
        customCommand: 'npm create -- marko@latest --name TARGET_DIR',
      },
    ],
  },
  {
    name: 'others',
    display: 'Others',
    color: reset,
    variants: [
      {
        name: 'create-vite-extra',
        display: 'Extra Vite Starters ↗',
        color: reset,
        customCommand: 'npm create vite-extra@latest TARGET_DIR',
      },
      {
        name: 'create-electron-vite',
        display: 'Electron ↗',
        color: reset,
        customCommand: 'npm create electron-vite@latest TARGET_DIR',
      },
    ],
  },
];

const TEMPLATES = FRAMEWORKS.map((f) => f.variants.map((v) => v.name)).reduce(
  (a, b) => a.concat(b),
  [],
);

// refactor from https://github.com/Gugustinette/create-tsdown/blob/main/src/options/index.ts#L51
type LibraryTemplate = {
  name: string;
  display: string;
};

const LIBRARY_TEMPLATES: LibraryTemplate[] = [
  {
    name: 'default',
    display: 'Default',
  },
  {
    name: 'minimal',
    display: 'Minimal',
  },
  {
    name: 'react',
    display: 'React',
  },
  {
    name: 'vue',
    display: 'Vue',
  },
  {
    name: 'solid',
    display: 'Solid',
  },
];

const defaultTargetDir = 'vite-plus-project';

async function init() {
  const argTargetDir = argv._[0]
    ? formatTargetDir(String(argv._[0]))
    : undefined;
  const argTemplate = argv.template;
  const argOverwrite = argv.overwrite;
  const argGit = argv.git;
  let argPackageManager = argv.pm;
  let argCreateMonorepo = argv.monorepo;
  const argCreateApp = argv.app;
  const argCreateLib = argv.lib;
  const isCreateAppOrLib = argCreateApp || argCreateLib;

  const help = argv.help;
  if (help) {
    console.log(helpMessage);
    return;
  }

  const cancel = () => prompts.cancel('Operation cancelled');

  prompts.intro(`${blueBright('Vite+')} - The Unified Toolchain for the Web`);

  // --app and --lib should inside monorepo
  if (isCreateAppOrLib) {
    const monorepoRoot = findMonorepoRoot();
    if (!monorepoRoot) {
      prompts.log.error('Not in a monorepo. The --app and --lib flags can only be used inside a monorepo.');
      prompts.outro('Run `vite new` to create a monorepo first.');
      return;
    }
  }

  // 0. Handle monorepo
  if (typeof argCreateMonorepo !== 'boolean' && !isCreateAppOrLib) {
    const createMonorepo = await prompts.confirm({
      message: 'Monorepo:',
      initialValue: true,
    });
    if (prompts.isCancel(createMonorepo)) return cancel();
    argCreateMonorepo = createMonorepo;
  }

  // 1. Get project name and target dir
  let targetDir = argTargetDir;
  if (!targetDir) {
    const projectName = await prompts.text({
      message: 'Project name:',
      defaultValue: defaultTargetDir,
      placeholder: defaultTargetDir,
      validate: (value) => {
        return value.length === 0 || formatTargetDir(value).length > 0
          ? undefined
          : 'Invalid project name';
      },
    });
    if (prompts.isCancel(projectName)) return cancel();
    targetDir = formatTargetDir(projectName);
  }

  // 2. Handle directory if exist and not empty
  if (fs.existsSync(targetDir) && !isEmpty(targetDir)) {
    const overwrite = argOverwrite
      ? 'yes'
      : await prompts.select({
        message: (targetDir === '.'
          ? 'Current directory'
          : `Target directory "${targetDir}"`) +
          ` is not empty. Please choose how to proceed:`,
        options: [
          {
            label: 'Cancel operation',
            value: 'no',
          },
          {
            label: 'Remove existing files and continue',
            value: 'yes',
          },
          {
            label: 'Ignore files and continue',
            value: 'ignore',
          },
        ],
      });
    if (prompts.isCancel(overwrite)) return cancel();
    switch (overwrite) {
      case 'yes':
        emptyDir(targetDir);
        break;
      case 'no':
        cancel();
        return;
    }
  }

  // 3. Get package name
  let packageName = path.basename(path.resolve(targetDir));
  if (!isValidPackageName(packageName)) {
    const packageNameResult = await prompts.text({
      message: 'Package name:',
      defaultValue: toValidPackageName(packageName),
      placeholder: toValidPackageName(packageName),
      validate(dir) {
        if (!isValidPackageName(dir)) {
          return 'Invalid package.json name';
        }
      },
    });
    if (prompts.isCancel(packageNameResult)) return cancel();
    packageName = packageNameResult;
  }

  const rawTargetDir = targetDir;
  let selectedPackageManager = argPackageManager ?? 'pnpm';
  if (!isCreateAppOrLib && !argPackageManager) {
    // select a package manager
    const packageManager = await prompts.select({
      message: 'Select a package manager:',
      options: [
        { label: 'pnpm', value: 'pnpm', hint: 'recommended' },
        { label: 'yarn', value: 'yarn' },
        { label: 'npm', value: 'npm' },
      ],
      initialValue: selectedPackageManager,
    });
    if (prompts.isCancel(packageManager)) return cancel();

    selectedPackageManager = packageManager;
  }

  // 4. Choose a framework and variant
  let template = argTemplate;
  let hasInvalidArgTemplate = false;
  if (argTemplate) {
    if (argCreateLib && !LIBRARY_TEMPLATES.some((t) => t.name === argTemplate)) {
      template = undefined;
      hasInvalidArgTemplate = true;
    } else if (argCreateApp && !TEMPLATES.includes(argTemplate)) {
      template = undefined;
      hasInvalidArgTemplate = true;
    }
  }
  if (!template) {
    if (argCreateLib) {
      const libraryTemplate = await prompts.select({
        message: 'Select a library template:',
        options: LIBRARY_TEMPLATES.map((template) => {
          return {
            label: template.display || template.name,
            value: template.name,
          };
        }),
      });
      if (prompts.isCancel(libraryTemplate)) return cancel();
      template = libraryTemplate;
    } else {
      const framework = await prompts.select({
        message: hasInvalidArgTemplate
          ? `"${argTemplate}" isn't a valid template. Please choose from below: `
          : 'Select a framework:',
        options: FRAMEWORKS.map((framework) => {
          const frameworkColor = framework.color;
          return {
            label: frameworkColor(framework.display || framework.name),
            value: framework,
          };
        }),
      });
      if (prompts.isCancel(framework)) return cancel();

      const variant = await prompts.select({
        message: 'Select a variant:',
        options: framework.variants.map((variant) => {
          const variantColor = variant.color;
          return {
            label: variantColor(variant.display || variant.name),
            value: variant.name,
          };
        }),
      });
      if (prompts.isCancel(variant)) return cancel();

      template = variant;
    }
  }

  if (argCreateMonorepo) {
    // init a monorepo
    await initMonorepo(path.join(cwd, targetDir), selectedPackageManager);

    // create a default app: apps/website
    targetDir = path.join(targetDir, 'apps/website');
  }

  const root = path.join(cwd, targetDir);
  const rawRoot = path.join(cwd, rawTargetDir);
  const cdProjectName = path.relative(cwd, rawRoot);
  const appOrLibName = path.relative(cwd, root);
  const isMonorepo = argCreateMonorepo ?? isCreateAppOrLib;

  // 5. Create project
  if (argCreateLib) {
    prompts.log.step(`Scaffolding library with ${green(template)} in ${appOrLibName}...`);
    // use create-tsdown to create a library project
    const createTsdownBin = fileURLToPath(import.meta.resolve('create-tsdown/run'));
    const createTsdownArgs = [
      createTsdownBin,
      '--overwrite',
      '--template',
      template,
      '--name',
      targetDir,
    ];
    const command = `create-tsdown ${createTsdownArgs.slice(1).join(' ')}`;
    prompts.log.info(gray(`$ ${command} ...`));
    const { status, stderr, stdout } = spawn.sync('node', createTsdownArgs, {
      stdio: 'pipe',
      cwd,
    });
    if (status && status > 0) {
      prompts.log.error(stderr.toString());
      prompts.log.error(stdout.toString());
      process.exit(status);
    }
    // fix "Duplicated package name: react-components-starter or tsdown-starter" name to user input
    editFile(path.join(root, 'package.json'), (content) => {
      const pkg = JSON.parse(content);
      pkg.name = path.basename(root);
      return JSON.stringify(pkg, null, 2) + '\n';
    });
  } else {
    prompts.log.step(`Scaffolding project with ${green(template)} in ${appOrLibName}...`);
    // use create-vite to create a app project
    const createViteBin = fileURLToPath(import.meta.resolve('create-vite/index.js'));
    const { customCommand } = FRAMEWORKS.flatMap((f) => f.variants).find((v) => v.name === template) ?? {};
    if (customCommand) {
      // print the custom command
      const cwd = path.dirname(root);
      fs.mkdirSync(cwd, { recursive: true });
      const appName = isMonorepo ? path.basename(targetDir) : targetDir;
      prompts.log.info(customCommand.replace('TARGET_DIR', appName) + ' ...');
      const createViteArgs = [
        createViteBin,
        '--overwrite',
        '--template',
        template,
        appName,
      ];
      const command = `create-vite ${createViteArgs.slice(1).join(' ')}`;
      prompts.log.info(gray(`$ ${command} ...`));
      const { status } = spawn.sync('node', createViteArgs, {
        stdio: 'inherit',
        cwd,
      });
      if (status && status > 0) {
        process.exit(status);
      }
    } else {
      const createViteArgs = [
        createViteBin,
        '--overwrite',
        '--template',
        template,
        targetDir,
      ];
      const { status, stderr, stdout } = spawn.sync('node', createViteArgs, {
        stdio: 'pipe',
      });
      if (status && status > 0) {
        prompts.log.error(stderr.toString());
        prompts.log.error(stdout.toString());
        process.exit(status);
      }
    }
  }

  await fixPackageJsonForVitePlus(root, selectedPackageManager, isMonorepo);

  // first init, ask user to init git
  if (!isCreateAppOrLib) {
    const initGit = typeof argGit === 'boolean' ? argGit : await prompts.confirm({
      message: `Initialize git repository? (git init ${cdProjectName})`,
      initialValue: true,
    });
    if (prompts.isCancel(initGit)) return cancel();
    if (initGit) {
      const { status, stderr, stdout } = spawn.sync('git', ['init', cdProjectName], {
        stdio: 'pipe',
      });
      if (status && status > 0) {
        prompts.log.error(stderr.toString());
        prompts.log.error(stdout.toString());
      }
    }
  }

  let doneMessage = '';
  doneMessage += `Done. Now run:`;
  if (rawRoot !== cwd) {
    doneMessage += green(`\n  cd ${cdProjectName.includes(' ') ? `"${cdProjectName}"` : cdProjectName}`);
  }
  doneMessage += green(`\n  vite run ready`);
  doneMessage += green(`\n  vite run dev`);

  if (argCreateMonorepo) {
    doneMessage += `\n\n  To add new packages to your monorepo:\n`;
    doneMessage += `\n  vite new --app apps/my-app`;
    doneMessage += `\n  vite new --lib packages/my-lib`;
  }
  prompts.outro(doneMessage);
}

function formatTargetDir(targetDir: string) {
  return targetDir.trim().replace(/\/+$/g, '');
}

function copy(src: string, dest: string) {
  const stat = fs.statSync(src);
  if (stat.isDirectory()) {
    copyDir(src, dest);
  } else {
    fs.copyFileSync(src, dest);
  }
}

function isValidPackageName(projectName: string) {
  return /^(?:@[a-z\d\-*~][a-z\d\-*._~]*\/)?[a-z\d\-~][a-z\d\-._~]*$/.test(
    projectName,
  );
}

function toValidPackageName(projectName: string) {
  return projectName
    .trim()
    .toLowerCase()
    .replace(/\s+/g, '-')
    .replace(/^[._]/, '')
    .replace(/[^a-z\d\-~]+/g, '-');
}

function copyDir(srcDir: string, destDir: string) {
  fs.mkdirSync(destDir, { recursive: true });
  for (const file of fs.readdirSync(srcDir)) {
    const srcFile = path.resolve(srcDir, file);
    const destFile = path.resolve(destDir, file);
    copy(srcFile, destFile);
  }
}

function isEmpty(path: string) {
  const files = fs.readdirSync(path);
  return files.length === 0 || (files.length === 1 && files[0] === '.git');
}

function emptyDir(dir: string) {
  if (!fs.existsSync(dir)) {
    return;
  }
  for (const file of fs.readdirSync(dir)) {
    if (file === '.git') {
      continue;
    }
    fs.rmSync(path.resolve(dir, file), { recursive: true, force: true });
  }
}

function editFile(file: string, callback: (content: string) => string) {
  const content = fs.readFileSync(file, 'utf-8');
  fs.writeFileSync(file, callback(content), 'utf-8');
}

async function initMonorepo(rootProjectDir: string, packageManager: string) {
  const templateDir = path.resolve(
    pkgRoot,
    'templates/monorepo',
  );
  copyDir(templateDir, rootProjectDir);
  if (packageManager === 'pnpm') {
    // remove workspaces field
    editFile(path.join(rootProjectDir, 'package.json'), (content) => {
      const pkg = JSON.parse(content);
      pkg.workspaces = undefined;
      return JSON.stringify(pkg, null, 2) + '\n';
    });
    fs.unlinkSync(path.join(rootProjectDir, '.yarnrc.yml'));
  } else if (packageManager === 'yarn') {
    // remove pnpm field
    editFile(path.join(rootProjectDir, 'package.json'), (content) => {
      const pkg = JSON.parse(content);
      pkg.pnpm = undefined;
      return JSON.stringify(pkg, null, 2) + '\n';
    });
    fs.unlinkSync(path.join(rootProjectDir, 'pnpm-workspace.yaml'));
  } else {
    // npm
    // remove pnpm field
    editFile(path.join(rootProjectDir, 'package.json'), (content) => {
      const pkg = JSON.parse(content);
      pkg.pnpm = undefined;
      return JSON.stringify(pkg, null, 2) + '\n';
    });
    fs.unlinkSync(path.join(rootProjectDir, 'pnpm-workspace.yaml'));
    fs.unlinkSync(path.join(rootProjectDir, '.yarnrc.yml'));
  }

  await setPackageManager(rootProjectDir, packageManager);
  renameFiles(rootProjectDir);
}

const RENAME_FILES: Record<string, string> = {
  _gitignore: '.gitignore',
};

function renameFiles(projectDir: string) {
  for (const [from, to] of Object.entries(RENAME_FILES)) {
    fs.renameSync(path.join(projectDir, from), path.join(projectDir, to));
  }
}

async function fixPackageJsonForVitePlus(projectDir: string, selectedPackageManager: string, isMonorepo?: boolean) {
  editFile(path.join(projectDir, 'package.json'), (content) => {
    const pkg = JSON.parse(content);

    // force to use the latest vite-plus instead of vite
    if (!isMonorepo) {
      const viteVersion = 'npm:@voidzero-dev/vite-plus@latest';
      pkg.devDependencies['vite'] = viteVersion;
      if (selectedPackageManager === 'pnpm') {
        pkg.pnpm = {
          ...pkg.pnpm,
          overrides: {
            ...pkg.pnpm?.overrides,
            vite: viteVersion,
          },
        };
      } else {
        pkg.resolutions = {
          ...pkg.resolutions,
          vite: viteVersion,
        };
      }
    }
    // fix vite dev command
    if (pkg.scripts?.dev === 'vite') {
      pkg.scripts.dev = 'vite dev';
    }
    // fix tsdown build and dev command
    if (pkg.scripts?.build === 'tsdown') {
      pkg.scripts.build = 'vite lib';
    }
    if (pkg.scripts?.dev === 'tsdown --watch') {
      pkg.scripts.dev = 'vite lib --watch';
    }
    // try to add ready script
    if (!pkg.scripts?.ready) {
      pkg.scripts.ready = 'vite lint --type-aware && vite run build && vite test --passWithNoTests';
    }
    // fix empty pkg.name
    if (!pkg.name) {
      pkg.name = path.basename(projectDir);
    }
    return JSON.stringify(pkg, null, 2) + '\n';
  });

  if (isMonorepo) {
    // remove .github, .vscode directories
    fs.rmSync(path.join(projectDir, '.github'), { recursive: true, force: true });
    fs.rmSync(path.join(projectDir, '.vscode'), { recursive: true, force: true });
  } else {
    await setPackageManager(projectDir, selectedPackageManager);
    // copy .npmrc file to install vite-plus
    if (selectedPackageManager === 'yarn') {
      copy(path.join(pkgRoot, 'templates/config/.yarnrc.yml'), path.join(projectDir, '.yarnrc.yml'));
    } else {
      copy(path.join(pkgRoot, 'templates/config/.npmrc'), path.join(projectDir, '.npmrc'));
    }
  }

  // remove package-lock.json when package manager is not npm
  if (selectedPackageManager !== 'npm') {
    fs.rmSync(path.join(projectDir, 'package-lock.json'), { force: true });
  }
}

async function setPackageManager(projectDir: string, packageManager: string) {
  // Fixed fallback versions for each package manager
  const FALLBACK_VERSIONS: Record<string, string> = {
    pnpm: '10.17.0',
    yarn: '4.10.2',
    npm: '11.6.0',
  };

  let version: string;
  const name = packageManager === 'yarn' ? '@yarnpkg/cli-dist' : packageManager;

  // Try to fetch with retries
  let retries = 3;
  let lastError: Error | null = null;

  while (retries > 0) {
    try {
      const response = await fetch(`https://registry.npmjs.org/${name}/latest`);
      if (!response.ok) {
        throw new Error(
          `Failed to fetch latest version for package "${name}" (network request failure). HTTP ${response.status}: ${response.statusText}`,
        );
      }
      const pkgInfo = (await response.json()) as { version: string };
      version = pkgInfo.version;
      break; // Success, exit retry loop
    } catch (err) {
      lastError = err as Error;
      retries--;
      if (retries > 0) {
        // Wait a bit before retrying (exponential backoff)
        await scheduler.wait((4 - retries) * 500);
      }
    }
  }

  // If all retries failed, use fallback version
  if (retries === 0) {
    version = FALLBACK_VERSIONS[packageManager];
    prompts.log.warn(`Failed to fetch latest ${packageManager} version after 3 retries: ${lastError?.message}`);
    prompts.log.warn(
      `Could not retrieve the latest version of ${packageManager}. Using fallback version: ${packageManager}@${version}. ` +
        `This may not be the most up-to-date version. Your project will still work, but you can manually update the package manager version in package.json later if you wish.`,
    );
  }

  // set package manager
  editFile(path.join(projectDir, 'package.json'), (content) => {
    const pkg = JSON.parse(content);
    pkg.packageManager = `${packageManager}@${version!}`;
    return JSON.stringify(pkg, null, 2) + '\n';
  });
}

function findMonorepoRoot(): string | null {
  let currentDir = process.cwd();

  while (currentDir !== path.dirname(currentDir)) {
    const pnpmWorkspaceFile = path.join(currentDir, 'pnpm-workspace.yaml');
    const packageJson = path.join(currentDir, 'package.json');

    // Check if this is a pnpm monorepo
    if (fs.existsSync(pnpmWorkspaceFile)) {
      return currentDir;
    }

    // Check if this is a npm/yarn monorepo
    if (fs.existsSync(packageJson)) {
      try {
        const pkg = JSON.parse(fs.readFileSync(packageJson, 'utf-8'));
        if (pkg.workspaces) {
          return currentDir;
        }
      } catch {
        // Continue searching
      }
    }

    currentDir = path.dirname(currentDir);
  }

  return null;
}

init().catch((err) => {
  console.error('[vite+] Failed to initialize project: %s', err);
  process.exit(1);
});

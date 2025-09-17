import fs from 'node:fs';
import path from 'node:path';
import { stdin as input, stdout as output } from 'node:process';
import readline from 'node:readline/promises';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export interface ScaffoldOptions {
  projectName?: string;
  projectType?: 'monorepo' | 'singlerepo';
  app?: string;
  lib?: string;
  help?: boolean;
}

function copyTemplate(src: string, dest: string) {
  // Create destination directory
  if (!fs.existsSync(dest)) {
    fs.mkdirSync(dest, { recursive: true });
  }

  const entries = fs.readdirSync(src, { withFileTypes: true });

  for (const entry of entries) {
    const srcPath = path.join(src, entry.name);
    // Rename _gitignore back to .gitignore when copying
    const destName = entry.name === '_gitignore' ? '.gitignore' : entry.name;
    const destPath = path.join(dest, destName);

    // Skip node_modules
    if (entry.name === 'node_modules') {
      continue;
    }

    if (entry.isDirectory()) {
      copyTemplate(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

async function promptForProjectType(): Promise<'monorepo' | 'singlerepo'> {
  const rl = readline.createInterface({ input, output });

  console.log('\nSelect project type:');
  console.log('  1) MonoRepo - Multiple packages in one repository (default)');
  console.log('  2) SingleRepo - Single package repository');
  console.log('');

  const answer = await rl.question('Your choice (1 or 2, default: 1): ');
  rl.close();

  // Default to monorepo if user just presses Enter or enters '1'
  if (!answer.trim() || answer === '1') return 'monorepo';
  if (answer === '2') return 'singlerepo';

  console.log('Invalid choice. Defaulting to MonoRepo.');
  return 'monorepo';
}

async function promptForProjectName(): Promise<string> {
  const rl = readline.createInterface({ input, output });
  const name = await rl.question('Project name: ');
  rl.close();

  if (name && name.trim()) {
    return name.trim();
  }

  console.log('Project name cannot be empty. Using default: my-vite-project');
  return 'my-vite-project';
}

function findMonoRepoRoot(): string | null {
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

function showHelp() {
  console.log(`
vite new - Create a new Vite+ project

Usage:
  vite new [project-name] [project-type] [options]
  vite new --app <path>    Add an app to existing monorepo
  vite new --lib <path>    Add a library to existing monorepo

Arguments:
  project-name    Name of the project to create
  project-type    Type of project: monorepo, mono, singlerepo, single

Options:
  --monorepo, -m      Create a monorepo project
  --singlerepo, -s    Create a singlerepo project
  --app <path>        Add an app package to existing monorepo
  --lib <path>        Add a library package to existing monorepo
  --help, -h          Show this help message

Examples:
  vite new                        Interactive mode (prompts for all options)
  vite new my-project             Create project with prompts for type
  vite new my-project monorepo   Create monorepo without prompts
  vite new my-project -s          Create singlerepo using flag
  vite new --app apps/my-app     Add app to current monorepo
  vite new --lib packages/utils  Add library to current monorepo

Notes:
  - MonoRepo is the default when prompted (press Enter to select)
  - Press Ctrl+C to exit during prompts
`);
}

export async function scaffold(options: ScaffoldOptions = {}) {
  // Show help if requested
  if (options.help) {
    showHelp();
    return;
  }

  const templatesDir = path.resolve(__dirname, '..', 'templates');

  // Handle monorepo app/lib additions
  if (options.app || options.lib) {
    const monorepoRoot = findMonoRepoRoot();

    if (!monorepoRoot) {
      console.error('Error: Not in a monorepo. The --app and --lib flags can only be used inside a monorepo.');
      process.exit(1);
    }

    if (options.app) {
      // use template from https://github.com/vitejs/vite/tree/main/packages/create-vite/template-vanilla-ts
      const templatePath = path.join(templatesDir, 'monorepo-app');
      const targetPath = path.join(monorepoRoot, options.app);

      if (fs.existsSync(targetPath)) {
        console.error(`Error: Directory ${options.app} already exists.`);
        process.exit(1);
      }

      console.log(`Creating new app at ${options.app}...`);
      copyTemplate(templatePath, targetPath);

      // Update package.json with the actual app name
      const packageJsonPath = path.join(targetPath, 'package.json');
      if (fs.existsSync(packageJsonPath)) {
        const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
        packageJson.name = path.basename(options.app);
        fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2) + '\n');
      }

      console.log(`✅ Successfully created app at ${options.app}`);
      console.log('\nNext steps:');
      console.log(`  cd ${path.relative(process.cwd(), targetPath)}`);
      // console.log('  vite run ready');
      console.log('  vite run dev');
    }

    if (options.lib) {
      // use template from https://github.com/Gugustinette/create-tsdown/tree/main/templates/default
      const templatePath = path.join(templatesDir, 'monorepo-lib');
      const targetPath = path.join(monorepoRoot, options.lib);

      if (fs.existsSync(targetPath)) {
        console.error(`Error: Directory ${options.lib} already exists.`);
        process.exit(1);
      }

      console.log(`Creating new library at ${options.lib}...`);
      copyTemplate(templatePath, targetPath);

      // Update package.json with the actual lib name
      const packageJsonPath = path.join(targetPath, 'package.json');
      if (fs.existsSync(packageJsonPath)) {
        const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
        packageJson.name = path.basename(options.lib);
        fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2) + '\n');
      }

      console.log(`✅ Successfully created library at ${options.lib}`);
      console.log('\nNext steps:');
      // console.log(`  cd ${path.relative(process.cwd(), targetPath)}`);
      console.log('  vite run ready');
    }

    return;
  }

  // Get project name - prompt only if not provided
  const projectName = options.projectName || await promptForProjectName();

  // Get project type - prompt only if not provided
  const projectType = options.projectType || await promptForProjectType();

  const targetDir = path.join(process.cwd(), projectName);

  if (fs.existsSync(targetDir)) {
    console.error(`Error: Directory ${projectName} already exists.`);
    process.exit(1);
  }

  console.log(`\nCreating ${projectType} project: ${projectName}...`);

  const templatePath = path.join(templatesDir, projectType);
  copyTemplate(templatePath, targetDir);

  // Update package.json with the actual project name
  const packageJsonPath = path.join(targetDir, 'package.json');
  if (fs.existsSync(packageJsonPath)) {
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
    packageJson.name = projectName;
    fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2) + '\n');
  }

  console.log(`✅ Successfully created ${projectType} project: ${projectName}`);
  console.log('\nNext steps:');
  console.log(`  cd ${projectName}`);
  console.log('  vite run ready');
  console.log('  vite run dev');

  if (projectType === 'monorepo') {
    console.log('\nTo add new packages to your monorepo:');
    console.log('  vite new --app apps/my-app');
    console.log('  vite new --lib packages/my-lib');
  }
}

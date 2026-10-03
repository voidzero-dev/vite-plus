import fs from 'node:fs';
import path from 'node:path';

import validateNpmPackageName from 'validate-npm-package-name';

import { editJsonFile } from '../utils/json.ts';
import type { CommandRunSummary } from '../utils/prompts.ts';
import { getRandomProjectName } from './random-name.ts';

export interface CreateCompletion {
  /** One line per step that failed, in the order the steps ran. */
  failures: string[];
  /** Command worth suggesting next: recovery when the project is not usable. */
  nextCommand: string;
  /** Non-zero when the scaffolded project cannot be run as it stands. */
  exitCode: number;
}

/**
 * Decide what `vp create` should report once scaffolding is done.
 *
 * The template files are written before dependencies are installed and the
 * result is formatted, so those later steps can fail while the project
 * directory already exists. "Files were generated" and "the project is ready
 * to run" are different states, and the completion summary has to tell them
 * apart instead of reporting success for both.
 *
 * A skipped step (`VP_SKIP_INSTALL`) is not a failure — nothing was attempted.
 *
 * Only a failed install changes the exit code. Formatting runs against a
 * project whose dependencies may not be resolvable yet — scaffolding a
 * `react-ts` or `vue-ts` template leaves a `vite.config.ts` importing a plugin
 * that is not installed, so `vp fmt` cannot load the config and fails on a
 * project that is otherwise complete. Reporting that as a failed command would
 * break every such `vp create` in CI, so it is named in the summary and left
 * out of the exit code.
 */
export function resolveCreateCompletion(options: {
  installSummary?: CommandRunSummary;
  fmtSummary?: CommandRunSummary;
}): CreateCompletion {
  const installFailed = options.installSummary?.status === 'failed';
  const fmtFailed = options.fmtSummary?.status === 'failed';

  const failures: string[] = [];
  if (installFailed) {
    failures.push('Dependencies were not installed');
  }
  if (fmtFailed) {
    failures.push('Code was not formatted');
  }

  return {
    failures,
    // Without `node_modules` the suggested `vp run` cannot work, so point at
    // the step that has to succeed first. A format failure leaves a runnable
    // project, so it does not change the suggestion.
    nextCommand: installFailed ? 'vp install' : 'vp run',
    exitCode: installFailed ? 1 : 0,
  };
}

export type CreateEditorOption = string | false | undefined;

function hasExplicitEditorOptIn(editor: CreateEditorOption): boolean {
  return typeof editor === 'string' && editor.trim() !== '';
}

export function shouldConfigureEditorsForCreate({
  editor,
  isMonorepo,
}: {
  editor: CreateEditorOption;
  isMonorepo: boolean;
}): boolean {
  if (editor === false) {
    return false;
  }
  if (!isMonorepo) {
    return true;
  }
  return hasExplicitEditorOptIn(editor);
}

// Helper functions for file operations
export function copy(src: string, dest: string) {
  const stat = fs.statSync(src);
  if (stat.isDirectory()) {
    copyDir(src, dest);
  } else {
    fs.copyFileSync(src, dest);
  }
}

export function copyDir(srcDir: string, destDir: string) {
  fs.mkdirSync(destDir, { recursive: true });
  for (const file of fs.readdirSync(srcDir)) {
    const srcFile = path.resolve(srcDir, file);
    const destFile = path.resolve(destDir, file);
    copy(srcFile, destFile);
  }
}

/**
 * Format the target directory into a valid directory name and package name
 *
 * Examples:
 * ```
 * # invalid target directories
 * /foo/bar -> { directory: '', packageName: '', error: 'Absolute path is not allowed' }
 * @scope/ -> { directory: '', packageName: '', error: 'Invalid target directory' }
 * ../../foo/bar -> { directory: '', packageName: '', error: 'Invalid target directory' }
 *
 * # valid target directories
 * . -> { directory: '.', packageName: '' }
 * ./my-package -> { directory: './my-package', packageName: 'my-package' }
 * ./foo/bar-package -> { directory: './foo/bar-package', packageName: 'bar-package' }
 * ./foo/bar-package/ -> { directory: './foo/bar-package', packageName: 'bar-package' }
 * my-package -> { directory: 'my-package', packageName: 'my-package' }
 * @my-scope/my-package -> { directory: 'my-package', packageName: '@my-scope/my-package' }
 * foo/@my-scope/my-package -> { directory: 'foo/my-package', packageName: '@scope/my-package' }
 * ./foo/@my-scope/my-package -> { directory: './foo/my-package', packageName: '@scope/my-package' }
 * ./foo/bar/@scope/my-package -> { directory: './foo/bar/my-package', packageName: '@scope/my-package' }
 * ```
 */
export function formatTargetDir(input: string): {
  directory: string;
  packageName: string;
  error?: string;
} {
  let targetDir = path.normalize(input.trim());

  // "." or "./" means current directory — valid directory, but no package name derivable
  if (targetDir === '.' || targetDir === `.${path.sep}`) {
    return { directory: '.', packageName: '' };
  }

  const parsed = path.parse(targetDir);
  if (parsed.root || path.isAbsolute(targetDir)) {
    return {
      directory: '',
      packageName: '',
      error: 'Absolute path is not allowed',
    };
  }
  if (targetDir.includes('..')) {
    return {
      directory: '',
      packageName: '',
      error: 'Relative path contains ".." which is not allowed',
    };
  }
  let packageName = parsed.base;
  const parentName = path.basename(parsed.dir);
  if (parentName.startsWith('@')) {
    // skip scope directory
    // ./@my-scope/my-package -> ./my-package
    targetDir = path.join(path.dirname(parsed.dir), packageName);
    packageName = `${parentName}/${packageName}`;
  }
  const result = validateNpmPackageName(packageName);
  if (!result.validForNewPackages) {
    // invalid package name
    const message = result.errors?.[0] ?? result.warnings?.[0] ?? 'Invalid package name';
    return {
      directory: '',
      packageName: '',
      error: `Parsed package name "${packageName}" is invalid: ${message}`,
    };
  }
  return { directory: targetDir.split(path.sep).join('/'), packageName };
}

// Get the project directory from the project name
// If the project name is a scoped package name, return the second part
// Otherwise, return the project name
export function getProjectDirFromPackageName(packageName: string) {
  if (packageName.startsWith('@')) {
    return packageName.split('/')[1];
  }
  return packageName;
}

export function setPackageName(projectDir: string, packageName: string) {
  editJsonFile<{ name?: string }>(path.join(projectDir, 'package.json'), (pkg) => {
    pkg.name = packageName;
    return pkg;
  });
}

const RENAME_FILES = {
  _gitignore: '.gitignore',
  _npmrc: '.npmrc',
  '_yarnrc.yml': '.yarnrc.yml',
} as const;

/** Rename underscore-prefixed scaffold files to their dotfile names in `projectDir`. */
export function renameFiles(projectDir: string): void {
  for (const [from, to] of Object.entries(RENAME_FILES)) {
    const fromPath = path.join(projectDir, from);
    if (fs.existsSync(fromPath)) {
      fs.renameSync(fromPath, path.join(projectDir, to));
    }
  }
}

const DOTENV_GITIGNORE_LINES = [
  '# dotenv environment variable files',
  '.env',
  '.env.*',
  '!.env.example',
] as const;

/**
 * Make sure the scaffolded project's `.gitignore` excludes default generated
 * project artifacts.
 *
 * Called right after `git init` so even bundled `@org` templates (which
 * may ship without a `.gitignore`) don't end up tracking dependencies or
 * local environment files on the user's first commit.
 */
export function ensureDefaultGitignoreEntries(projectDir: string): void {
  const gitignorePath = path.join(projectDir, '.gitignore');
  let content = '';
  try {
    content = fs.readFileSync(gitignorePath, 'utf-8');
  } catch {
    // No existing .gitignore — we'll write a fresh one below.
  }

  const lines: string[] = [];
  if (!hasNodeModulesGitignoreLine(content)) {
    lines.push('node_modules');
  }

  const missingDotenvLines = DOTENV_GITIGNORE_LINES.filter(
    (line) => !hasGitignoreLine(content, line),
  );
  if (missingDotenvLines.length > 0) {
    const startsDotenvSection = missingDotenvLines[0] === DOTENV_GITIGNORE_LINES[0];
    if (lines.length > 0 || (startsDotenvSection && content.trim() !== '')) {
      lines.push('');
    }
    lines.push(...missingDotenvLines);
  }

  appendGitignoreLines(gitignorePath, content, lines);
}

function hasNodeModulesGitignoreLine(content: string): boolean {
  return /^\s*node_modules\/?\s*$/m.test(content);
}

function hasGitignoreLine(content: string, line: string): boolean {
  return content.split(/\r?\n/).some((entry) => entry.trim() === line);
}

function appendGitignoreLines(
  gitignorePath: string,
  content: string,
  lines: readonly string[],
): void {
  if (lines.length === 0) {
    return;
  }
  const prefix = content === '' || content.endsWith('\n') ? '' : '\n';
  fs.appendFileSync(gitignorePath, `${prefix}${lines.join('\n')}\n`);
}

const VSCODE_SETTINGS_PATH = '.vscode/settings.json';
const VSCODE_EXTENSIONS_PATH = '.vscode/extensions.json';
const VSCODE_CONFIG_UNIGNORE_BLOCK = [
  '!.vscode/',
  `!${VSCODE_SETTINGS_PATH}`,
  `!${VSCODE_EXTENSIONS_PATH}`,
] as const;

/**
 * Make generated VS Code workspace config trackable when `vp create` writes VS Code config.
 */
export function ensureGitignoreVsCodeEditorConfigs(projectDir: string): void {
  if (!fs.existsSync(path.join(projectDir, VSCODE_SETTINGS_PATH))) {
    return;
  }

  const gitignorePath = path.join(projectDir, '.gitignore');
  let content: string;
  try {
    content = fs.readFileSync(gitignorePath, 'utf-8');
  } catch {
    return;
  }

  appendGitignoreVsCodeEditorConfigsBlock(gitignorePath, content);
}

function appendGitignoreVsCodeEditorConfigsBlock(gitignorePath: string, content: string): void {
  if (content.trimEnd().endsWith(VSCODE_CONFIG_UNIGNORE_BLOCK.join('\n'))) {
    return;
  }
  appendGitignoreLines(gitignorePath, content, VSCODE_CONFIG_UNIGNORE_BLOCK);
}

export function formatDisplayTargetDir(targetDir: string) {
  const normalized = targetDir.split(path.sep).join('/');
  if (normalized === '' || normalized === '.') {
    return './';
  }
  if (
    normalized.startsWith('./') ||
    normalized.startsWith('../') ||
    normalized.startsWith('/') ||
    normalized.startsWith('~')
  ) {
    return normalized;
  }
  return `./${normalized}`;
}

export function deriveDefaultPackageName(
  cwd: string,
  scope: string | undefined,
  fallbackName: string,
): string {
  const dirName = path.basename(cwd);
  const candidate = scope ? `${scope}/${dirName}` : dirName;
  return validateNpmPackageName(candidate).validForNewPackages
    ? candidate
    : getRandomProjectName({ scope, fallbackName });
}

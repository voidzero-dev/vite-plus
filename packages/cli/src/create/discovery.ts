import fs from 'node:fs';
import path from 'node:path';

import type { WorkspaceInfo, WorkspaceInfoOptional } from '../types/index.ts';
import { readJsonFile } from '../utils/json.ts';
import { isBingoTemplate } from '../utils/workspace.ts';
import { prependToPathToEnvs } from './command.ts';
import { isRelativePath } from './org-manifest.ts';
import { BuiltinTemplate, type TemplateInfo, TemplateType } from './templates/types.ts';

// Check if template name is a GitHub URL
export function isGitHubUrl(templateName: string): boolean {
  return (
    templateName.startsWith('https://github.com/') ||
    templateName.startsWith('github:') ||
    templateName.includes('github.com/')
  );
}

// Convert GitHub URL to degit format
export function parseGitHubUrl(url: string): string | null {
  // github:user/repo → user/repo
  if (url.startsWith('github:')) {
    return url.slice(7);
  }

  // https://github.com/user/repo → user/repo
  const match = url.match(/github\.com\/([^/]+\/[^/]+)/);
  if (match) {
    return match[1].replace(/\.git$/, '');
  }

  return null;
}

export function inferGitHubRepoName(templateName: string): string | null {
  const degitPath = parseGitHubUrl(templateName);
  if (!degitPath) {
    return null;
  }

  const repoName = degitPath.split('/').pop();
  return repoName || null;
}

// Resolve a declared local template (by workspace package name or a relative
// `./path`) to its directory, relative to the workspace root and using forward
// slashes (so it matches `parentDirs` and joins cleanly on any platform).
function localTemplateDir(
  workspaceInfo: WorkspaceInfoOptional,
  templateName: string,
): string | undefined {
  if (isRelativePath(templateName)) {
    return templateName.replace(/^\.\//, '');
  }
  return workspaceInfo.packages.find((pkg) => pkg.name === templateName)?.path;
}

// Resolve the bin script a local template package should be executed through.
// A single bin (string, or a one-entry object) is unambiguous. For multiple
// bin entries, prefer the one named after the package (scoped or unscoped) and
// fail clearly otherwise, since a local generator is run directly by `node`.
function resolveLocalBinPath(
  localPackagePath: string,
  packageName: string,
  bin: Record<string, string> | string | undefined,
): string | undefined {
  if (!bin) {
    return undefined;
  }
  if (typeof bin === 'string') {
    return path.join(localPackagePath, bin);
  }
  const entries = Object.entries(bin);
  if (entries.length === 0) {
    return undefined;
  }
  if (entries.length === 1) {
    return path.join(localPackagePath, entries[0][1]);
  }
  const unscopedName = packageName.slice(packageName.lastIndexOf('/') + 1);
  const preferred = bin[packageName] ?? bin[unscopedName];
  if (preferred) {
    return path.join(localPackagePath, preferred);
  }
  throw new Error(
    `Local template package "${packageName}" defines multiple "bin" entries (${entries
      .map(([name]) => name)
      .join(', ')}); add a "bin" entry named "${packageName}" so the template entry is unambiguous`,
  );
}

// Discover and identify a template
export function discoverTemplate(
  templateName: string,
  templateArgs: string[],
  workspaceInfo: WorkspaceInfo,
  interactive?: boolean,
  bundledLocalPath?: string,
  skipShorthand?: boolean,
  // True when `templateName` was resolved from a `create.templates` entry, so a
  // matching workspace package should run as a local template (and a missing
  // `bin` is an error rather than an npm fall-through).
  localTemplate?: boolean,
): TemplateInfo {
  const envs = prependToPathToEnvs(workspaceInfo.downloadPackageManager.binPrefix, {
    ...process.env,
  });
  const parentDir = inferParentDir(templateName, workspaceInfo, localTemplate);
  if (bundledLocalPath) {
    return {
      command: '',
      args: [...templateArgs],
      envs,
      type: TemplateType.bundled,
      parentDir,
      interactive,
      localPath: bundledLocalPath,
    };
  }
  // Check for built-in templates
  if (templateName.startsWith('vite:')) {
    return {
      command: templateName,
      args: [...templateArgs],
      envs,
      type: TemplateType.builtin,
      parentDir,
      interactive,
    };
  }

  // Check for GitHub URLs
  if (isGitHubUrl(templateName)) {
    const degitPath = parseGitHubUrl(templateName);
    if (degitPath) {
      return {
        command: 'degit',
        args: [degitPath, ...templateArgs],
        envs,
        type: TemplateType.remote,
        parentDir,
        interactive,
      };
    }
  }

  // Resolve a declared `create.templates` entry that points at a local package,
  // either by workspace package name or by a relative `./path` to its directory
  // (resolved against the workspace root). Only when `localTemplate` is set —
  // `create.templates` is the source of truth; a bare workspace name is not a
  // template otherwise. Relative paths are escape-checked at config validation.
  if (localTemplate) {
    const localDir = localTemplateDir(workspaceInfo, templateName);
    // A declared local template that resolves to nothing (a renamed/removed
    // workspace package, or a typo) must fail clearly instead of falling
    // through to an unrelated same-named npm package.
    if (!localDir) {
      throw new Error(
        `Local template "${templateName}" does not match any workspace package; ` +
          `update the \`create.templates\` entry in vite.config.ts`,
      );
    }
    const localPackagePath = path.join(workspaceInfo.rootDir, localDir);
    const packageJsonPath = path.join(localPackagePath, 'package.json');
    if (!fs.existsSync(packageJsonPath)) {
      throw new Error(
        `Local template "${templateName}" has no package.json, so it cannot be run as a template`,
      );
    }
    const pkg = readJsonFile(packageJsonPath) as {
      name?: string;
      dependencies?: Record<string, string>;
      bin?: Record<string, string> | string;
    };
    const binPath = resolveLocalBinPath(localPackagePath, pkg.name ?? templateName, pkg.bin);
    // A declared template without a bin entry cannot be executed. Fail clearly
    // instead of falling through to an unrelated `create-<name>` npm package.
    if (!binPath) {
      throw new Error(
        `Local template "${templateName}" has no "bin" entry in its package.json, so it cannot be run as a template`,
      );
    }
    const args = [binPath, ...templateArgs];
    let type: TemplateType = TemplateType.remote;
    if (isBingoTemplate(pkg)) {
      type = TemplateType.bingo;
      // add `--skip-requests` by default for bingo templates
      args.push('--skip-requests');
    }
    return {
      command: 'node',
      args,
      envs,
      type,
      parentDir,
      interactive,
    };
  }

  // Manifest-resolved entries are already fully qualified by the author —
  // `@scope/template-web` means that exact package, not `@scope/create-template-web`.
  const expandedName = skipShorthand ? templateName : expandCreateShorthand(templateName);
  return {
    command: expandedName,
    args: [...templateArgs],
    envs,
    type: TemplateType.remote,
    parentDir,
    interactive,
  };
}

/**
 * Expand shorthand template names to their full `create-*` package names.
 *
 * This follows the same convention as `npm create` / `pnpm create`:
 * - `vite` → `create-vite`
 * - `vite@latest` → `create-vite@latest`
 *
 * Special cases for packages where the convention doesn't work:
 * - `nitro` → `create-nitro-app` (create-nitro is abandoned)
 * - `svelte` → `sv`
 * - `@tanstack/start` → `@tanstack/cli` (@tanstack/create-start is deprecated)
 *
 * Skips expansion for:
 * - Builtin templates (`vite:*`)
 * - GitHub URLs
 * - Local paths (`./`, `../`, `/`)
 * - Names already starting with `create-` (or `@scope/create-`)
 */
export function expandCreateShorthand(templateName: string): string {
  // Skip builtins (vite:monorepo, vite:application, etc.)
  if (templateName.includes(':')) {
    return templateName;
  }

  // Skip GitHub URLs
  if (isGitHubUrl(templateName)) {
    return templateName;
  }

  // Skip local paths
  if (
    templateName.startsWith('./') ||
    templateName.startsWith('../') ||
    templateName.startsWith('/')
  ) {
    return templateName;
  }

  // Scoped package: @scope/name[@version]
  if (templateName.startsWith('@')) {
    const slashIndex = templateName.indexOf('/');
    if (slashIndex === -1) {
      // @scope or @scope@version → @scope/create[@version]
      const atIndex = templateName.indexOf('@', 1);
      const scope = atIndex === -1 ? templateName : templateName.slice(0, atIndex);
      const version = atIndex === -1 ? '' : templateName.slice(atIndex);
      return `${scope}/create${version}`;
    }
    const scope = templateName.slice(0, slashIndex);
    const rest = templateName.slice(slashIndex + 1);

    // Split name and version: name@version
    const atIndex = rest.indexOf('@');
    const name = atIndex === -1 ? rest : rest.slice(0, atIndex);
    const version = atIndex === -1 ? '' : rest.slice(atIndex);

    if (name.startsWith('create-')) {
      return templateName;
    }

    // Special cases where the default convention doesn't apply
    if (scope === '@tanstack' && name === 'start') {
      return `@tanstack/cli${version}`;
    }

    return `${scope}/create-${name}${version}`;
  }

  // Unscoped package: name[@version]
  const atIndex = templateName.indexOf('@');
  const name = atIndex === -1 ? templateName : templateName.slice(0, atIndex);
  const version = atIndex === -1 ? '' : templateName.slice(atIndex);

  if (name.startsWith('create-')) {
    return templateName;
  }

  // Special cases where the default convention doesn't apply
  if (name === 'nitro') {
    return `create-nitro-app${version}`;
  }
  if (name === 'svelte') {
    return `sv${version}`;
  }

  return `create-${name}${version}`;
}

// Infer the parent directory of the generated package based on the template name
export function inferParentDir(
  templateName: string,
  workspaceInfo: WorkspaceInfoOptional,
  localTemplate = false,
): string | undefined {
  if (workspaceInfo.parentDirs.length === 0) {
    return undefined;
  }
  // Output generated from a local package belongs next to that package, in the
  // parent directory it already lives in, rather than defaulting to the `apps`
  // rule below. Gated like `discoverTemplate`: only a `create.templates`
  // resolution makes the name local — an npm template that merely collides
  // with a workspace package name must not be co-located with it.
  const localDir = localTemplate ? localTemplateDir(workspaceInfo, templateName) : undefined;
  if (localDir) {
    const ownParentDir = path.dirname(localDir);
    if (workspaceInfo.parentDirs.includes(ownParentDir)) {
      return ownParentDir;
    }
  }
  // apps/applications by default
  let rule = /app/i;
  if (templateName === BuiltinTemplate.library) {
    // libraries/packages/components
    rule = /lib|component|package/i;
  } else if (templateName === BuiltinTemplate.generator) {
    // generators/tools
    rule = /generator|tool/i;
  }
  for (const parentDir of workspaceInfo.parentDirs) {
    if (rule.test(parentDir)) {
      return parentDir;
    }
  }
  return undefined;
}

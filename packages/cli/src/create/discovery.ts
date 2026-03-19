import path from 'node:path';

import type { WorkspaceInfo, WorkspaceInfoOptional } from '../types/index.js';
import { readJsonFile } from '../utils/json.js';
import { prependToPathToEnvs } from './command.js';
import { BuiltinTemplate, type TemplateInfo, TemplateType } from './templates/types.js';

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

// Discover and identify a template
export function discoverTemplate(
  templateName: string,
  templateArgs: string[],
  workspaceInfo: WorkspaceInfo,
  interactive?: boolean,
): TemplateInfo {
  const envs = prependToPathToEnvs(workspaceInfo.downloadPackageManager.binPrefix, {
    ...process.env,
  });
  const parentDir = inferParentDir(templateName, workspaceInfo);
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

  // Check for local package
  const localPackage = workspaceInfo.packages.find((pkg) => pkg.name === templateName);
  if (localPackage) {
    const localPackagePath = path.join(workspaceInfo.rootDir, localPackage.path);
    const packageJsonPath = path.join(localPackagePath, 'package.json');
    const pkg = readJsonFile<{
      dependencies?: Record<string, string>;
      keywords?: string[];
      bin?: Record<string, string> | string;
    }>(packageJsonPath);
    let binPath = '';
    if (pkg.bin) {
      if (typeof pkg.bin === 'string') {
        binPath = path.join(localPackagePath, pkg.bin);
      } else {
        const binName = Object.keys(pkg.bin)[0];
        binPath = path.join(localPackagePath, pkg.bin[binName]);
      }
    }
    const args = [binPath, ...templateArgs];
    let type: TemplateType = TemplateType.remote;
    if (pkg.keywords?.includes('bingo-template') || !!pkg.dependencies?.bingo) {
      type = TemplateType.bingo;
      // add `--skip-requests` by default for bingo templates
      args.push('--skip-requests');
    }
    if (binPath) {
      return {
        command: 'node',
        args,
        envs,
        type,
        parentDir,
        interactive,
      };
    }
  }

  const expandedName = expandCreateShorthand(templateName);
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
 * - `@tanstack/start` → `@tanstack/create-start`
 * - `@tanstack/start@latest` → `@tanstack/create-start@latest`
 *
 * Special cases for packages where the convention doesn't work:
 * - `nitro` → `create-nitro-app` (create-nitro is abandoned)
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
): string | undefined {
  if (workspaceInfo.parentDirs.length === 0) {
    return;
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
  return;
}

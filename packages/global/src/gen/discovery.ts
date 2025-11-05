import path from 'node:path';

import { prependToPathToEnvs } from './command.ts';
import { BuiltinTemplate, type TemplateInfo, TemplateType, type WorkspaceInfo } from './types.ts';
import { readJsonFile } from './utils.ts';

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
        args: [degitPath, templateName, ...templateArgs],
        envs,
        type: TemplateType.remote,
        parentDir,
        interactive,
      };
    }
  }

  // Check for local package
  const localPackage = workspaceInfo.packages.find(pkg => pkg.name === templateName);
  if (localPackage) {
    const localPackagePath = path.join(workspaceInfo.rootDir, localPackage.path);
    const packageJsonPath = path.join(localPackagePath, 'package.json');
    const pkg = readJsonFile<
      { dependencies?: Record<string, string>; keywords?: string[]; bin?: Record<string, string> | string }
    >(packageJsonPath);
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

  return {
    command: templateName,
    args: [...templateArgs],
    envs,
    type: TemplateType.remote,
    parentDir,
    interactive,
  };
}

// Infer the parent directory of the generated package based on the template name
function inferParentDir(templateName: string, workspaceInfo: WorkspaceInfo): string | undefined {
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

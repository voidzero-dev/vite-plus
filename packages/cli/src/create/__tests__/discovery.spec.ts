import { describe, expect, it } from 'vitest';

import {
  discoverTemplate,
  expandCreateShorthand,
  inferGitHubRepoName,
  parseGitHubUrl,
} from '../discovery.js';

describe('expandCreateShorthand', () => {
  it('should expand unscoped names to create-* packages', () => {
    expect(expandCreateShorthand('vite')).toBe('create-vite');
    expect(expandCreateShorthand('next-app')).toBe('create-next-app');
    expect(expandCreateShorthand('nuxt')).toBe('create-nuxt');
    expect(expandCreateShorthand('vue')).toBe('create-vue');
  });

  it('should expand unscoped names with version', () => {
    expect(expandCreateShorthand('vite@latest')).toBe('create-vite@latest');
    expect(expandCreateShorthand('vite@5.0.0')).toBe('create-vite@5.0.0');
  });

  it('should expand scoped names to @scope/create-* packages', () => {
    expect(expandCreateShorthand('@tanstack/start')).toBe('@tanstack/create-start');
    expect(expandCreateShorthand('@my-org/app')).toBe('@my-org/create-app');
  });

  it('should expand scoped names with version', () => {
    expect(expandCreateShorthand('@tanstack/start@latest')).toBe('@tanstack/create-start@latest');
    expect(expandCreateShorthand('@tanstack/start@1.0.0')).toBe('@tanstack/create-start@1.0.0');
  });

  it('should not expand names already starting with create-', () => {
    expect(expandCreateShorthand('create-vite')).toBe('create-vite');
    expect(expandCreateShorthand('create-vite@latest')).toBe('create-vite@latest');
    expect(expandCreateShorthand('create-next-app')).toBe('create-next-app');
    expect(expandCreateShorthand('@tanstack/create-start')).toBe('@tanstack/create-start');
    expect(expandCreateShorthand('@tanstack/create-start@latest')).toBe(
      '@tanstack/create-start@latest',
    );
  });

  it('should not expand builtin templates (vite:*)', () => {
    expect(expandCreateShorthand('vite:monorepo')).toBe('vite:monorepo');
    expect(expandCreateShorthand('vite:application')).toBe('vite:application');
    expect(expandCreateShorthand('vite:library')).toBe('vite:library');
    expect(expandCreateShorthand('vite:generator')).toBe('vite:generator');
  });

  it('should not expand GitHub URLs', () => {
    expect(expandCreateShorthand('github:user/repo')).toBe('github:user/repo');
    expect(expandCreateShorthand('https://github.com/user/repo')).toBe(
      'https://github.com/user/repo',
    );
  });

  it('should not expand local paths', () => {
    expect(expandCreateShorthand('./local-template')).toBe('./local-template');
    expect(expandCreateShorthand('../parent-template')).toBe('../parent-template');
    expect(expandCreateShorthand('/absolute/path')).toBe('/absolute/path');
  });

  it('should handle scope-only input gracefully', () => {
    expect(expandCreateShorthand('@scope')).toBe('@scope');
  });

  it('should handle special cases where default convention does not apply', () => {
    expect(expandCreateShorthand('nitro')).toBe('create-nitro-app');
    expect(expandCreateShorthand('nitro@latest')).toBe('create-nitro-app@latest');
  });
});

describe('GitHub template helpers', () => {
  it('should parse GitHub shorthand URLs', () => {
    expect(parseGitHubUrl('github:user/repo')).toBe('user/repo');
  });

  it('should parse GitHub https URLs', () => {
    expect(parseGitHubUrl('https://github.com/user/repo')).toBe('user/repo');
    expect(parseGitHubUrl('https://github.com/user/repo.git')).toBe('user/repo');
  });

  it('should infer the repository name from GitHub templates', () => {
    expect(inferGitHubRepoName('github:nkzw-tech/fate-template')).toBe('fate-template');
    expect(inferGitHubRepoName('https://github.com/nkzw-tech/fate-template')).toBe('fate-template');
  });

  it('should resolve GitHub templates to degit without reusing the original URL as destination', () => {
    const template = discoverTemplate('https://github.com/nkzw-tech/fate-template', ['my-app'], {
      rootDir: '/tmp/workspace',
      isMonorepo: false,
      monorepoScope: '',
      workspacePatterns: [],
      parentDirs: [],
      packageManager: 'pnpm',
      packageManagerVersion: 'latest',
      downloadPackageManager: {
        binPrefix: '/tmp/bin',
        version: '10.0.0',
      } as never,
      packages: [],
    });

    expect(template.command).toBe('degit');
    expect(template.args).toEqual(['nkzw-tech/fate-template', 'my-app']);
  });
});

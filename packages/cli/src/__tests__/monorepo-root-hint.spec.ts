import { describe, expect, it } from 'vitest';

import {
  devCommandHasExplicitRoot,
  formatDevMonorepoRootHint,
  shouldWarnDevFromMonorepoRoot,
} from '../monorepo-root-hint.js';
import type { WorkspaceInfoOptional } from '../types/index.js';

const workspaceInfo: WorkspaceInfoOptional = {
  rootDir: '/repo',
  packageManager: undefined,
  packageManagerVersion: 'latest',
  isMonorepo: true,
  monorepoScope: '',
  workspacePatterns: ['apps/*'],
  parentDirs: ['apps'],
  packages: [
    {
      name: 'website',
      path: 'apps/website',
      version: '1.0.0',
      dependencies: [],
      scripts: { dev: 'vp dev' },
    },
  ],
};

describe('devCommandHasExplicitRoot', () => {
  it('detects a positional Vite root', () => {
    expect(devCommandHasExplicitRoot(['dev', 'apps/website'])).toBe(true);
  });

  it('ignores known flag values when looking for a positional root', () => {
    expect(devCommandHasExplicitRoot(['dev', '--host', '0.0.0.0', '--port', '3000'])).toBe(false);
  });

  it('handles equals-form flags', () => {
    expect(devCommandHasExplicitRoot(['dev', '--host=0.0.0.0', '--port=3000'])).toBe(false);
  });
});

describe('shouldWarnDevFromMonorepoRoot', () => {
  it('warns for bare dev from the monorepo root', () => {
    expect(shouldWarnDevFromMonorepoRoot('dev', ['dev'], '/repo', workspaceInfo)).toBe(true);
  });

  it('does not warn when dev targets a package directory', () => {
    expect(
      shouldWarnDevFromMonorepoRoot('dev', ['dev', 'apps/website'], '/repo', workspaceInfo),
    ).toBe(false);
  });

  it('does not warn outside the monorepo root', () => {
    expect(shouldWarnDevFromMonorepoRoot('dev', ['dev'], '/repo/apps/website', workspaceInfo)).toBe(
      false,
    );
  });

  it('does not warn for help output', () => {
    expect(shouldWarnDevFromMonorepoRoot('dev', ['dev', '--help'], '/repo', workspaceInfo)).toBe(
      false,
    );
  });
});

describe('formatDevMonorepoRootHint', () => {
  it('uses an existing workspace package as the example target', () => {
    expect(formatDevMonorepoRootHint(workspaceInfo)).toContain('vp dev apps/website');
  });
});

import { describe, expect, it } from 'vitest';

import type { WorkspacePackage } from '../../types/workspace.js';
import { getInitialTemplateOptions } from '../initial-template-options.js';

describe('getInitialTemplateOptions', () => {
  it('shows only built-in monorepo, application, and library options outside a monorepo', () => {
    expect(getInitialTemplateOptions(false)).toEqual([
      {
        label: 'Vite+ Monorepo',
        value: 'vite:monorepo',
        hint: 'Create a new Vite+ monorepo project',
      },
      {
        label: 'Vite+ Application',
        value: 'vite:application',
        hint: 'Create vite applications',
      },
      {
        label: 'Vite+ Library',
        value: 'vite:library',
        hint: 'Create vite libraries',
      },
    ]);
  });

  it('shows only built-in application and library options inside a monorepo', () => {
    expect(getInitialTemplateOptions(true)).toEqual([
      {
        label: 'Vite+ Application',
        value: 'vite:application',
        hint: 'Create vite applications',
      },
      {
        label: 'Vite+ Library',
        value: 'vite:library',
        hint: 'Create vite libraries',
      },
    ]);
  });

  // https://github.com/voidzero-dev/vite-plus: local generator packages
  // (scaffolded by `vp create vite:generator`) must be offered by the picker
  it('includes local template packages (generators) inside a monorepo', () => {
    const packages: WorkspacePackage[] = [
      {
        name: 'million-finding',
        path: 'tools/million-finding',
        description: 'Generate new components for our monorepo',
        version: '0.0.0',
        isTemplatePackage: true,
      },
      {
        name: 'utils',
        path: 'packages/utils',
        isTemplatePackage: false,
      },
    ];

    const options = getInitialTemplateOptions(true, packages);
    const values = options.map((option) => option.value);

    // Built-in templates are still offered
    expect(values).toContain('vite:application');
    expect(values).toContain('vite:library');
    // The local generator is offered and selectable by its package name
    expect(values).toContain('million-finding');
    // Regular workspace packages are not offered as templates
    expect(values).not.toContain('utils');

    const generatorOption = options.find((option) => option.value === 'million-finding');
    expect(generatorOption?.hint).toBe('Generate new components for our monorepo');
  });

  it('does not include local template packages outside a monorepo', () => {
    const packages: WorkspacePackage[] = [
      {
        name: 'million-finding',
        path: 'tools/million-finding',
        isTemplatePackage: true,
      },
    ];

    const values = getInitialTemplateOptions(false, packages).map((option) => option.value);
    expect(values).not.toContain('million-finding');
  });
});

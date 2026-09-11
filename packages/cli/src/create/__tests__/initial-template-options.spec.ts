import { describe, expect, it } from 'vitest';

import { getInitialTemplateOptions } from '../initial-template-options.js';
import type { CreateTemplateEntry } from '../org-manifest.js';

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

  it('lists local create.templates entries inside a monorepo, by name', () => {
    const templates: CreateTemplateEntry[] = [
      {
        name: 'component',
        description: 'Internal UI component',
        template: './tools/create-component',
      },
      { name: 'service', description: 'Backend service', template: 'service-generator' },
    ];

    const options = getInitialTemplateOptions(true, templates);
    const values = options.map((option) => option.value);

    // Built-in templates are still offered
    expect(values).toContain('vite:application');
    expect(values).toContain('vite:library');
    // Each local template is offered and selectable by its entry name
    expect(values).toContain('component');
    expect(values).toContain('service');

    const componentOption = options.find((option) => option.value === 'component');
    expect(componentOption?.label).toBe('component');
    expect(componentOption?.hint).toBe('Internal UI component');
  });

  it('does not include local templates outside a monorepo', () => {
    const templates: CreateTemplateEntry[] = [
      { name: 'component', description: 'Internal UI component', template: 'component-generator' },
    ];

    const values = getInitialTemplateOptions(false, templates).map((option) => option.value);
    expect(values).not.toContain('component');
  });
});

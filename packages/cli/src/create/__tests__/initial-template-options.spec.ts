import { describe, expect, it } from 'vitest';

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
});

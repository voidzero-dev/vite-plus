import { afterEach, describe, expect, it, vi } from 'vitest';

import type { OrgManifest } from '../org-manifest.js';
import {
  formatManifestTable,
  ORG_PICKER_BUILTIN_ESCAPE,
  ORG_PICKER_CANCEL,
  pickOrgTemplate,
} from '../org-picker.js';

const { mockSelect, mockIsCancel } = vi.hoisted(() => ({
  mockSelect: vi.fn(),
  mockIsCancel: vi.fn((value: unknown) => value === '__cancel__'),
}));

vi.mock('@voidzero-dev/vite-plus-prompts', () => ({
  select: mockSelect,
  isCancel: mockIsCancel,
}));

function manifest(overrides?: Partial<OrgManifest>): OrgManifest {
  return {
    scope: '@your-org',
    packageName: '@your-org/create',
    version: '1.0.0',
    tarballUrl: 'https://example/create-1.0.0.tgz',
    integrity: 'sha512-fake',
    templates: [
      { name: 'monorepo', description: 'Full scaffold', template: './m', monorepo: true },
      { name: 'web', description: 'Web app', template: '@your-org/template-web' },
      { name: 'library', description: 'Library', template: '@your-org/template-library' },
    ],
    ...overrides,
  };
}

describe('pickOrgTemplate', () => {
  afterEach(() => {
    mockSelect.mockReset();
    mockIsCancel.mockClear();
  });

  it('appends a built-in escape-hatch entry as the last option', async () => {
    mockSelect.mockResolvedValue('web');
    await pickOrgTemplate(manifest(), { isMonorepo: false });
    const passedOptions = mockSelect.mock.calls[0][0].options;
    expect(passedOptions.map((o: { value: string }) => o.value).slice(0, -1)).toEqual([
      'monorepo',
      'web',
      'library',
    ]);
    expect(passedOptions.at(-1)).toMatchObject({ label: 'Vite+ built-in templates' });
  });

  it('filters monorepo:true entries when isMonorepo is true', async () => {
    mockSelect.mockResolvedValue('web');
    await pickOrgTemplate(manifest(), { isMonorepo: true });
    const passedOptions = mockSelect.mock.calls[0][0].options;
    expect(passedOptions.map((o: { value: string }) => o.value).slice(0, -1)).toEqual([
      'web',
      'library',
    ]);
    expect(passedOptions.at(-1)).toMatchObject({ label: 'Vite+ built-in templates' });
  });

  it('returns the entry for a non-escape selection', async () => {
    mockSelect.mockResolvedValue('web');
    const result = await pickOrgTemplate(manifest(), { isMonorepo: false });
    expect(result).toEqual({
      kind: 'entry',
      entry: expect.objectContaining({ name: 'web' }),
    });
  });

  it('returns the escape-hatch sentinel when the escape entry is picked', async () => {
    // Emulate `select` resolving with whatever value the picker assigned
    // to its escape-hatch option. If the option isn't in the list at all,
    // the assertion below fails.
    mockSelect.mockImplementation(
      async (opts: { options: { value: string; label: string }[] }) =>
        opts.options.find((o) => o.label === 'Vite+ built-in templates')?.value,
    );
    expect(await pickOrgTemplate(manifest(), { isMonorepo: false })).toBe(
      ORG_PICKER_BUILTIN_ESCAPE,
    );
  });

  it('returns the cancel sentinel when the prompt is cancelled', async () => {
    mockSelect.mockResolvedValue('__cancel__');
    expect(await pickOrgTemplate(manifest(), { isMonorepo: false })).toBe(ORG_PICKER_CANCEL);
  });

  it('returns the escape-hatch sentinel when every entry is filtered out', async () => {
    const allMonorepo = manifest({
      templates: [
        { name: 'a', description: 'a', template: './a', monorepo: true },
        { name: 'b', description: 'b', template: './b', monorepo: true },
      ],
    });
    const result = await pickOrgTemplate(allMonorepo, { isMonorepo: true });
    expect(result).toBe(ORG_PICKER_BUILTIN_ESCAPE);
    expect(mockSelect).not.toHaveBeenCalled();
  });
});

describe('formatManifestTable', () => {
  it('renders a stable, whitespace-aligned table', () => {
    const { lines, filteredCount } = formatManifestTable(manifest(), false);
    expect(filteredCount).toBe(0);
    expect(lines[0]).toMatch(/^ {2}NAME\s+DESCRIPTION\s+TEMPLATE/);
    // Every row includes name, description, and template specifier.
    expect(lines[1]).toMatch(/monorepo\s+Full scaffold\s+\.\/m/);
    expect(lines[2]).toMatch(/web\s+Web app\s+@your-org\/template-web/);
  });

  it('filters monorepo entries inside a monorepo and reports the count', () => {
    const { lines, filteredCount } = formatManifestTable(manifest(), true);
    expect(filteredCount).toBe(1);
    expect(lines.some((line) => line.includes('monorepo '))).toBe(false);
    expect(lines.some((line) => line.includes('web'))).toBe(true);
  });
});

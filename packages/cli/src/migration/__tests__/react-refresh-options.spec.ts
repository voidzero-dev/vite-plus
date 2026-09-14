import fs from 'node:fs';

import { type OxlintConfig } from 'oxlint';
import { describe, expect, it } from 'vitest';

import { sanitizeMigratedOxlintConfig } from '../migrator.ts';
import { createMigrationReport } from '../report.ts';

describe('React Refresh option migration', () => {
  it('keeps the workaround tied to the bundled Oxlint schema', () => {
    const schema = JSON.parse(
      fs.readFileSync(
        new URL('configuration_schema.json', import.meta.resolve('oxlint/package.json')),
        'utf8',
      ),
    );
    // When Oxlint implements this option, preserve it during migration.
    expect(schema.definitions.OnlyExportComponentsConfig.properties).not.toHaveProperty(
      'allowCompoundComponents',
    );
  });

  it.each([true, false])('removes the unsupported option when it is %s', (value) => {
    // Model the invalid JSON emitted by @oxlint/migrate.
    const config = {
      rules: {
        'react/only-export-components': [
          'warn',
          { allowCompoundComponents: value, allowConstantExport: true, checkJS: true },
        ],
      },
      overrides: [
        {
          files: ['*.tsx'],
          rules: {
            'react/only-export-components': ['error', { allowCompoundComponents: value }],
          },
        },
      ],
    } as unknown as OxlintConfig;
    const report = createMigrationReport();

    sanitizeMigratedOxlintConfig(config, new Set(), report);

    expect(config.rules?.['react/only-export-components']).toEqual([
      'warn',
      { allowConstantExport: true, checkJS: true },
    ]);
    expect(config.overrides?.[0].rules?.['react/only-export-components']).toEqual(['error', {}]);
    expect(report.warnings).toEqual([
      'The bundled Oxlint does not support react/only-export-components.allowCompoundComponents. ' +
        'Removed this option from the migrated config; compound component exports may now report lint errors.',
    ]);
  });

  it('preserves supported options, other rules, and rules without options', () => {
    const config = {
      rules: { 'react/only-export-components': ['error', { allowConstantExport: true }] },
      overrides: [
        { files: ['*.js'], rules: { 'react/only-export-components': 'off' } },
        { files: ['*.jsx'], rules: { 'react/only-export-components': ['warn'] } },
        {
          files: ['*.tsx'],
          jsPlugins: [{ name: 'custom', specifier: './plugin.js' }],
          rules: { 'custom/rule': ['error', { allowCompoundComponents: true }] },
        },
      ],
    } as OxlintConfig;
    const expected = structuredClone(config);
    const report = createMigrationReport();

    sanitizeMigratedOxlintConfig(config, new Set(), report);

    expect(config).toEqual(expected);
    expect(report.warnings).toEqual([]);
  });
});

import fs from 'node:fs';

import { type OxlintConfig } from 'oxlint';
import { describe, expect, it } from 'vitest';

import { sanitizeMigratedOxlintConfig } from '../migrator.ts';
import { createMigrationReport } from '../report.ts';

describe('React Refresh option migration', () => {
  it('matches the bundled Oxlint schema, which supports the option', () => {
    const schema = JSON.parse(
      fs.readFileSync(
        new URL('configuration_schema.json', import.meta.resolve('oxlint/package.json')),
        'utf8',
      ),
    );
    // The bundled Oxlint implements this option, so migration preserves it.
    expect(schema.definitions.OnlyExportComponentsConfig.properties).toHaveProperty(
      'allowCompoundComponents',
    );
  });

  it.each([true, false])('preserves the supported option when it is %s', (value) => {
    // Model the JSON emitted by @oxlint/migrate.
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
    const expected = structuredClone(config);
    const report = createMigrationReport();

    sanitizeMigratedOxlintConfig(config, new Set(), report);

    expect(config).toEqual(expected);
    expect(report.warnings).toEqual([]);
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

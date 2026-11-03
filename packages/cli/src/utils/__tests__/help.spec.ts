import { stripVTControlCharacters } from 'node:util';

import { describe, expect, it, vi } from 'vitest';

import { renderCliDoc } from '../help.js';

describe('renderCliDoc', () => {
  it('renders usage, rows, and sections with stable spacing', () => {
    const output = renderCliDoc(
      {
        usage: 'vp demo <name>',
        summary: 'Create a demo project.',
        sections: [
          {
            title: 'Arguments',
            rows: [
              {
                label: '<name>',
                description: ['Project name', 'Must be kebab-case'],
              },
            ],
          },
          {
            title: 'Options',
            rows: [{ label: '-h, --help', description: 'Print help' }],
          },
          {
            title: 'Examples',
            lines: ['  vp demo my-app'],
          },
        ],
      },
      { color: false },
    );

    expect(output).toMatchInlineSnapshot(`
      "Usage: vp demo <name>

      Create a demo project.

      Arguments:
        <name>  Project name
                Must be kebab-case

      Options:
        -h, --help  Print help

      Examples:
        vp demo my-app
      "
    `);
  });

  it('renders section-only documents without usage prelude', () => {
    const output = renderCliDoc(
      {
        sections: [
          {
            title: 'Package Versions',
            rows: [{ label: 'global vite-plus', description: 'v0.1.0' }],
          },
        ],
      },
      { color: false },
    );

    expect(output).toMatchInlineSnapshot(`
      "Package Versions:
        global vite-plus  v0.1.0
      "
    `);
  });

  it('aligns wrapped help text within the terminal width', () => {
    Object.defineProperty(process.stdout, 'columns', { configurable: true, value: 40 });

    try {
      const output = renderCliDoc(
        {
          sections: [
            {
              title: 'Details',
              lines: ['  * `all`  - Include every category except one.'],
            },
            {
              title: 'Options',
              rows: [
                {
                  label: '--config=<path>',
                  description: 'Override the configuration file used for import resolution.',
                },
              ],
            },
          ],
        },
        { color: false },
      );

      expect(output).toMatchInlineSnapshot(`
        "Details:
          * \`all\`  - Include every category
          except one.

        Options:
          --config=<path>  Override the
                           configuration
                           file used for
                           import
                           resolution.
        "
      `);
    } finally {
      Reflect.deleteProperty(process.stdout, 'columns');
    }
  });

  it('renders documentation footer when present', () => {
    const output = renderCliDoc({
      usage: 'vp demo',
      documentationUrl: 'https://viteplus.dev/guide/demo',
      sections: [{ title: 'Arguments', rows: [{ label: '<name>', description: 'Project name' }] }],
    });

    expect(stripVTControlCharacters(output)).toContain(
      'Documentation: https://viteplus.dev/guide/demo',
    );
  });

  it('renders inline example comments in a muted color', () => {
    const noColor = process.env.NO_COLOR;
    delete process.env.NO_COLOR;
    vi.stubEnv('FORCE_COLOR', '1');

    try {
      const output = renderCliDoc({
        sections: [
          {
            title: 'Examples',
            lines: ['  vp exec node --version  # Run local node'],
          },
        ],
      });

      expect(output).toContain('\u001B[90m # Run local node\u001B[39m');
    } finally {
      vi.unstubAllEnvs();
      if (noColor !== undefined) {
        process.env.NO_COLOR = noColor;
      }
    }
  });
});

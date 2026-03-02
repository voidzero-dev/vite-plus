import { describe, expect, it } from 'vitest';

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
});

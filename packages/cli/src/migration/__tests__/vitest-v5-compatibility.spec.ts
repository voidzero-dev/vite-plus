import fs from 'node:fs';
import { runInNewContext } from 'node:vm';

import { describe, expect, it } from 'vitest';

import { parseSource, SourceEditor } from '../vitest-v5/ast.ts';
import { migrateVitestV5Config } from '../vitest-v5/config.ts';

const options = { preserveV4: true, temporalPolyfill: true };
const migrate = (source: string) => migrateVitestV5Config('vitest.config.mjs', source, options);
const evaluate = (source: string) => runInNewContext(source.replace('export default', ''));

describe('Vitest v4 compatibility comments', () => {
  it.each([
    ['export default {}', 'clearMocks: false', 'clearmocks-is-enabled-by-default'],
    ['export default { test: {} }', 'clearMocks: false', 'clearmocks-is-enabled-by-default'],
    [
      'export default { test: { projects: [{ test: {} }] } }',
      'sharedViteServer: false',
      'inline-projects-share-the-vite-server-by-default',
    ],
    [
      'export default { test: { projects: [{}] } }',
      'extends: false',
      'inline-projects-inherit-the-root-config-by-default',
    ],
    ['export default { test: { browser: {} } }', 'exact: false', 'locators-are-strict-by-default'],
    [
      'export default { test: { browser: { locators: {} } } }',
      'exact: false',
      'locators-are-strict-by-default',
    ],
    [
      "export default { test: { coverage: { thresholds: { perFile: true, 'src/**': { lines: 90 } } } } }",
      'perFile: true',
      'glob-coverage-thresholds-no-longer-inherit-perfile',
    ],
    ...[
      'export default {}',
      'export default { test: {} }',
      'export default { test: { fakeTimers: {} } }',
    ].map((source) => [
      source,
      "toNotFake: ['Temporal']",
      'fake-timers-and-setsystemtime-now-mock-temporal',
    ]),
  ])('documents a newly inserted %s setting: %s', (source, setting, section) => {
    const result = migrate(source);
    expect(result.findings).toEqual([]);
    const comments = parseSource('vitest.config.mjs', result.content).comments;
    const link = `// https://vitest.dev/guide/migration/#${section}`;
    const index = comments.findIndex((span) => result.content.slice(span.start, span.end) === link);
    expect(index).toBeGreaterThanOrEqual(3);
    expect(result.content.slice(comments[index - 3].start, comments[index - 3].end)).toMatch(
      /^\/\/ Vitest v4 compatibility: .+\.$/,
    );
    expect(result.content.slice(comments[index - 2].start, comments[index - 2].end)).toMatch(
      /^\/\/ (Remove|Keep) .+\.$/,
    );
    expect(result.content.slice(comments[index - 1].start, comments[index - 1].end)).toBe(
      '// https://viteplus.dev/guide/vitest-v5#remove-unneeded-compatibility-settings',
    );
    expect(result.content.slice(comments[index].end).trimStart().startsWith(setting)).toBe(true);
    expect(migrate(result.content)).toEqual(result);
    expect(
      migrateVitestV5Config('vitest.config.mjs', result.content, { preserveV4: false }),
    ).toEqual(result);
    const withoutComments = result.content.replace(/^[\t ]*\/\/.*(?:\r?\n|$)/gm, '');
    expect(migrate(withoutComments)).toEqual({ content: withoutComments, findings: [] });
    const guide = fs.readFileSync(
      new URL('../../../../../docs/guide/vitest-v5.md', import.meta.url),
      'utf8',
    );
    expect(guide).toContain(link.slice(3));
  });

  it.each([
    "['json' /* Report consumer. */]",
    "['json', /* Report consumer. */]",
    "['json' // Report consumer.\n]",
  ])('retains comments in a reporter tuple: %s', (reporter) => {
    const input = `export default ({ test: { reporters: [${reporter}] } });`;
    const result = migrate(input);
    expect(result.findings).toEqual([]);
    expect(result.content).toContain('Report consumer.');
    expect(result.content).toContain(`reporters: [${reporter}]`);
    expect(result.content).not.toContain('stdout');
    expect(evaluate(result.content).test.reporters).toEqual([['json']]);
    expect(migrate(result.content)).toEqual(result);
  });

  it.each(['\n', '\r\n'])(
    'preserves settings and line endings through nested insertion (%j)',
    (newline) => {
      const input = `export default ({
  test: {
    // Keep this project comment.
    projects: [{}],
    browser: {},
    reporters: ['json', ['junit', {}]],
    coverage: { thresholds: { perFile: true, 'src/**': { lines: 90 } } },
  },
});`.replaceAll('\n', newline);
      const result = migrate(input);
      expect(result.findings).toEqual([]);
      expect(result.content).toContain('// Keep this project comment.');
      if (newline === '\r\n') {
        expect(result.content.replaceAll('\r\n', '')).not.toContain('\n');
      }
      expect(evaluate(result.content)).toEqual({
        test: {
          clearMocks: false,
          sharedViteServer: false,
          fakeTimers: { toNotFake: ['Temporal'] },
          projects: [
            {
              extends: false,
              test: { clearMocks: false, fakeTimers: { toNotFake: ['Temporal'] } },
            },
          ],
          browser: { locators: { exact: false } },
          reporters: ['json', ['junit', {}]],
          coverage: { thresholds: { perFile: true, 'src/**': { perFile: true, lines: 90 } } },
        },
      });
      expect(migrate(result.content)).toEqual(result);
    },
  );

  it.each(['true', 'false'])(
    'leaves explicit settings and their comments untouched (%s)',
    (value) => {
      const input = `export default {
  test: {
    // Our mock policy.
    clearMocks: ${value},
    sharedViteServer: ${value},
    projects: [{ extends: ${value}, test: { clearMocks: ${value}, fakeTimers: { toNotFake: [] } } }],
    browser: { locators: { exact: ${value} } },
    reporters: [['json', { stdout: ${value} }]],
    coverage: { thresholds: { perFile: true, 'src/**': { perFile: ${value}, lines: 90 } } },
    fakeTimers: { toNotFake: ['Date'] },
  },
};`;
      expect(migrate(input)).toEqual({ content: input, findings: [] });
    },
  );

  it('indents generated comments inside compact inline projects', () => {
    const input = `export default {
  test: {
    projects: [{ test: { name: 'unit' } }],
  },
};`;
    const result = migrate(input);
    expect(result.findings).toEqual([]);
    expect(result.content).toContain(
      '\n      extends: false,\n      test: {\n        // Vitest v4 compatibility: preserve mock call history.',
    );
    expect(result.content).toContain("\n        name: 'unit'");
    expect(migrate(result.content)).toEqual(result);
  });

  it('does not annotate permanent API moves or v5 configs', () => {
    const input = 'export default { test: { browser: { api: { port: 1234 } }, projects: [{}] } };';
    const result = migrateVitestV5Config('vitest.config.mjs', input, { preserveV4: false });
    expect(result.findings).toEqual([]);
    expect(result.content).toContain('api: { port: 1234 }');
    expect(result.content).not.toContain('Vitest v4 compatibility');
  });

  it('does not reindent copied multiline expressions when adding comments', () => {
    const input = 'export default ({ test: { value: `first\n  second` } });';
    const result = migrate(input);
    expect(result.findings).toEqual([]);
    expect(evaluate(result.content).test.value).toBe('first\n  second');
    const editor = new SourceEditor('vitest.config.mjs', 'export default ({});');
    editor.visit({
      ObjectExpression(node) {
        editor.add(node, 'copied', '`first\n  second`', '// Preserve this expression.');
      },
    });
    const added = editor.finish();
    expect(added.findings).toEqual([]);
    expect(evaluate(added.content).copied).toBe('first\n  second');
  });
});

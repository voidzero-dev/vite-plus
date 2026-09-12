import { describe, expect, it } from 'vitest';

import { SourceEditor, parseSource } from '../vitest-v5/ast.ts';
import { migrateVitestV5Config } from '../vitest-v5/config.ts';
import { migrateVitestV5Source } from '../vitest-v5/source.ts';

const options = { preserveV4: true, browser: true, globals: true };

describe('Oxc migration analysis', () => {
  it.each([
    ['test.js', 'const element = <div />;'],
    ['test.jsx', 'export const element = <div />;'],
    ['test.ts', 'const identity = <T>(value: T): T => value;'],
    ['test.tsx', 'const element = <div />; interface Props { value: string }'],
    ['test.mts', 'export const value: number = 1;'],
    ['test.cts', 'const value: number = 1; module.exports = value;'],
    ['test.js', '@decorate class Example { method() {} }'],
    ['test.ts', 'const value = 123n; const expression = /partial/v;'],
  ])('parses %s syntax without another parser dependency', (file, content) => {
    expect(parseSource(file, content).program.type).toBe('Program');
  });

  it('rejects Flow and malformed syntax instead of returning a partial AST', () => {
    expect(() => parseSource('test.js', '// @flow\nconst value: string = "x";')).toThrow(
      'Flow is not supported',
    );
    expect(() => parseSource('test.ts', 'const = ;')).toThrow();
  });

  it.each([
    [String.raw`'\ud800'`, '\ud800'],
    [String.raw`'\udc00'`, '\udc00'],
    [String.raw`'\ud800x'`, '\ud800x'],
    [String.raw`'\u{d800}'`, '\ud800'],
    [String.raw`'\ud800\udc00'`, '\ud800\udc00'],
  ])('preserves the value and source spelling of %s', (literal, value) => {
    const declaration = `const value = ${literal};`;
    expect(parseSource('test.ts', declaration).program.body[0]).toMatchObject({
      declarations: [{ init: { type: 'Literal', value } }],
    });
    const input = `${declaration}\nexpect(() => {}).toThrow('');`;
    expect(migrateVitestV5Source('test.ts', input, options)).toEqual({
      content: `${declaration}\nexpect(() => {}).toThrow(/^$/);`,
      findings: [],
    });
  });

  it('preserves unpaired surrogates in template values', () => {
    const declaration = 'const value = `\\ud800`;';
    expect(parseSource('test.ts', declaration).program.body[0]).toMatchObject({
      declarations: [{ init: { quasis: [{ value: { cooked: '\ud800' } }] } }],
    });
    const input = `${declaration}\nexpect(() => {}).toThrow('');`;
    expect(migrateVitestV5Source('test.ts', input, options)).toEqual({
      content: `${declaration}\nexpect(() => {}).toThrow(/^$/);`,
      findings: [],
    });
  });

  it('migrates ASTs deeper than the Rust JSON recursion limit', () => {
    const declaration = `const value = ${Array(160).fill('1').join(' + ')};`;
    const input = `${declaration}\nexpect(() => {}).toThrow('');`;
    expect(migrateVitestV5Source('test.ts', input, options)).toEqual({
      content: `${declaration}\nexpect(() => {}).toThrow(/^$/);`,
      findings: [],
    });
  });

  it('preserves shadowed globals in destructuring, catch clauses, and hoisted declarations', () => {
    const shadowed = `function parameter({ expect }) { expect(() => {}).toThrow(''); }
try {} catch (expect) { expect(() => {}).toThrow(''); }
function hoisted() { expect(() => {}).toThrow(''); var expect; }
{ expect(() => {}).toThrow(''); let expect; }`;
    const input = `${shadowed}\nexpect(() => {}).toThrow('');`;
    const result = migrateVitestV5Source('test.ts', input, options);
    expect(result.content).toBe(`${shadowed}\nexpect(() => {}).toThrow(/^$/);`);
    expect(result.findings).toEqual([]);
  });

  it('does not follow reassigned runner aliases', () => {
    const input = `import { createVitest } from 'vitest/node';
let runner = await createVitest('test', {});
runner = other;
runner.collect();`;
    const result = migrateVitestV5Source('test.ts', input, options);
    expect(result.content).toBe(input);
    expect(result.findings).toEqual([expect.objectContaining({ code: 'static-collect' })]);
  });

  it('resolves constructor references before and after their declaration', () => {
    const input = `import { vi } from 'vitest';
new mock(); const mock = vi.fn(); new mock();`;
    const result = migrateVitestV5Source('test.ts', input, options);
    expect(result.findings).toEqual([expect.objectContaining({ code: 'class-mock' })]);
  });

  it('reserves generated names across bindings, unresolved references, and earlier edits', () => {
    const input = `import { getFn } from '@vitest/runner';
import { getHooks } from 'vitest/suite';
const _VitestTestRunner = 1; use(_VitestTestRunner2);`;
    const result = migrateVitestV5Source('test.ts', input, options);
    expect(result.content).toContain('TestRunner as _VitestTestRunner3');
    expect(result.content).toContain('TestRunner as _VitestTestRunner4');
    expect(result.findings).toEqual([]);
    expect(parseSource('test.ts', result.content).program.type).toBe('Program');
  });

  it.each([
    `import type * as v from 'vitest'; v.expect(() => {}).toThrow('');`,
    `import type * as expect from 'vitest'; expect(() => {}).toThrow('');`,
    `import type { expect } from 'vitest'; expect(() => {}).toThrow('');`,
    `import { type expect } from 'vitest'; expect(() => {}).toThrow('');`,
  ])('does not mistake a type-only import for a runtime API or global: %s', (input) => {
    expect(migrateVitestV5Source('test.ts', input, options).content).toBe(input);
  });

  it.each([
    `import { expect } from 'vitest'; expect?.(() => {}).toThrow('');`,
    `import { expect } from 'vitest'; expect(() => {})?.toThrow('');`,
    `import * as v from 'vitest'; v?.expect(() => {}).toThrow('');`,
  ])('preserves optional API calls: %s', (input) => {
    expect(migrateVitestV5Source('test.ts', input, options).content).toBe(input);
  });

  it('retains dynamic imports, CommonJS imports, and TS import-type diagnostics', () => {
    const input = `const runner = await import('@vitest/runner');
const other = require('vitest/runners');
type Runner = import('vitest/internal/module-runner').ModuleRunner;
function local(require) { return require('vitest/runners'); }`;
    const result = migrateVitestV5Source('test.ts', input, options);
    expect(result.content).toBe(input);
    expect(result.findings.map(({ code, severity }) => [code, severity])).toEqual([
      ['removed-api', 'block'],
      ['removed-api', 'block'],
      ['removed-api', 'review'],
    ]);
  });

  it('preserves non-ASCII text and reports UTF-16 columns with CRLF line endings', () => {
    const input = `// 中文 😀\r\nimport { expect } from 'vitest';\r\nconst label = '😀'; expect.poll(() => label).toBe('x');`;
    const result = migrateVitestV5Source('test.ts', input, options);
    expect(result.content).toBe(input);
    expect(result.findings).toContainEqual(
      expect.objectContaining({
        code: 'poll-timeout',
        line: 3,
        column: "const label = '😀'; ".length + 1,
      }),
    );
    const edited = migrateVitestV5Source(
      'test.ts',
      `${input}\r\nexpect(() => {}).toThrow('');`,
      options,
    );
    expect(edited.content).toBe(`${input}\r\nexpect(() => {}).toThrow(/^$/);`);
  });

  it.each([
    'api: { port: 1 } /* keep , 😀 */, enabled: true',
    'enabled: true, /* keep , 😀 */ api: { port: 1 }',
    "enabled: true, api: { port: 1 }, /* keep , 😀 */ name: 'browser'",
    'api: { port: 1 } /* keep , 😀 */,',
    'api: { port: 1 } // keep , 😀\n, enabled: true',
  ])('removes property punctuation without consuming comments: %s', (properties) => {
    const input = `// 😀\nexport default { test: { browser: { ${properties} } } };`;
    const result = migrateVitestV5Config('vite.config.ts', input, { preserveV4: false });
    expect(result.findings).toEqual([]);
    expect(result.content).toContain('keep , 😀');
    expect(result.content.match(/api:/g)).toHaveLength(1);
    expect(parseSource('vite.config.ts', result.content).program.type).toBe('Program');
    expect(
      migrateVitestV5Config('vite.config.ts', result.content, { preserveV4: false }).content,
    ).toBe(result.content);
  });

  it('does not treat methods or getters as static config properties', () => {
    const input = 'export default { test: { get browser() { return settings; } } };';
    expect(migrateVitestV5Config('vite.config.ts', input, options).content).toBe(input);
  });

  it('returns the original source when an edit would produce invalid syntax', () => {
    const editor = new SourceEditor('test.ts', 'const value = 1;');
    editor.edit(0, 5, 'const =');
    expect(editor.finish()).toEqual({
      content: 'const value = 1;',
      findings: [expect.objectContaining({ code: 'unsafe-syntax' })],
    });
  });
});

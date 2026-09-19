import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import path from 'node:path';
import ts from 'typescript';

const file = path.resolve('virtual-jest-dom-type-probe.ts');
assert.equal(existsSync('node_modules/vite-plus'), false);
const options = {
  target: ts.ScriptTarget.ESNext,
  module: ts.ModuleKind.Preserve,
  moduleResolution: ts.ModuleResolutionKind.Bundler,
  strict: true,
  skipLibCheck: true,
  noEmit: true,
  types: [],
};
for (const browserFirst of [true, false]) {
  const imports = ['vitest/browser', '@testing-library/jest-dom/vitest'];
  if (!browserFirst) imports.reverse();
  const source = imports.map((name) => `import '${name}';`).join('\n') + `
    import { expect } from 'vitest';
    import { page } from 'vitest/browser';
    page.getByRole('button');
    expect(document.body).toHaveTextContent(/text/);
    expect(document.body).toHaveStyle({ '--custom': '1px' });
  `;
  const host = ts.createCompilerHost(options);
  const getSourceFile = host.getSourceFile.bind(host);
  host.getSourceFile = (name, language, ...rest) =>
    name.replaceAll('\\', '/') === file.replaceAll('\\', '/')
      ? ts.createSourceFile(name, source, language, true)
      : getSourceFile(name, language, ...rest);
  const program = ts.createProgram([file], options, host);
  assert.ok(program.getSourceFiles().some(({ fileName }) =>
    /@vitest\/browser\/matchers\.d\.ts$/.test(fileName.replaceAll('\\', '/')),
  ));
  const diagnostics = ts.getPreEmitDiagnostics(program);
  assert.deepEqual(diagnostics.map(({ code }) => code), browserFirst ? [2345, 2353] : []);
  if (browserFirst) {
    assert.match(ts.flattenDiagnosticMessageText(diagnostics[0].messageText, ' '), /RegExp/);
    assert.match(ts.flattenDiagnosticMessageText(diagnostics[1].messageText, ' '), /CSSStyleDeclaration/);
  }
  console.log(`vitest: browser-first=${browserFirst}, Node jest-dom type errors=${diagnostics.length}`);
}
console.log('Release blocker reproduced: browser and jest-dom declaration order changes Node matcher types');

import { describe, expect, it } from 'vitest';

import { rewriteVitePlusTestSpecifier } from '../define-config.ts';

describe('rewriteVitePlusTestSpecifier', () => {
  it('is a no-op when source does not mention vite-plus/test', () => {
    const code = "import { describe } from 'vitest';\nimport * as fs from 'node:fs';\n";
    expect(rewriteVitePlusTestSpecifier(code)).toBe(code);
  });

  it("rewrites `from 'vite-plus/test'` to `from 'vitest'`", () => {
    const input = "import { vi } from 'vite-plus/test';\nvi.mock('./foo');\n";
    const expected = "import { vi } from 'vitest';\nvi.mock('./foo');\n";
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it('rewrites the double-quoted form too', () => {
    const input = 'import { vi } from "vite-plus/test";\n';
    const expected = 'import { vi } from "vitest";\n';
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it('does NOT rewrite subpaths such as vite-plus/test/browser', () => {
    const input = "import { context } from 'vite-plus/test/browser';\n";
    expect(rewriteVitePlusTestSpecifier(input)).toBe(input);
  });

  it('does NOT rewrite a bare string literal containing vite-plus/test', () => {
    const input = "const x = 'vite-plus/test';\nconsole.log(x);\n";
    expect(rewriteVitePlusTestSpecifier(input)).toBe(input);
  });

  it("rewrites dynamic `import('vite-plus/test')`", () => {
    const input = "const mod = await import('vite-plus/test');\n";
    const expected = "const mod = await import('vitest');\n";
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it("rewrites `require('vite-plus/test')` while leaving the subpath form alone", () => {
    const input = [
      "const a = require('vite-plus/test');",
      "const b = require('vite-plus/test/browser');",
      '',
    ].join('\n');
    const expected = [
      "const a = require('vitest');",
      "const b = require('vite-plus/test/browser');",
      '',
    ].join('\n');
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });

  it('preserves all other imports in the file', () => {
    const input = [
      "import { describe, it, expect } from 'vite-plus/test';",
      "import * as fs from 'node:fs';",
      "import { something } from 'vite-plus/test/browser';",
      '',
    ].join('\n');
    const expected = [
      "import { describe, it, expect } from 'vitest';",
      "import * as fs from 'node:fs';",
      "import { something } from 'vite-plus/test/browser';",
      '',
    ].join('\n');
    expect(rewriteVitePlusTestSpecifier(input)).toBe(expected);
  });
});

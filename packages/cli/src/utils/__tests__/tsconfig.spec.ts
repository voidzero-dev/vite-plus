import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import {
  findTsconfigFiles,
  removeDeprecatedTsconfigFalseOption,
  rewriteTypesInTsconfig,
} from '../tsconfig.js';

describe('findTsconfigFiles', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'tsconfig-test-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('finds all tsconfig variants', () => {
    fs.writeFileSync(path.join(tmpDir, 'tsconfig.json'), '{}');
    fs.writeFileSync(path.join(tmpDir, 'tsconfig.app.json'), '{}');
    fs.writeFileSync(path.join(tmpDir, 'tsconfig.node.json'), '{}');
    fs.writeFileSync(path.join(tmpDir, 'tsconfig.build.json'), '{}');
    fs.writeFileSync(path.join(tmpDir, 'other.json'), '{}');
    fs.writeFileSync(path.join(tmpDir, 'package.json'), '{}');

    const files = findTsconfigFiles(tmpDir);
    const expected = [
      path.join(tmpDir, 'tsconfig.app.json'),
      path.join(tmpDir, 'tsconfig.build.json'),
      path.join(tmpDir, 'tsconfig.json'),
      path.join(tmpDir, 'tsconfig.node.json'),
    ];
    expect(new Set(files)).toEqual(new Set(expected));
    expect(files).toHaveLength(4);
  });

  it('returns empty array for non-existent directory', () => {
    expect(findTsconfigFiles('/non-existent-dir-12345')).toEqual([]);
  });

  it('returns empty array when no tsconfig files exist', () => {
    fs.writeFileSync(path.join(tmpDir, 'package.json'), '{}');
    expect(findTsconfigFiles(tmpDir)).toEqual([]);
  });
});

describe.each(['esModuleInterop', 'allowSyntheticDefaultImports'])(
  'removeDeprecatedTsconfigFalseOption — %s',
  (option) => {
    let tmpDir: string;

    beforeEach(() => {
      tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'tsconfig-test-'));
    });

    afterEach(() => {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    });

    function writeAndRemove(filePath: string, content: string): string {
      fs.writeFileSync(filePath, content);
      const result = removeDeprecatedTsconfigFalseOption(filePath, option);
      expect(result).toBe(true);
      return fs.readFileSync(filePath, 'utf-8');
    }

    it('removes option: false (middle property)', () => {
      const filePath = path.join(tmpDir, 'tsconfig.json');
      expect(
        writeAndRemove(
          filePath,
          `{
  "compilerOptions": {
    "target": "ES2023",
    "${option}": false,
    "strict": true
  }
}`,
        ),
      ).toMatchInlineSnapshot(`
        "{
          "compilerOptions": {
            "target": "ES2023",
            "strict": true
          }
        }"
      `);
    });

    it('preserves comments in JSONC', () => {
      const filePath = path.join(tmpDir, 'tsconfig.json');
      expect(
        writeAndRemove(
          filePath,
          `{
  // This is a comment
  "compilerOptions": {
    "target": "ES2023",
    "${option}": false,
    /* block comment */
    "strict": true
  }
}`,
        ),
      ).toMatchInlineSnapshot(`
        "{
          // This is a comment
          "compilerOptions": {
            "target": "ES2023",
            /* block comment */
            "strict": true
          }
        }"
      `);
    });

    it('handles option: false as last property', () => {
      const filePath = path.join(tmpDir, 'tsconfig.json');
      expect(
        writeAndRemove(
          filePath,
          `{
  "compilerOptions": {
    "target": "ES2023",
    "${option}": false
  }
}`,
        ),
      ).toMatchInlineSnapshot(`
        "{
          "compilerOptions": {
            "target": "ES2023"
          }
        }"
      `);
    });

    it('handles inline block comment next to option: false', () => {
      const filePath = path.join(tmpDir, 'tsconfig.json');
      expect(
        writeAndRemove(
          filePath,
          `{
  "compilerOptions": {
    "target": "ES2023",
    "${option}": false /* reason */,
    "strict": true
  }
}`,
        ),
      ).toMatchInlineSnapshot(`
        "{
          "compilerOptions": {
            "target": "ES2023" /* reason */,
            "strict": true
          }
        }"
      `);
    });

    it('handles compact single-line JSON', () => {
      const filePath = path.join(tmpDir, 'tsconfig.json');
      expect(
        writeAndRemove(filePath, `{"compilerOptions":{"${option}": false, "strict": true}}`),
      ).toMatchInlineSnapshot(`"{"compilerOptions":{"strict": true}}"`);
    });

    it('handles compact single-line JSONC with spaces', () => {
      const filePath = path.join(tmpDir, 'tsconfig.json');
      expect(
        writeAndRemove(filePath, `{ "compilerOptions": { "${option}": false, "strict": true } }`),
      ).toMatchInlineSnapshot(`"{ "compilerOptions": {"strict": true } }"`);
    });

    it('leaves option: true untouched', () => {
      const filePath = path.join(tmpDir, 'tsconfig.json');
      const original = JSON.stringify({ compilerOptions: { [option]: true } }, null, 2);
      fs.writeFileSync(filePath, original);

      const result = removeDeprecatedTsconfigFalseOption(filePath, option);
      expect(result).toBe(false);
      expect(fs.readFileSync(filePath, 'utf-8')).toBe(original);
    });

    it('returns false for non-existent file', () => {
      expect(removeDeprecatedTsconfigFalseOption('/non-existent-file.json', option)).toBe(false);
    });

    it('returns false when no compilerOptions', () => {
      const filePath = path.join(tmpDir, 'tsconfig.json');
      fs.writeFileSync(filePath, '{}');

      expect(removeDeprecatedTsconfigFalseOption(filePath, option)).toBe(false);
    });

    it('returns false when option is not present', () => {
      const filePath = path.join(tmpDir, 'tsconfig.json');
      fs.writeFileSync(filePath, JSON.stringify({ compilerOptions: { strict: true } }, null, 2));

      expect(removeDeprecatedTsconfigFalseOption(filePath, option)).toBe(false);
    });
  },
);

describe('rewriteTypesInTsconfig', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'tsconfig-test-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('rewrites tsdown/client to vite-plus/pack/client', () => {
    const filePath = path.join(tmpDir, 'tsconfig.json');
    fs.writeFileSync(
      filePath,
      `{
  "compilerOptions": {
    "types": ["tsdown/client"]
  }
}`,
    );

    expect(rewriteTypesInTsconfig(filePath)).toBe(true);
    expect(fs.readFileSync(filePath, 'utf-8')).toMatchInlineSnapshot(`
      "{
        "compilerOptions": {
          "types": ["vite-plus/pack/client"]
        }
      }"
    `);
  });

  it('rewrites vite/client to vite-plus/client', () => {
    const filePath = path.join(tmpDir, 'tsconfig.json');
    fs.writeFileSync(
      filePath,
      `{
  "compilerOptions": {
    "types": ["vite/client"]
  }
}`,
    );

    expect(rewriteTypesInTsconfig(filePath)).toBe(true);
    expect(fs.readFileSync(filePath, 'utf-8')).toMatchInlineSnapshot(`
      "{
        "compilerOptions": {
          "types": ["vite-plus/client"]
        }
      }"
    `);
  });

  it('rewrites both in the same array', () => {
    const filePath = path.join(tmpDir, 'tsconfig.json');
    fs.writeFileSync(
      filePath,
      `{
  "compilerOptions": {
    "types": ["tsdown/client", "vite/client"]
  }
}`,
    );

    expect(rewriteTypesInTsconfig(filePath)).toBe(true);
    expect(fs.readFileSync(filePath, 'utf-8')).toMatchInlineSnapshot(`
      "{
        "compilerOptions": {
          "types": ["vite-plus/pack/client", "vite-plus/client"]
        }
      }"
    `);
  });

  it('returns false when no target types exist', () => {
    const filePath = path.join(tmpDir, 'tsconfig.json');
    fs.writeFileSync(
      filePath,
      `{
  "compilerOptions": {
    "types": ["some/other/type"]
  }
}`,
    );

    expect(rewriteTypesInTsconfig(filePath)).toBe(false);
  });

  it('returns false for non-existent file', () => {
    expect(rewriteTypesInTsconfig('/non-existent-file.json')).toBe(false);
  });
});

describe('removeDeprecatedTsconfigFalseOption — combined removal', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'tsconfig-test-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('removes both esModuleInterop and allowSyntheticDefaultImports when both are false', () => {
    const filePath = path.join(tmpDir, 'tsconfig.json');
    fs.writeFileSync(
      filePath,
      `{
  "compilerOptions": {
    "target": "ES2023",
    "esModuleInterop": false,
    "allowSyntheticDefaultImports": false,
    "strict": true
  }
}`,
    );

    expect(removeDeprecatedTsconfigFalseOption(filePath, 'esModuleInterop')).toBe(true);
    expect(removeDeprecatedTsconfigFalseOption(filePath, 'allowSyntheticDefaultImports')).toBe(
      true,
    );
    expect(fs.readFileSync(filePath, 'utf-8')).toMatchInlineSnapshot(`
      "{
        "compilerOptions": {
          "target": "ES2023",
          "strict": true
        }
      }"
    `);
  });
});

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { findTsconfigFiles, removeEsModuleInteropFalseFromFile } from '../tsconfig.js';

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

describe('removeEsModuleInteropFalseFromFile', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'tsconfig-test-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  function writeAndRemove(filePath: string, content: string): string {
    fs.writeFileSync(filePath, content);
    const result = removeEsModuleInteropFalseFromFile(filePath);
    expect(result).toBe(true);
    return fs.readFileSync(filePath, 'utf-8');
  }

  it('removes esModuleInterop: false (middle property)', () => {
    const filePath = path.join(tmpDir, 'tsconfig.json');
    expect(
      writeAndRemove(
        filePath,
        `{
  "compilerOptions": {
    "target": "ES2023",
    "esModuleInterop": false,
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
    "esModuleInterop": false,
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

  it('handles esModuleInterop: false as last property', () => {
    const filePath = path.join(tmpDir, 'tsconfig.json');
    expect(
      writeAndRemove(
        filePath,
        `{
  "compilerOptions": {
    "target": "ES2023",
    "esModuleInterop": false
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

  it('handles inline block comment next to esModuleInterop: false', () => {
    const filePath = path.join(tmpDir, 'tsconfig.json');
    expect(
      writeAndRemove(
        filePath,
        `{
  "compilerOptions": {
    "target": "ES2023",
    "esModuleInterop": false /* reason */,
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
      writeAndRemove(filePath, '{"compilerOptions":{"esModuleInterop": false, "strict": true}}'),
    ).toMatchInlineSnapshot(`"{"compilerOptions":{"strict": true}}"`);
  });

  it('handles compact single-line JSONC with spaces', () => {
    const filePath = path.join(tmpDir, 'tsconfig.json');
    expect(
      writeAndRemove(
        filePath,
        '{ "compilerOptions": { "esModuleInterop": false, "strict": true } }',
      ),
    ).toMatchInlineSnapshot(`"{ "compilerOptions": {"strict": true } }"`);
  });

  it('leaves esModuleInterop: true untouched', () => {
    const filePath = path.join(tmpDir, 'tsconfig.json');
    const original = JSON.stringify({ compilerOptions: { esModuleInterop: true } }, null, 2);
    fs.writeFileSync(filePath, original);

    const result = removeEsModuleInteropFalseFromFile(filePath);
    expect(result).toBe(false);
    expect(fs.readFileSync(filePath, 'utf-8')).toBe(original);
  });

  it('returns false for non-existent file', () => {
    expect(removeEsModuleInteropFalseFromFile('/non-existent-file.json')).toBe(false);
  });

  it('returns false when no compilerOptions', () => {
    const filePath = path.join(tmpDir, 'tsconfig.json');
    fs.writeFileSync(filePath, '{}');

    expect(removeEsModuleInteropFalseFromFile(filePath)).toBe(false);
  });

  it('returns false when esModuleInterop is not present', () => {
    const filePath = path.join(tmpDir, 'tsconfig.json');
    fs.writeFileSync(filePath, JSON.stringify({ compilerOptions: { strict: true } }, null, 2));

    expect(removeEsModuleInteropFalseFromFile(filePath)).toBe(false);
  });
});

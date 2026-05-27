import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import { hasBaseUrlInTsconfig } from '../utils/tsconfig.js';

const tempDirs: string[] = [];

function createTempDir() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-tsconfig-'));
  tempDirs.push(dir);
  return dir;
}

afterEach(() => {
  for (const dir of tempDirs.splice(0, tempDirs.length)) {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

describe('hasBaseUrlInTsconfig', () => {
  it('detects baseUrl in JSONC tsconfig files', () => {
    const projectPath = createTempDir();
    fs.writeFileSync(
      path.join(projectPath, 'tsconfig.json'),
      `{
  "compilerOptions": {
    // Laravel starter tsconfig files commonly keep generated comments.
    "moduleResolution": "bundler",
    "baseUrl": ".",
  }
}
`,
    );

    expect(hasBaseUrlInTsconfig(projectPath)).toBe(true);
  });

  it('returns false when baseUrl is only present in a comment', () => {
    const projectPath = createTempDir();
    fs.writeFileSync(
      path.join(projectPath, 'tsconfig.json'),
      `{
  "compilerOptions": {
    // "baseUrl": ".",
    "moduleResolution": "bundler"
  }
}
`,
    );

    expect(hasBaseUrlInTsconfig(projectPath)).toBe(false);
  });
});

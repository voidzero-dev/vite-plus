import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import {
  findTsconfigFilesWithBaseUrl,
  fixBaseUrlInTsconfig,
  hasBaseUrlInTsconfig,
} from '../utils/tsconfig.js';

const tempDirs: string[] = [];
const originalVpCliBin = process.env.VP_CLI_BIN;

function createTempDir() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-tsconfig-'));
  tempDirs.push(dir);
  return dir;
}

afterEach(() => {
  for (const dir of tempDirs.splice(0, tempDirs.length)) {
    fs.rmSync(dir, { recursive: true, force: true });
  }
  if (originalVpCliBin === undefined) {
    delete process.env.VP_CLI_BIN;
  } else {
    process.env.VP_CLI_BIN = originalVpCliBin;
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

  it('treats null baseUrl as absent', () => {
    const projectPath = createTempDir();
    fs.writeFileSync(
      path.join(projectPath, 'tsconfig.json'),
      JSON.stringify({ compilerOptions: { baseUrl: null } }),
    );

    expect(hasBaseUrlInTsconfig(projectPath)).toBe(false);
    expect(findTsconfigFilesWithBaseUrl(projectPath)).toEqual([]);
  });

  it('detects baseUrl in secondary tsconfig files', () => {
    const projectPath = createTempDir();
    fs.writeFileSync(
      path.join(projectPath, 'tsconfig.json'),
      JSON.stringify({ compilerOptions: { moduleResolution: 'bundler' } }),
    );
    fs.writeFileSync(
      path.join(projectPath, 'tsconfig.app.json'),
      JSON.stringify({ compilerOptions: { baseUrl: '.' } }),
    );

    expect(hasBaseUrlInTsconfig(projectPath)).toBe(true);
  });

  it('returns tsconfig files that contain baseUrl', () => {
    const projectPath = createTempDir();
    fs.writeFileSync(
      path.join(projectPath, 'tsconfig.json'),
      JSON.stringify({ compilerOptions: { moduleResolution: 'bundler' } }),
    );
    fs.writeFileSync(
      path.join(projectPath, 'tsconfig.app.json'),
      JSON.stringify({ compilerOptions: { baseUrl: '.' } }),
    );

    expect(findTsconfigFilesWithBaseUrl(projectPath)).toEqual([
      path.join(projectPath, 'tsconfig.app.json'),
    ]);
  });

  it('fixes every tsconfig file that contains baseUrl', async () => {
    const projectPath = createTempDir();
    const fixerPath = path.join(projectPath, 'fix-baseurl.mjs');
    const invocationsPath = path.join(projectPath, 'fix-invocations.json');
    fs.writeFileSync(
      fixerPath,
      `#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';

const target = process.argv.at(-1);
const invocationsPath = path.join(process.cwd(), 'fix-invocations.json');
const invocations = fs.existsSync(invocationsPath)
  ? JSON.parse(fs.readFileSync(invocationsPath, 'utf8'))
  : [];
invocations.push(target);
fs.writeFileSync(invocationsPath, JSON.stringify(invocations));

const tsconfigPath = path.resolve(process.cwd(), target);
const text = fs.readFileSync(tsconfigPath, 'utf8');
fs.writeFileSync(tsconfigPath, text.replace(/\\n\\s*"baseUrl": "\\.",?/, ''));
`,
    );
    fs.chmodSync(fixerPath, 0o755);
    process.env.VP_CLI_BIN = fixerPath;
    fs.writeFileSync(
      path.join(projectPath, 'tsconfig.json'),
      `{
  "compilerOptions": {
    "moduleResolution": "bundler"
  }
}
`,
    );
    fs.writeFileSync(
      path.join(projectPath, 'tsconfig.app.json'),
      `{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": { "@/*": ["./src/*"] }
  }
}
`,
    );
    fs.writeFileSync(
      path.join(projectPath, 'tsconfig.node.json'),
      `{
  "compilerOptions": {
    "baseUrl": ".",
    "types": ["node"]
  }
}
`,
    );

    await expect(fixBaseUrlInTsconfig(projectPath)).resolves.toBe('fixed');

    const invocations = JSON.parse(fs.readFileSync(invocationsPath, 'utf8')) as string[];
    expect(new Set(invocations)).toEqual(new Set(['tsconfig.app.json', 'tsconfig.node.json']));
    expect(invocations).toHaveLength(2);
    expect(hasBaseUrlInTsconfig(projectPath)).toBe(false);
  });
});

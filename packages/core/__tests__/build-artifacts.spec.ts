import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import url from 'node:url';

import { init, parse } from 'es-module-lexer';
import { describe, expect, it } from 'vite-plus/test';

const coreDir = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), '..');
const distDir = path.join(coreDir, 'dist');

describe('build artifacts', () => {
  it('should include esm-shims.js in dist for tsdown shims support', () => {
    const shimsPath = path.join(distDir, 'esm-shims.js');
    expect(fs.existsSync(shimsPath), `${shimsPath} should exist`).toBe(true);

    const content = fs.readFileSync(shimsPath, 'utf8');
    expect(content).toContain('__dirname');
    expect(content).toContain('__filename');
  });

  it('should include tsdown client.d.ts in dist/tsdown for pack/client support', () => {
    const clientPath = path.join(distDir, 'tsdown/client.d.ts');
    expect(fs.existsSync(clientPath), `${clientPath} should exist`).toBe(true);

    const content = fs.readFileSync(clientPath, 'utf8');
    expect(content).toContain('ImportMeta');
    expect(content).toContain('glob');
  });

  it('should include the Vite bin script pointing at the bundled CLI', () => {
    const binPath = path.join(distDir, 'vite/bin/vite.js');
    expect(fs.existsSync(binPath), `${binPath} should exist`).toBe(true);

    const content = fs.readFileSync(binPath, 'utf8');
    expect(content).toContain(`import('../node/cli.js')`);
    expect(fs.existsSync(path.resolve(path.dirname(binPath), '../node/cli.js'))).toBe(true);
  });

  it('bundles JavaScript Yuku packages and keeps the native parser external', async () => {
    await init;
    const tsdownDir = path.join(distDir, 'tsdown');
    const imports = fs.readdirSync(tsdownDir).flatMap((file) => {
      if (!file.endsWith('.js')) {
        return [];
      }
      const [imports] = parse(fs.readFileSync(path.join(tsdownDir, file), 'utf8'));
      return imports.map((entry) => entry.n);
    });
    expect(imports).toContain('yuku-parser');
    expect(imports).not.toContain('yuku-ast');
    expect(imports).not.toContain('yuku-codegen');
  });

  it('pins the external parser to the Yuku versions used by the bundled dts plugin', () => {
    const coreRequire = createRequire(path.join(coreDir, 'package.json'));
    const pkg = coreRequire('./package.json');
    const tsdownRequire = createRequire(coreRequire.resolve('tsdown/package.json'));
    const pluginRequire = createRequire(tsdownRequire.resolve('rolldown-plugin-dts/package.json'));
    const parserVersion = pluginRequire('yuku-parser/package.json').version;

    expect(pkg.dependencies['yuku-parser']).toBe(parserVersion);
    expect(coreRequire('yuku-parser/package.json').version).toBe(parserVersion);
    for (const name of ['yuku-ast', 'yuku-codegen']) {
      expect(pluginRequire(`${name}/package.json`).version, name).toBe(parserVersion);
    }
  });

  // The bundled dts plugin resolves `vue-tsc`, `@volar/typescript`,
  // `@vue/language-core` and `@typescript/native-preview` from core at runtime,
  // so isolated installs such as pnpm's global virtual store need core to
  // declare them.
  it('should declare the peers of the bundled rolldown-plugin-dts as optional', () => {
    const pkg = JSON.parse(fs.readFileSync(path.join(coreDir, 'package.json'), 'utf8'));
    // Read the copy that tsdown depends on, which is the one bundled into dist/tsdown.
    const tsdownPkgPath = fs.realpathSync(path.join(coreDir, 'node_modules/tsdown/package.json'));
    const dtsPkg = createRequire(tsdownPkgPath)('rolldown-plugin-dts/package.json');

    // `rolldown` is bundled into core, so it must not be a peer.
    expect(pkg.peerDependencies).not.toHaveProperty('rolldown');
    const peers = Object.keys(dtsPkg.peerDependencies).filter((name) => name !== 'rolldown');
    expect(peers).toContain('vue-tsc');
    for (const name of peers) {
      expect(pkg.peerDependencies, name).toHaveProperty([name]);
      expect(pkg.peerDependenciesMeta[name], name).toEqual({ optional: true });
    }
  });
});

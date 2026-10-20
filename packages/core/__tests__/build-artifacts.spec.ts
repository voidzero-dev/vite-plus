import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import url from 'node:url';

import { Lang, parse as parseAst, type SgNode } from '@ast-grep/napi';
import { init, parse } from 'es-module-lexer';
import { glob } from 'tinyglobby';
import { describe, expect, it } from 'vite-plus/test';

const coreDir = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), '..');
const distDir = path.join(coreDir, 'dist');

const referenceContexts = new Set<ReturnType<SgNode['kind']>>([
  'import_statement',
  'export_statement',
  'call_expression',
  'new_expression',
  'variable_declarator',
  'pair',
  'expression_statement',
]);

function referenceContext(node: SgNode): string {
  let parent = node.parent();
  while (parent && !referenceContexts.has(node.kind())) {
    node = parent;
    parent = node.parent();
  }
  return node.text().replace(/\s+/g, ' ');
}

describe('build artifacts', () => {
  it('snapshots remaining Rolldown package names and their uses in all built code', async () => {
    const files = (await glob('**/*.{js,mjs,cjs,ts,mts,cts}', { cwd: distDir })).toSorted();
    expect(files.length).toBeGreaterThan(0);
    const references: { file: string; code: string }[] = [];

    for (const file of files) {
      const source = fs.readFileSync(path.join(distDir, file), 'utf8');
      const lang = /\.[cm]?ts$/.test(file) ? Lang.TypeScript : Lang.JavaScript;
      const root = parseAst(lang, source).root();
      const record = (node: SgNode) => {
        references.push({
          file: file.replace(/-[\w-]{8}(?=(?:\.d)?\.[cm]?[jt]s$)/, '-[hash]'),
          code: referenceContext(node),
        });
      };

      // Inspect literals, not only imports: require.resolve, import.meta.resolve,
      // templates, and new resolution APIs must not bypass this check. Comments
      // are excluded by the parser. Native @rolldown/binding-* packages have a
      // separate release rewrite and coverage in rewrite-rolldown-binding.spec.ts.
      const literals = root.findAll({
        rule: { any: [{ kind: 'string' }, { kind: 'template_string' }] },
      });
      for (const literal of literals) {
        if (
          !/^(?:rolldown|@rolldown\/pluginutils)(?:\/|$|\$\{)/.test(literal.text().slice(1, -1))
        ) {
          continue;
        }
        record(literal);

        // The browser-import matching constant is intentional. Snapshot its uses
        // too, so restoring import.meta.resolve(ROLLDOWN_DEV_RUNTIME_ENTRY) fails
        // even though the declaration itself has not changed. Follow aliases.
        const declaration = literal.parent();
        const name = declaration?.kind() === 'variable_declarator' && declaration.field('name');
        if (!name || name.kind() !== 'identifier') {
          continue;
        }
        const bindings = new Set([name.text()]);
        const identifiers = root.findAll({ rule: { kind: 'identifier' } });
        for (const binding of bindings) {
          for (const identifier of identifiers) {
            if (identifier.text() !== binding) {
              continue;
            }
            const parent = identifier.parent();
            if (
              parent?.kind() === 'variable_declarator' &&
              parent.field('name')?.text() === binding
            ) {
              continue;
            }
            record(identifier);
            if (parent?.kind() === 'variable_declarator') {
              const alias = parent.field('name');
              if (alias?.kind() === 'identifier') {
                bindings.add(alias.text());
              }
            }
          }
        }
      }
    }

    // Only CLI branding, embedded package metadata, and the browser import that
    // Vite rewrites before serving may remain. New entries require review;
    // do not accept a snapshot containing an unresolved Node package lookup.
    expect(references).toMatchSnapshot();
  });

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
    await init();
    const tsdownDir = path.join(distDir, 'tsdown');
    const imports = fs.readdirSync(tsdownDir).flatMap((file) => {
      if (!file.endsWith('.js')) {
        return [];
      }
      const [imports] = parse(fs.readFileSync(path.join(tsdownDir, file), 'utf8'));
      return imports.map((entry) => entry.specifier);
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

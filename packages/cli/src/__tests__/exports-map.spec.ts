import fs from 'node:fs';
import { createRequire } from 'node:module';
import os from 'node:os';
import path from 'node:path';
import url from 'node:url';

import { isStringLiteral } from 'typescript/unstable/ast';
import type { Project } from 'typescript/unstable/sync';
import { API } from 'typescript/unstable/sync';
import { assert } from 'vitest';
import { describe, expect, it } from 'vitest';

/**
 * Regression tests for the generated package.json `exports` map.
 *
 * Node.js package-exports conditions are order-sensitive: when resolving
 * `require('vite-plus/test/config')`, Node walks the condition object and
 * picks the first matching key. `default` matches everything, so a wrongly
 * ordered map like `{ types, default, require }` causes CJS consumers to
 * load the ESM shim — the `.cjs` shim becomes unreachable.
 *
 * These tests pin the invariant that any dual-condition entry emits
 * `require` BEFORE `default` and that runtime resolution returns the
 * expected file extension for each consumer.
 */
import { isJsonObject } from '../utils/json-value.ts';
import { readJsonFile } from '../utils/json.ts';

const cliPkgDir = path.resolve(import.meta.dirname, '../..');
const cliPkgJsonPath = path.join(cliPkgDir, 'package.json');
const requireFromHere = createRequire(import.meta.url);

function readExports() {
  const { exports } = readJsonFile(cliPkgJsonPath);
  assert(isJsonObject(exports), 'Expected package.json exports');
  return exports;
}

// `default` is a module-shape artifact, not a named export migration cares
// about; only the named bindings need to survive the `vitest/config` rewrite.
function namedValueExports(keys: Array<string>): Array<string> {
  return keys.filter((key) => key !== 'default');
}

function missingExports(names: Array<string>, entries: Array<[string, unknown]>) {
  const available = new Map(entries);
  return names.filter((name) => available.get(name) === undefined);
}

function withTypeProject<T>(
  files: Array<string>,
  check: (project: Project) => T,
  sources: Record<string, string> = {},
): T {
  const config = path.join(cliPkgDir, '__test_exports__.json').replaceAll('\\', '/');
  const virtualFiles = {
    ...sources,
    [config]: JSON.stringify({
      files,
      compilerOptions: {
        noEmit: true,
        strict: true,
        skipLibCheck: true,
        types: [],
        module: 'nodenext',
        target: 'esnext',
      },
    }),
  };
  const api = new API({
    fs: {
      readFile: (file) => virtualFiles[file.replaceAll('\\', '/')],
      fileExists: (file) => (file.replaceAll('\\', '/') in virtualFiles ? true : undefined),
    },
  });
  try {
    const project = api.updateSnapshot({ openProjects: [config] }).getProject(config);
    expect(project).toBeDefined();
    return check(project!);
  } finally {
    api.close();
  }
}

function projectDiagnostics({ program }: Project): Array<string> {
  return [
    ...program.getConfigFileParsingDiagnostics(),
    ...program.getProgramDiagnostics(),
    ...program.getGlobalDiagnostics(),
    ...program.getSyntacticDiagnostics(),
    ...program.getSemanticDiagnostics(),
  ].map((diagnostic) => diagnostic.text);
}

function typeDiagnostics(source: string, separator: string = path.sep): Array<string> {
  const filename = path.join(cliPkgDir, '__test_exports__.mts').replaceAll(/[\\/]/g, separator);
  return withTypeProject([filename], projectDiagnostics, {
    [filename.replaceAll('\\', '/')]: source,
  });
}

describe('package.json exports map', () => {
  // The first full type check in this file takes up to about 3 s on the Alpine musl
  // runner, so a slow runner pushes it past Vitest's 5 s default.
  it('checks Playwright options with only declared dependencies available', () => {
    const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'vite-plus-provider-types-'));
    try {
      const pkg = JSON.parse(fs.readFileSync(cliPkgJsonPath, 'utf8'));
      const installed = path.join(directory, 'node_modules/vite-plus');
      fs.mkdirSync(installed, { recursive: true });
      fs.writeFileSync(path.join(installed, 'package.json'), JSON.stringify(pkg));
      fs.cpSync(path.join(cliPkgDir, 'dist/test'), path.join(installed, 'dist/test'), {
        recursive: true,
      });
      // Keep the copied declarations outside the workspace so hoisted dependencies
      // cannot hide missing peers, as in pnpm's global virtual store (#2854).
      for (const name of Object.keys({ ...pkg.dependencies, ...pkg.peerDependencies })) {
        const target = path.join(directory, 'node_modules', name);
        fs.mkdirSync(path.dirname(target), { recursive: true });
        fs.symlinkSync(
          fs.realpathSync(path.join(cliPkgDir, 'node_modules', name)),
          target,
          'junction',
        );
      }
      const filename = path.join(directory, 'config.mts');
      fs.writeFileSync(
        filename,
        `
import { playwright } from 'vite-plus/test/browser-playwright';
import { playwright as alias } from 'vite-plus/test/browser/providers/playwright';
playwright({ contextOptions: { reducedMotion: 'reduce' } });
alias({ contextOptions: { reducedMotion: 'no-preference' } });
// @ts-expect-error Invalid Playwright options must not become unchecked under skipLibCheck.
playwright({ contextOptions: { reducedMotion: 'bogus' } });
// @ts-expect-error The compatibility alias must retain the same option types.
alias({ contextOptions: { reducedMotion: 'bogus' } });
`,
      );
      expect(withTypeProject([filename], projectDiagnostics)).toEqual([]);
    } finally {
      fs.rmSync(directory, { recursive: true, force: true });
    }
  }, 15_000);

  it('provides the bundled Vitest Vite peer without relying on project dependencies', () => {
    const pkg = JSON.parse(fs.readFileSync(cliPkgJsonPath, 'utf8'));
    expect(pkg.dependencies.vite).toBe('workspace:@voidzero-dev/vite-plus-core@*');
    expect(pkg.devDependencies.vite).toBeUndefined();
  });

  it('keeps the reviewed Vitest v5 export surface', () => {
    const pkg = JSON.parse(fs.readFileSync(cliPkgJsonPath, 'utf8'));
    expect(
      Object.keys(pkg.exports)
        .filter((key) => key === './test' || key.startsWith('./test/'))
        .toSorted(),
    ).toMatchSnapshot();
  });

  it('resolves every runtime test export under Node ESM conditions', () => {
    for (const [key, value] of Object.entries(readExports())) {
      if (
        !(key === './test' || key.startsWith('./test/')) ||
        !isJsonObject(value) ||
        (!value.default && !value.import)
      ) {
        continue;
      }
      const resolved = import.meta.resolve(`vite-plus${key.slice(1)}`);
      expect(fs.existsSync(url.fileURLToPath(resolved)), key).toBe(true);
    }
  });

  it.each(['/', '\\'])('compiles all typed test exports with %s path separators', (separator) => {
    const entries = Object.entries(readExports()).filter(
      ([key, entry]) =>
        (key === './test' || key.startsWith('./test/')) &&
        isJsonObject(entry) &&
        (entry.types || (isJsonObject(entry.import) && entry.import.types)),
    );
    const source =
      entries
        .map(([key], index) => `import type * as E${index} from 'vite-plus${key.slice(1)}';`)
        .join('\n') +
      `
import { expect } from 'vite-plus/test';
declare module 'vite-plus/test' {
  interface Matchers<R extends void | Promise<void> = void | Promise<void>, T = unknown> {
    toMatchReceived(expected: T): R;
  }
}
const synchronous: void = expect(42).toMatchReceived(42);
const asynchronous: Promise<void> = expect(Promise.resolve(42)).resolves.toMatchReceived(42);
// @ts-expect-error The received type is number, not string.
expect(42).toMatchReceived('42');
// @ts-expect-error An asynchronous matcher does not return void.
const invalidReturn: void = expect(Promise.resolve(42)).resolves.toMatchReceived(42);
`;
    expect(typeDiagnostics(source, separator)).toEqual([]);
  });

  it('loads browser matchers without another browser entry masking missing declarations', () => {
    expect(
      typeDiagnostics(`
import 'vite-plus/test/matchers';
import { expect } from 'vite-plus/test';
expect(document.body).toBeInTheDocument();
expect(document.body).toHaveFocus();
expect(document.body).toHaveAttribute('id', 'app');
// @ts-expect-error Attribute names must be strings.
expect(document.body).toHaveAttribute(123);
`),
    ).toEqual([]);
  });

  it.each([
    'browser',
    'context',
    'browser/context',
    'plugins/browser-context',
    'browser-playwright/context',
    'browser-preview/context',
    'browser/providers/playwright/context',
    'browser/providers/preview/context',
  ])('preserves provider augmentations and role types through %s independently', (name) => {
    // Do not import the other aliases here: their augmentations can hide a
    // broken declaration. Provider options and roles must retain their types.
    expect(
      typeDiagnostics(`
import 'vite-plus/test/browser-playwright';
import { page, type UserEventClickOptions } from 'vite-plus/test/${name}';
declare const click: UserEventClickOptions;
const force: boolean | undefined = click.force;
await page.getByRole('button').screenshot({ caret: 'hide' });
// @ts-expect-error Roles accept strings, not numbers.
page.getByRole(123);
// @ts-expect-error Playwright's force option must remain a boolean.
const invalidClick: UserEventClickOptions = { force: 'yes' };
`),
    ).toEqual([]);
  });

  it('resolves every relative import in generated test declarations', () => {
    const directory = path.join(cliPkgDir, 'dist/test');
    const files = fs
      .readdirSync(directory, { recursive: true, encoding: 'utf8' })
      .filter((name) => /\.d\.(?:ts|mts|cts)$/.test(name))
      .map((name) => path.join(directory, name));
    expect(files.length).toBeGreaterThan(0);
    withTypeProject(files, ({ program, checker }) => {
      for (const file of files) {
        const source = program.getSourceFile(file);
        expect(source, file).toBeDefined();
        for (const specifier of source!.imports) {
          if (!isStringLiteral(specifier) || !specifier.text.startsWith('.')) {
            continue;
          }
          expect(
            checker.getSymbolAtLocation(specifier)?.declarations?.length,
            `${path.relative(directory, file)}: ${specifier.text}`,
          ).toBeGreaterThan(0);
        }
      }
    });
  });

  it.each(['coverage', 'reporters', 'environments', 'snapshot'])(
    'does not publish the removed %s alias',
    (name) => {
      const pkg = JSON.parse(fs.readFileSync(cliPkgJsonPath, 'utf8'));
      expect(pkg.exports).not.toHaveProperty(`./test/${name}`);
      expect(() => requireFromHere.resolve(`vite-plus/test/${name}`)).toThrow(
        expect.objectContaining({ code: 'ERR_PACKAGE_PATH_NOT_EXPORTED' }),
      );
      for (const extension of ['js', 'd.ts']) {
        expect(fs.existsSync(path.join(cliPkgDir, 'dist/test', `${name}.${extension}`))).toBe(
          false,
        );
      }
    },
  );

  it('keeps the standalone mocker migration target', () => {
    const source = fs.readFileSync(path.join(cliPkgDir, 'dist/test/mocker.js'), 'utf8');
    expect(source).toBe("export * from '@vitest/mocker';\n");
  });

  it('every dual-condition entry emits `require` before `default`', () => {
    const exports = readExports();

    const offenders: Array<{ path: string; order: Array<string> }> = [];

    function walk(subpath: string, value: unknown) {
      if (!isJsonObject(value)) {
        return;
      }
      const keys = Object.keys(value);
      const requireIdx = keys.indexOf('require');
      const defaultIdx = keys.indexOf('default');
      if (requireIdx !== -1 && defaultIdx !== -1 && requireIdx > defaultIdx) {
        offenders.push({ path: subpath, order: keys });
      }
      for (const [k, v] of Object.entries(value)) {
        walk(`${subpath} > ${k}`, v);
      }
    }

    for (const [subpath, value] of Object.entries(exports)) {
      walk(subpath, value);
    }

    expect(offenders, 'entries with require ordered after default').toEqual([]);
  });

  it.each([
    'browser/context',
    'context',
    'plugins/browser-context',
    'browser-preview/context',
    'browser-playwright/context',
  ])('routes the %s runtime alias to the v5 browser virtual module', (name) => {
    expect(fs.readFileSync(path.join(cliPkgDir, 'dist/test', `${name}.js`), 'utf8')).toBe(
      "export * from 'vitest/browser';\n",
    );
  });

  it('retains the upstream optional WebDriverIO peer without public shims', () => {
    const pkg = JSON.parse(fs.readFileSync(cliPkgJsonPath, 'utf8'));
    expect(pkg.peerDependencies['@vitest/browser-webdriverio']).toBe(
      requireFromHere('vitest/package.json').peerDependencies['@vitest/browser-webdriverio'],
    );
    expect(pkg.peerDependenciesMeta['@vitest/browser-webdriverio']).toEqual({ optional: true });
    expect(Object.keys(pkg.exports).filter((name) => name.includes('webdriverio'))).toEqual([]);
  });

  it('`require.resolve("vite-plus/test/config")` resolves to the .cjs shim', () => {
    const resolved = requireFromHere.resolve('vite-plus/test/config');
    expect(resolved.endsWith('.cjs'), `resolved to ${resolved}`).toBe(true);
  });

  it('ESM `import.meta.resolve("vite-plus/test/config")` resolves to the .js shim', () => {
    // import.meta.resolve is sync in modern Node (>= 20.6) and respects the
    // `default` (ESM) condition for ESM consumers.
    const resolved = import.meta.resolve('vite-plus/test/config');
    expect(resolved.endsWith('.js'), `resolved to ${resolved}`).toBe(true);
  });

  it('CJS shim at ./test/config delegates to vitest/config via require()', () => {
    expect(requireFromHere('vite-plus/test/config')).toBe(requireFromHere('vitest/config'));
  });

  it('browser-playwright declaration uses NodeNext-compatible relative specifiers', () => {
    // Matches quoted relative module specifiers such as './node' and '../browser.js'.
    const relativeModuleSpecifier = /(['"])(\.\.?\/[^'"]+)\1/g;
    const entry = readExports()['./test/browser-playwright'];
    assert(isJsonObject(entry));
    const { types } = entry;
    assert(typeof types === 'string');
    const source = fs.readFileSync(path.resolve(cliPkgDir, types), 'utf8');
    const relativeSpecifiers = Array.from(
      source.matchAll(relativeModuleSpecifier),
      (match) => match[2],
    );
    expect(
      relativeSpecifiers.length,
      'declaration should contain relative specifiers',
    ).toBeGreaterThan(0);
    expect(
      relativeSpecifiers.filter((specifier) => !specifier.endsWith('.js')),
      'declaration contains NodeNext-incompatible relative specifiers',
    ).toEqual([]);
  });
});

// Migrated plugins depend on these entry points resolving the upstream APIs.
describe('Oxlint JS-plugin authoring entrypoints', () => {
  it('re-exports the full @oxlint/plugins value surface', async () => {
    const [lintPlugins, oxlintPlugins] = await Promise.all([
      import('vite-plus/lint/plugins'),
      import('@oxlint/plugins'),
    ]);
    const expected = namedValueExports(Object.keys(oxlintPlugins));
    expect(expected.length, 'sanity: @oxlint/plugins should expose value exports').toBeGreaterThan(
      0,
    );
    const missing = missingExports(expected, Object.entries(lintPlugins));
    expect(missing, '@oxlint/plugins value exports missing from vite-plus/lint/plugins').toEqual(
      [],
    );
  });

  it('serves the authoring API to CommonJS too', () => {
    // A `.cts` plugin, or a `.ts` one compiled with `module: commonjs`, emits
    // its import as `require()`. `@oxlint/plugins` ships CJS, so the shim does
    // too. `plugins-dev` instead exposes its ESM entry to both module systems.
    const plugins = requireFromHere('vite-plus/lint/plugins');
    expect(plugins.defineRule).toBeTypeOf('function');
    expect(plugins.definePlugin).toBeTypeOf('function');
  });

  it('exposes RuleTester from vite-plus/lint/plugins-dev', async () => {
    const ruleTester = await import('vite-plus/lint/plugins-dev');
    expect(ruleTester.RuleTester).toBeTypeOf('function');
  });

  it('serves the same RuleTester to CommonJS', async () => {
    // Static imports in .cts files compile to require() after migration.
    const ruleTester = requireFromHere('vite-plus/lint/plugins-dev');
    const upstream = requireFromHere('oxlint/plugins-dev');
    const esm = await import('vite-plus/lint/plugins-dev');
    expect(ruleTester.RuleTester).toBeTypeOf('function');
    expect(ruleTester.RuleTester).toBe(upstream.RuleTester);
    expect(ruleTester.RuleTester).toBe(esm.RuleTester);
  });
});

/**
 * Migration rewrites the `vitest/config` specifier to bare `vite-plus` (see the
 * Rust `import_rewriter.rs` rule and the `prefer-vite-plus-imports` oxlint rule
 * in `oxlint-plugin.ts`). After that rewrite a user's
 * `import { x } from 'vitest/config'` (and the `require(...)` form) becomes
 * `from 'vite-plus'`, so the `vite-plus` root MUST stay a superset of
 * `vitest/config`'s runtime exports. The named re-export lists in `index.ts`
 * (ESM) and `index.cts` (CJS) are SEPARATE manual lists, and both deliberately
 * omit `defineConfig` (local `./define-config.ts` wrapper) and `mergeConfig`
 * (`@voidzero-dev/vite-plus-core`), which are supplied by other paths. These
 * guards assert the *aggregate* surface stays complete on BOTH module systems so
 * a future vitest bump that adds a config export can't silently leave migrated
 * imports `undefined` — and can't be fixed in one entry while the other regresses.
 *
 * Note: these cover the runtime (value) surface only. `vitest/config`'s
 * type-only exports flow through a separate `export type { … }` list in
 * `index.ts`; removal-drift there is already caught by the repo typecheck (a
 * re-export of a deleted type fails to compile).
 */
describe('vite-plus root re-exports the full vitest/config surface', () => {
  it('exposes every vitest/config value export on the ESM vite-plus root', async () => {
    const [vitePlus, vitestConfig] = await Promise.all([
      import('vite-plus'),
      import('vitest/config'),
    ]);
    const expected = namedValueExports(Object.keys(vitestConfig));
    expect(expected.length, 'sanity: vitest/config should expose value exports').toBeGreaterThan(0);
    const missing = missingExports(expected, Object.entries(vitePlus));
    expect(missing, 'vitest/config value exports missing from the ESM vite-plus root').toEqual([]);
  });

  it('exposes every vitest/config value export on the CJS vite-plus root', () => {
    // `require('vitest/config')` -> `require('vite-plus')` resolves the package
    // root's `require` condition (the index.cts build), a separate manual list.
    const vitePlus = requireFromHere('vite-plus');
    const vitestConfig = requireFromHere('vitest/config');
    const expected = namedValueExports(Object.keys(vitestConfig));
    expect(expected.length, 'sanity: vitest/config should expose value exports').toBeGreaterThan(0);
    const missing = missingExports(expected, Object.entries(vitePlus));
    expect(missing, 'vitest/config value exports missing from the CJS vite-plus root').toEqual([]);
  });
});

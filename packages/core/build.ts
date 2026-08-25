import { existsSync } from 'node:fs';
import { copyFile, cp, mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { format } from 'oxfmt';

// Convert native path to POSIX format for glob patterns
function toPosixPath(nativePath: string): string {
  return nativePath.split(path.sep).join(path.posix.sep);
}

import { build, type BuildOptions } from 'rolldown';
import { dts } from 'rolldown-plugin-dts';
import { glob } from 'tinyglobby';

import { generateLicenseFile } from '../../scripts/generate-license.js';
import viteRolldownConfig from '../../vite/packages/vite/rolldown.config.js';
import cliPkgJson from '../cli/package.json' with { type: 'json' };
import { buildCjsDeps } from './build-support/build-cjs-deps.js';
import { replaceThirdPartyCjsRequires } from './build-support/find-create-require.js';
import { RewriteImportsPlugin } from './build-support/rewrite-imports.js';
import {
  createRolldownRewriteRules,
  createViteRewriteRules,
  rewriteModuleSpecifiers,
  type ReplacementRule,
} from './build-support/rewrite-module-specifiers.js';
import { rewriteRolldownBindingRequires } from './build-support/rewrite-rolldown-binding.js';
import pkgJson from './package.json' with { type: 'json' };

const projectDir = join(fileURLToPath(import.meta.url), '..');

const rolldownPluginUtilsDir = resolve(
  projectDir,
  '..',
  '..',
  'rolldown',
  'packages',
  'rolldown',
  'node_modules',
  '@rolldown',
  'pluginutils',
);

const rolldownSourceDir = resolve(projectDir, '..', '..', 'rolldown', 'packages', 'rolldown');

const rolldownViteSourceDir = resolve(projectDir, '..', '..', 'vite', 'packages', 'vite');

const tsdownSourceDir = resolve(projectDir, 'node_modules/tsdown');

// Main build orchestration
await bundleRolldownPluginutils();
await bundleRolldown();
await buildVite();
await bundleTsdown();
await brandTsdown();
await wireBundledTsdownExtensions();
generateLicenseFile({
  title: 'Vite-Plus core license',
  packageName: 'Vite-Plus',
  outputPath: join(projectDir, 'LICENSE'),
  coreLicensePath: join(projectDir, '..', '..', 'LICENSE'),
  bundledPaths: [join(projectDir, 'dist')],
  resolveFrom: [
    projectDir,
    join(projectDir, '..', '..'),
    join(projectDir, '..', '..', 'rolldown'),
    join(projectDir, '..', '..', 'vite'),
  ],
  extraPackages: [
    {
      packageDir: rolldownSourceDir,
      licensePath: join(projectDir, '..', '..', 'rolldown', 'LICENSE'),
    },
    {
      packageDir: rolldownPluginUtilsDir,
      licensePath: join(rolldownPluginUtilsDir, 'LICENSE'),
    },
    {
      packageDir: rolldownViteSourceDir,
    },
    {
      packageDir: tsdownSourceDir,
    },
  ],
});
if (!existsSync(join(projectDir, 'LICENSE'))) {
  throw new Error('LICENSE was not generated during build');
}
await mergePackageJson();

async function buildVite() {
  const newViteRolldownConfig = viteRolldownConfig.map((config) => {
    config.tsconfig = join(projectDir, 'tsconfig.json');
    config.cwd = projectDir;

    if (Array.isArray(config.external)) {
      config.external = config.external.filter((external) => {
        return !(
          (typeof external === 'string' &&
            (external === 'picomatch' ||
              external === 'tinyglobby' ||
              external === 'fdir' ||
              external === 'rolldown')) ||
          external === 'yaml' ||
          (external instanceof RegExp && (external.test('rolldown/') || external.test('vite/')))
        );
      });
    }

    if (typeof config.output === 'object' && !Array.isArray(config.output)) {
      config.output.dir = './dist/vite';
    }

    if (config.platform === 'node') {
      if (config.resolve) {
        if (Array.isArray(config.resolve?.conditionNames)) {
          config.resolve?.conditionNames?.unshift('dev');
        } else {
          config.resolve.conditionNames = ['dev'];
        }
      } else {
        config.resolve = {
          conditionNames: ['dev'],
        };
      }
    }

    if (Array.isArray(config.plugins)) {
      config.plugins = [
        // Add RewriteImportsPlugin to handle vite/rolldown import rewrites
        RewriteImportsPlugin,
        {
          name: 'fix-module-runner-dynamic-request-url',
          transform(_, id, meta) {
            if (id.endsWith(join('vite', 'src', 'module-runner', 'runner.ts'))) {
              const { magicString } = meta;
              if (magicString) {
                // Fix dynamicRequest to use the server-normalized module URL
                // (mod.url) instead of the raw URL parameter for relative path
                // resolution. The raw `url` can be a file:// URL (e.g. from
                // VitePress's `import(pathToFileURL(entryPath).href)`) which
                // pathe.resolve cannot handle as an absolute path, producing
                // malformed paths like "<cwd>/file:<path>".
                // mod.url is always a server-normalized URL (e.g. /@fs/...)
                // that posixResolve handles correctly.
                magicString.replace(
                  `if (dep[0] === '.') {\n        dep = posixResolve(posixDirname(url), dep)\n      }`,
                  `if (dep[0] === '.') {\n        dep = posixResolve(posixDirname(mod.url), dep)\n      }`,
                );
                return {
                  code: magicString,
                };
              }
            }
            return undefined;
          },
        },
        {
          name: 'rewrite-static-paths',
          transform(_, id, meta) {
            if (id.endsWith(join('vite', 'src', 'node', 'constants.ts'))) {
              const { magicString } = meta;
              if (magicString) {
                magicString.replace(
                  `export const VITE_PACKAGE_DIR: string = resolve(
  fileURLToPath(import.meta.url),
  '../../..',
)`,
                  // From 'node_modules/@voidzero-dev/vite-plus-core/dist/vite/node/chunks/const.js' to  'node_modules/@voidzero-dev/vite-plus-core'
                  `export const VITE_PACKAGE_DIR: string = path.join(fileURLToPath(/** #__KEEP__ */import.meta.url), '..', '..', '..', '..', '..')`,
                );
                magicString.replace(
                  `export const CLIENT_ENTRY: string = resolve(
  VITE_PACKAGE_DIR,
  'dist/client/client.mjs',
)`,
                  `export const CLIENT_ENTRY = path.join(VITE_PACKAGE_DIR, 'dist/vite/client/client.mjs')`,
                );
                magicString.replace(
                  `export const BUNDLED_DEV_CLIENT_ENTRY: string = resolve(
  VITE_PACKAGE_DIR,
  'dist/client/bundledDevClient.mjs',
)`,
                  `export const BUNDLED_DEV_CLIENT_ENTRY = path.join(VITE_PACKAGE_DIR, 'dist/vite/client/bundledDevClient.mjs')`,
                );
                magicString.replace(
                  `export const ENV_ENTRY: string = resolve(
  VITE_PACKAGE_DIR,
  'dist/client/env.mjs',
)`,
                  `export const ENV_ENTRY = path.join(VITE_PACKAGE_DIR, 'dist/vite/client/env.mjs')`,
                );
                magicString.replace(
                  `const { version } = JSON.parse(
  readFileSync(new URL('../../package.json', import.meta.url)).toString(),
)`,
                  `import { version } from '../../package.json' with { type: 'json' }`,
                );
                return {
                  code: magicString,
                };
              }
            }
            return undefined;
          },
        },
        ...config.plugins.filter((plugin) => {
          return !(
            typeof plugin === 'object' &&
            plugin !== null &&
            'name' in plugin &&
            (plugin.name === 'rollup-plugin-license' || plugin.name === 'bundle-limit')
          );
        }),
      ];
    }

    if (config.experimental) {
      config.experimental.nativeMagicString = true;
    } else {
      config.experimental = {
        nativeMagicString: true,
      };
    }

    return config;
  });

  await build(newViteRolldownConfig as BuildOptions[]);

  // Copy additional vite files

  await cp(join(rolldownViteSourceDir, 'misc'), join(projectDir, 'dist/vite/misc'), {
    recursive: true,
  });

  // Copy and rewrite .d.ts files
  // Normalize glob pattern to use forward slashes on Windows
  const dtsFiles = await glob(
    toPosixPath(join(rolldownViteSourceDir, 'dist', 'node', '**/*.d.ts')),
    { absolute: true },
  );

  for (const dtsFile of dtsFiles) {
    const file = await readFile(dtsFile, 'utf-8');
    // Normalize paths to use forward slashes for consistent replacement on Windows
    const relativePath = toPosixPath(dtsFile).replace(
      toPosixPath(join(rolldownViteSourceDir, 'dist', 'node')),
      '',
    );
    const dstFilePath = join(projectDir, 'dist', 'vite', 'node', relativePath);
    const rewrittenFile = rewriteModuleSpecifiers(file, dtsFile, {
      rules: [...createViteRewriteRules(pkgJson.name), ...createRolldownRewriteRules(pkgJson.name)],
    });
    await writeFile(dstFilePath, rewrittenFile);
  }

  // Copy type files
  // Normalize glob pattern to use forward slashes on Windows
  const srcTypeFiles = await glob(toPosixPath(join(rolldownViteSourceDir, 'types', '**/*.d.ts')), {
    absolute: true,
  });

  await mkdir(join(projectDir, 'dist/vite/types'), { recursive: true });

  for (const srcDtsFile of srcTypeFiles) {
    const file = await readFile(srcDtsFile, 'utf-8');
    // Normalize paths to use forward slashes for consistent replacement on Windows
    const relativePath = toPosixPath(srcDtsFile).replace(
      toPosixPath(join(rolldownViteSourceDir, 'types')),
      '',
    );
    const dstFilePath = join(projectDir, 'dist', 'vite', 'types', relativePath);
    const dir = dirname(dstFilePath);
    if (!existsSync(dir)) {
      await mkdir(dir, { recursive: true });
    }
    let rewrittenFile = rewriteModuleSpecifiers(file, srcDtsFile, {
      rules: [...createViteRewriteRules(pkgJson.name), ...createRolldownRewriteRules(pkgJson.name)],
    });
    // Upstream bug (vitejs/vite#21863, commit cc39e5540): `node/index.ts`
    // re-exports `KnownQueryTypeMap` from `#types/importGlob`, but the type is
    // declared there without `export`. Bundling vite's types into a downstream
    // project then fails with `[MISSING_EXPORT]`. Add the missing `export` here
    // until it is fixed upstream (drop this once vite exports it).
    if (relativePath === '/importGlob.d.ts') {
      const knownQueryTypeMapPattern = /^type KnownQueryTypeMap\b/m;
      if (!knownQueryTypeMapPattern.test(rewrittenFile)) {
        throw new Error(
          'Expected vite types/importGlob.d.ts to declare non-exported KnownQueryTypeMap. ' +
            'Upstream may have fixed vitejs/vite#21863; remove the KnownQueryTypeMap export workaround from packages/core/build.ts.',
        );
      }
      rewrittenFile = rewrittenFile.replace(
        knownQueryTypeMapPattern,
        'export type KnownQueryTypeMap',
      );
    }
    await writeFile(dstFilePath, rewrittenFile);
  }

  await cp(
    join(rolldownViteSourceDir, 'client.d.ts'),
    join(projectDir, 'dist', 'vite', 'client.d.ts'),
  );
}

async function bundleRolldownPluginutils() {
  await mkdir(join(projectDir, 'dist', 'pluginutils'), { recursive: true });

  await cp(join(rolldownPluginUtilsDir, 'dist'), join(projectDir, 'dist', 'pluginutils'), {
    recursive: true,
  });
}

async function bundleRolldown() {
  await mkdir(join(projectDir, 'dist/rolldown'), { recursive: true });

  const rolldownFiles = new Set<string>();

  await cp(join(rolldownSourceDir, 'dist'), join(projectDir, 'dist/rolldown'), {
    recursive: true,
    filter: async (from, to) => {
      if ((await stat(from)).isFile()) {
        rolldownFiles.add(to);
      }
      return true;
    },
  });

  // Platform suffixes Vite+ publishes native packages for, e.g. `darwin-arm64`
  // from `aarch64-apple-darwin`. `@rolldown/binding-*` uses the same napi
  // suffix convention, so these are the loader branches release builds
  // redirect to `<napi.packageName>-<suffix>`. `@napi-rs/cli` loads lazily
  // because only release builds need it (it costs ~120ms to import).
  let vitePlusPlatformSuffixes: ReadonlySet<string> | undefined;
  if (process.env.RELEASE_BUILD) {
    const { parseTriple } = await import('@napi-rs/cli');
    vitePlusPlatformSuffixes = new Set(
      cliPkgJson.napi.targets.map((target) => parseTriple(target).platformArchABI),
    );
  }
  const rewrittenSuffixes = new Set<string>();
  let bindingSpecifierRewrites = 0;
  let bindingGuardRewrites = 0;

  // Rewrite @rolldown/pluginutils imports in JS and type declaration files
  for (const file of rolldownFiles) {
    if (
      file.endsWith('.mjs') ||
      file.endsWith('.js') ||
      file.endsWith('.d.mts') ||
      file.endsWith('.d.ts')
    ) {
      let source = await readFile(file, 'utf-8');
      const rules: ReplacementRule[] = [...createRolldownRewriteRules(pkgJson.name)];
      if (vitePlusPlatformSuffixes) {
        const result = rewriteRolldownBindingRequires(source, {
          packageName: cliPkgJson.napi.packageName,
          platformSuffixes: vitePlusPlatformSuffixes,
          version: pkgJson.version,
        });
        source = result.source;
        for (const suffix of result.rewrittenSuffixes) {
          rewrittenSuffixes.add(suffix);
        }
        bindingSpecifierRewrites += result.specifierRewrites;
        bindingGuardRewrites += result.guardRewrites;
      }
      const newSource = rewriteModuleSpecifiers(source, file, { rules });
      await writeFile(file, newSource);
    }
  }

  // Every published platform suffix must find its loader branch, and each
  // redirected branch requires the platform package twice (the binding itself
  // and its package.json version guard) with one guard. A napi-rs upgrade
  // that reshapes the generated loader, or a Rolldown loader that drops a
  // branch, breaks these invariants; fail the release build instead of
  // shipping a partial rewrite.
  if (vitePlusPlatformSuffixes) {
    const missing = [...vitePlusPlatformSuffixes].filter((s) => !rewrittenSuffixes.has(s));
    if (missing.length > 0 || bindingSpecifierRewrites !== bindingGuardRewrites * 2) {
      throw new Error(
        `bundleRolldown: unexpected Rolldown binding loader shape ` +
          `(${bindingSpecifierRewrites} specifier rewrites, ${bindingGuardRewrites} guard rewrites` +
          (missing.length > 0 ? `, missing platform branches: ${missing.join(', ')}` : '') +
          `); update build-support/rewrite-rolldown-binding.ts for the current napi-rs loader format`,
      );
    }
  }
}

async function bundleTsdown() {
  await mkdir(join(projectDir, 'dist/tsdown/dist'), { recursive: true });

  const require = createRequire(import.meta.url);

  // `@tsdown/exe` and `@tsdown/css` are bundled directly into core instead of
  // being externalized. They have a hard peer dependency on `tsdown` and import
  // `tsdown/internal`, but Vite+ bundles tsdown inside core rather than exposing
  // a resolvable top-level `tsdown` package. Bundling them here resolves
  // `tsdown/internal` against the bundled tsdown at build time, so `vp pack
  // --exe` and CSS bundling work without users installing anything extra.
  // See https://github.com/voidzero-dev/vite-plus/issues/1586
  const bundledTsdownPackages = new Set(['@tsdown/exe', '@tsdown/css']);

  // Everything else in tsdown's peer dependencies stays external. `lightningcss`
  // (a native module) and `postcss` also stay external: `@tsdown/css` pulls them
  // in and they cannot be bundled. Both are core `dependencies`, so they resolve
  // at runtime and CSS bundling works without users installing anything.
  //
  // `yuku-codegen` and `yuku-parser` (pulled in by `rolldown-plugin-dts` for dts
  // generation) are napi packages: their loaders `require` a per-platform
  // `@yuku-*/binding-<platform>/*.node` relative to their own `__dirname`.
  // Bundling their JS into a core chunk rebinds `__dirname` to `dist/tsdown/`,
  // so the native binding no longer resolves. Keep them external and list them
  // as core `dependencies` so each package's loader resolves its own binding
  // (declared as optionalDependencies) at runtime.
  const tsdownExternal = [
    ...Object.keys(pkgJson.peerDependencies).filter((name) => !bundledTsdownPackages.has(name)),
    'lightningcss',
    'postcss',
    'yuku-codegen',
    'yuku-parser',
  ];
  const isExternal = (id: string) => tsdownExternal.some((e) => id === e || id.startsWith(`${e}/`));

  const thirdPartyCjsModules = new Set<string>();

  // Re-build tsdown cli plus the bundled `@tsdown/exe` and `@tsdown/css`
  // extensions as stable named entries (`tsdown-exe.js`, `tsdown-css.js`).
  await build({
    input: {
      run: join(tsdownSourceDir, 'dist/run.mjs'),
      index: join(tsdownSourceDir, 'dist/index.mjs'),
      'tsdown-exe': require.resolve('@tsdown/exe'),
      'tsdown-css': require.resolve('@tsdown/css'),
    },
    output: {
      format: 'esm',
      cleanDir: true,
      dir: join(projectDir, 'dist/tsdown'),
      entryFileNames: '[name].js',
    },
    platform: 'node',
    external: isExternal,
    plugins: [
      RewriteImportsPlugin,
      {
        name: 'find-third-party-cjs-requires',
        async transform(code, id) {
          if (id.endsWith('.js') || id.endsWith('.mjs')) {
            const { code: updatedCode, modules: thirdPartyModules } =
              await replaceThirdPartyCjsRequires(code, id, new Set(tsdownExternal));
            for (const module of thirdPartyModules) {
              thirdPartyCjsModules.add(module);
            }
            return { code: updatedCode };
          }
          return undefined;
        },
      },
    ],
  });

  await buildCjsDeps(thirdPartyCjsModules, join(projectDir, 'dist/tsdown'));

  await build({
    input: {
      'index-types': join(tsdownSourceDir, 'dist/index.d.mts'),
    },
    output: {
      format: 'esm',
      dir: join(projectDir, 'dist/tsdown'),
    },
    external: isExternal,
    plugins: [
      RewriteImportsPlugin,
      dts({
        generator: 'oxc',
        dtsInput: true,
      }),
    ],
  });

  // Copy esm-shims.js to dist/ so tsdown's shims option can resolve it.
  // tsdown resolves this file via path.resolve(import.meta.dirname, '..', 'esm-shims.js'),
  // which means it expects the file at dist/esm-shims.js (one level up from dist/tsdown/).
  await copyFile(join(tsdownSourceDir, 'esm-shims.js'), join(projectDir, 'dist/esm-shims.js'));

  // Copy client.d.ts to dist/tsdown/ to expose it as the vite-plus/pack/client entry point,
  // equivalent to tsdown/client for registering bundler type features with TypeScript.
  await copyFile(join(tsdownSourceDir, 'client.d.ts'), join(projectDir, 'dist/tsdown/client.d.ts'));
}

// Ensure a bundled chunk imports the given ansis color helpers (e.g. `bold`,
// `red`) from the shared `main-*.js` chunk. tsdown's logger module does not
// import every color the Vite+ branding uses, so after the logger patches we
// add any missing ones, resolving their (minified) export aliases from main's
// own `export { ... }` map so the fix survives rolldown renaming them.
async function ensureAnsisImports(
  content: string,
  names: string[],
  distDir: string,
): Promise<string> {
  // Scan every relative chunk import in the branded logger chunk. Which shared
  // chunk holds the ansis colors depends on rolldown's chunking and has moved
  // between versions (e.g. `main-*.js` → `ansis-*.js`), so we don't assume a
  // fixed chunk name: instead we append each missing color to whichever imported
  // chunk actually re-exports it.
  const importRe = /import \{([^}]*)\} from "(\.\/[^"]+\.js)";/g;
  const imports = [...content.matchAll(importRe)];
  if (imports.length === 0) {
    throw new Error('ensureAnsisImports: no relative chunk import found in branded logger chunk');
  }

  // Every binding already in scope across all imports (its local name).
  const localNames = new Set<string>();
  for (const [, bindings] of imports) {
    for (const binding of bindings.split(',')) {
      const trimmed = binding.trim();
      if (!trimmed) {
        continue;
      }
      const aliased = trimmed.match(/\bas\s+([A-Za-z0-9_$]+)$/);
      localNames.add(aliased ? aliased[1] : trimmed);
    }
  }
  const missing = names.filter((name) => !localNames.has(name));
  if (missing.length === 0) {
    return content;
  }

  // Group missing colors by the imported chunk that re-exports them. Chunks
  // re-export colors as `<local> as <alias>` (e.g. `bold as i`); the consumer
  // side imports `<alias> as <local>`, so capture the alias here.
  const additionsBySpecifier = new Map<string, string[]>();
  for (const name of missing) {
    let resolved = false;
    for (const [, , specifier] of imports) {
      const chunkContent = await readFile(join(distDir, specifier.slice(2)), 'utf-8');
      const exportAlias = chunkContent.match(new RegExp(`\\b${name} as ([A-Za-z0-9_$]+)`));
      if (!exportAlias) {
        continue;
      }
      const additions = additionsBySpecifier.get(specifier) ?? [];
      additions.push(`${exportAlias[1]} as ${name}`);
      additionsBySpecifier.set(specifier, additions);
      resolved = true;
      break;
    }
    if (!resolved) {
      throw new Error(`ensureAnsisImports: \`${name}\` is not re-exported from any imported chunk`);
    }
  }

  let result = content;
  for (const [fullImport, bindings, specifier] of imports) {
    const additions = additionsBySpecifier.get(specifier);
    if (!additions) {
      continue;
    }
    const newImport = `import { ${bindings.trim().replace(/,$/, '')}, ${additions.join(', ')} } from "${specifier}";`;
    result = result.replace(fullImport, newImport);
  }
  return result;
}

async function brandTsdown() {
  const tsdownDistDir = join(projectDir, 'dist/tsdown');
  const buildFiles = await glob(toPosixPath(join(tsdownDistDir, 'build-*.js')), { absolute: true });
  // The logger code lives in a shared chunk whose name depends on rolldown's
  // chunking (e.g. `main-*.js` or `debug-*.js`), so scan every chunk for it.
  const loggerCandidateFiles = await glob(toPosixPath(join(tsdownDistDir, '*.js')), {
    absolute: true,
  });
  if (buildFiles.length === 0) {
    throw new Error('brandTsdown: no build chunk found in dist/tsdown/');
  }

  const search = '"tsdown <your-file>"';
  const replacement = '"vp pack <your-file>"';
  const buildErrorPatches = [
    {
      search:
        'else throw new Error(`${nameLabel} No input files, try "vp pack <your-file>" or create src/index.ts`);',
      replacement:
        'else throw new Error(`${nameLabel ? `${nameLabel} ` : ""}No input files, try "vp pack <your-file>" or create src/index.ts`);',
    },
    {
      search:
        'if (entries.length === 0) throw new Error(`${nameLabel} Cannot find entry: ${JSON.stringify(entry)}`);',
      replacement:
        'if (entries.length === 0) throw new Error(`${nameLabel ? `${nameLabel} ` : ""}Cannot find entry: ${JSON.stringify(entry)}`);',
    },
  ];
  let patched = false;
  let buildErrorsPatched = false;

  for (const buildFile of buildFiles) {
    let content = await readFile(buildFile, 'utf-8');
    let changed = false;
    if (!content.includes(search)) {
      // Keep going to apply other safety patches below.
    } else {
      content = content.replace(search, replacement);
      console.log(`Branded tsdown → vp pack in ${buildFile}`);
      patched = true;
      changed = true;
    }

    for (const { search, replacement } of buildErrorPatches) {
      if (content.includes(search)) {
        content = content.replaceAll(search, replacement);
        buildErrorsPatched = true;
        changed = true;
      }
    }

    if (changed) {
      await writeFile(buildFile, content, 'utf-8');
    }
  }

  if (!patched) {
    throw new Error(`brandTsdown: pattern ${search} not found in any build chunk`);
  }
  if (!buildErrorsPatched) {
    throw new Error('brandTsdown: build error message patterns not found in any build chunk');
  }

  const loggerPatches = [
    {
      search: 'output("warn", `\\n${bgYellow` WARN `} ${message}\\n`);',
      replacement: 'output("warn", `${bold(yellow`warn:`)} ${message}`);',
    },
    {
      search: 'output("warn", `${bgYellow` WARN `} ${message}\\n`);',
      replacement: 'output("warn", `${bold(yellow`warn:`)} ${message}`);',
    },
    {
      search: 'output("error", `\\n${bgRed` ERROR `} ${format(msgs)}\\n`);',
      replacement:
        'output("error", `${bold(red`error:`)} ${format(msgs).replace(/^([A-Za-z]*Error):\\s*/, "")}`);',
    },
    {
      search: 'output("error", `${bgRed` ERROR `} ${format(msgs)}\\n`);',
      replacement:
        'output("error", `${bold(red`error:`)} ${format(msgs).replace(/^([A-Za-z]*Error):\\s*/, "")}`);',
    },
    {
      search: 'output("error", `${bold(red`error:`)} ${format(msgs)}`);',
      replacement:
        'output("error", `${bold(red`error:`)} ${format(msgs).replace(/^([A-Za-z]*Error):\\s*/, "")}`);',
    },
  ];
  let loggerPatched = false;

  for (const candidateFile of loggerCandidateFiles) {
    let content = await readFile(candidateFile, 'utf-8');
    let changed = false;
    for (const { search, replacement } of loggerPatches) {
      if (content.includes(search)) {
        content = content.replaceAll(search, replacement);
        changed = true;
      }
    }
    if (!changed) {
      continue;
    }
    // The branded logger output uses `bold(...)` and `red` (see loggerPatches),
    // but tsdown's logger module only imports the other ansis colors it needs
    // (`bgRed`, `bgYellow`, `yellow`, ...). Those identifiers only happened to be
    // in scope when rolldown co-located them in this chunk; newer chunking splits
    // them out, leaving `bold`/`red` undefined at runtime. Ensure the branded
    // chunk imports them from the same shared chunk it already pulls colors from.
    content = await ensureAnsisImports(content, ['bold', 'red'], tsdownDistDir);
    await writeFile(candidateFile, content, 'utf-8');
    console.log(`Branded tsdown logger prefixes in ${candidateFile}`);
    loggerPatched = true;
  }

  if (!loggerPatched) {
    throw new Error('brandTsdown: logger prefix patterns not found in any chunk');
  }
}

// Wire the bundled `@tsdown/exe` and `@tsdown/css` extensions into the bundled
// tsdown so they load from the local chunks instead of resolving external
// top-level packages. See https://github.com/voidzero-dev/vite-plus/issues/1586
async function wireBundledTsdownExtensions() {
  const tsdownDistDir = join(projectDir, 'dist/tsdown');
  // Scan every emitted chunk, not just `build-*.js`: which chunk rolldown hoists
  // these call sites into depends on its chunking heuristics, so don't pin to one.
  const chunkFiles = await glob(toPosixPath(join(tsdownDistDir, '*.js')), { absolute: true });
  if (chunkFiles.length === 0) {
    throw new Error('wireBundledTsdownExtensions: no chunk found in dist/tsdown/');
  }

  // Route `@tsdown/exe` and `@tsdown/css` to the bundled entries.
  //   - `importWithError("@tsdown/exe")` dynamically imports a runtime string,
  //     which rolldown cannot follow, so rewrite the call site to the chunk.
  //   - `pkgExists("@tsdown/css")` resolves the top-level package at runtime;
  //     since it is bundled now, force it on and point the import at the chunk.
  // `@tsdown/css` still imports `lightningcss` (a native module that cannot be
  // bundled), which resolves to core's own `lightningcss` dependency.
  let exeWired = false;
  let cssWired = false;
  // The `import("@tsdown/css")` call site may already be deduped to the bundled
  // entry by rolldown; track whether the bundled load ends up referenced either
  // way so a silent miss (no rewrite and no dedup) fails the build.
  let cssLoadWired = false;
  // Newer rolldown neither leaves `import("@tsdown/css")` as a literal nor dedupes
  // it to the `tsdown-css` entry: it splits `@tsdown/css` into its own generated
  // chunk (`import("./dist-<hash>.js")`), leaving a redundant duplicate of the
  // `tsdown-css.js` bundle. Collect those generated chunks (keyed off the stable
  // `{ CssPlugin }` destructure, not the hashed name) so they can be dropped once
  // the call site is repointed at the stable entry.
  const redundantCssChunks = new Set<string>();
  for (const chunkFile of chunkFiles) {
    let content = await readFile(chunkFile, 'utf-8');
    let changed = false;
    if (content.includes('importWithError("@tsdown/exe")')) {
      content = content.replaceAll('importWithError("@tsdown/exe")', 'import("./tsdown-exe.js")');
      exeWired = true;
      changed = true;
    }
    if (content.includes('pkgExists("@tsdown/css")')) {
      content = content.replaceAll('pkgExists("@tsdown/css")', 'true');
      cssWired = true;
      changed = true;
    }
    if (content.includes('import("@tsdown/css")')) {
      content = content.replaceAll('import("@tsdown/css")', 'import("./tsdown-css.js")');
      changed = true;
    }
    const cssChunkImport = content.match(/const \{ CssPlugin \} = await import\("(\.\/[^"]+)"\)/);
    if (cssChunkImport && cssChunkImport[1] !== './tsdown-css.js') {
      content = content.replaceAll(
        cssChunkImport[0],
        'const { CssPlugin } = await import("./tsdown-css.js")',
      );
      redundantCssChunks.add(cssChunkImport[1].slice(2));
      changed = true;
    }
    if (content.includes('import("./tsdown-css.js")')) {
      cssLoadWired = true;
    }
    if (changed) {
      await writeFile(chunkFile, content);
    }
  }
  // Drop the now-orphaned duplicate `@tsdown/css` chunks emitted by rolldown.
  for (const chunk of redundantCssChunks) {
    await rm(join(tsdownDistDir, chunk), { force: true });
  }
  if (!exeWired) {
    throw new Error('wireBundledTsdownExtensions: `importWithError("@tsdown/exe")` not found');
  }
  if (!cssWired) {
    throw new Error('wireBundledTsdownExtensions: `pkgExists("@tsdown/css")` not found');
  }
  if (!cssLoadWired) {
    throw new Error('wireBundledTsdownExtensions: bundled `./tsdown-css.js` is never imported');
  }
}

async function mergePackageJson() {
  const tsdownPkgPath = join(tsdownSourceDir, 'package.json');
  const rolldownPkgPath = join(rolldownSourceDir, 'package.json');
  const vitePkgPath = join(rolldownViteSourceDir, 'package.json');
  const destPkgPath = resolve(projectDir, 'package.json');

  const tsdownPkg = JSON.parse(await readFile(tsdownPkgPath, 'utf-8'));
  const rolldownPkg = JSON.parse(await readFile(rolldownPkgPath, 'utf-8'));
  const vitePkg = JSON.parse(await readFile(vitePkgPath, 'utf-8'));
  const destPkg = JSON.parse(await readFile(destPkgPath, 'utf-8'));

  // Merge peerDependencies from tsdown and vite
  destPkg.peerDependencies = {
    ...tsdownPkg.peerDependencies,
    ...vitePkg.peerDependencies,
  };

  // Merge peerDependenciesMeta from tsdown and vite
  destPkg.peerDependenciesMeta = {
    ...tsdownPkg.peerDependenciesMeta,
    ...vitePkg.peerDependenciesMeta,
  };

  // `@tsdown/exe` and `@tsdown/css` are bundled into core (see bundleTsdown), so
  // they must not be advertised as peers anymore. `lightningcss` (which the
  // bundled `@tsdown/css` and Vite's lightningcss transformer use) stays a core
  // `dependency`, kept in lockstep with `@tsdown/css` by the upgrade-deps script,
  // so CSS bundling works without any extra install.
  for (const bundled of ['@tsdown/exe', '@tsdown/css']) {
    delete destPkg.peerDependencies[bundled];
    delete destPkg.peerDependenciesMeta[bundled];
  }

  destPkg.bundledVersions = {
    ...destPkg.bundledVersions,
    vite: vitePkg.version,
    rolldown: rolldownPkg.version,
    tsdown: tsdownPkg.version,
  };

  const { code, errors } = await format(destPkgPath, JSON.stringify(destPkg, null, 2) + '\n', {
    sortPackageJson: true,
  });
  if (errors.length > 0) {
    for (const error of errors) {
      console.error(error);
    }
    process.exit(1);
  }
  await writeFile(destPkgPath, code);
}

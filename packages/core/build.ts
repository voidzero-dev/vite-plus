import { existsSync } from 'node:fs';
import { copyFile, cp, mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { dirname, join, parse, resolve, relative } from 'node:path';
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
import { buildCjsDeps } from './build-support/build-cjs-deps.js';
import { replaceThirdPartyCjsRequires } from './build-support/find-create-require.js';
import { RewriteImportsPlugin } from './build-support/rewrite-imports.js';
import {
  createRolldownRewriteRules,
  createViteRewriteRules,
  rewriteModuleSpecifiers,
  type ReplacementRule,
} from './build-support/rewrite-module-specifiers.js';
import pkgJson from './package.json' with { type: 'json' };
import viteRolldownConfig from './vite-rolldown.config.js';

const projectDir = join(fileURLToPath(import.meta.url), '..');

const rolldownPluginUtilsDir = resolve(
  projectDir,
  '..',
  '..',
  'rolldown',
  'packages',
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
await bundleVitepress();
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
      licensePath: join(projectDir, '..', '..', 'rolldown', 'LICENSE'),
    },
    {
      packageDir: rolldownViteSourceDir,
    },
    {
      packageDir: tsdownSourceDir,
    },
    {
      packageDir: join(projectDir, '..', '..', 'node_modules', 'vitepress'),
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
        {
          name: 'suppress-vite-version-only-reporter-line',
          transform(code, id) {
            if (!id.endsWith(join('vite', 'src', 'node', 'plugins', 'reporter.ts'))) {
              return undefined;
            }

            // Upstream native reporter can emit a redundant standalone "vite vX.Y.Z" line.
            // Filter it at source so snapshots and CLI output remain stable.
            if (code.includes('VITE_VERSION_ONLY_LINE_RE')) {
              return undefined;
            }

            const constLine =
              'const COMPRESSIBLE_ASSETS_RE = /\\.(?:html|json|svg|txt|xml|xhtml|wasm)$/';
            const logInfoLine =
              '        logInfo: shouldLogInfo ? (msg) => env.logger.info(msg) : undefined,';

            if (!code.includes(constLine) || !code.includes(logInfoLine)) {
              return undefined;
            }

            return {
              code: code
                .replace(
                  constLine,
                  `${constLine}\nconst VITE_VERSION_ONLY_LINE_RE = /^vite v\\S+$/`,
                )
                .replace(
                  logInfoLine,
                  `        logInfo: shouldLogInfo
          ? (msg) => {
              // Keep transformed/chunk/gzip logs but suppress redundant version-only line.
              if (VITE_VERSION_ONLY_LINE_RE.test(msg.trim())) {
                return
              }
              env.logger.info(msg)
            }
          : undefined,`,
                ),
            };
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
    const rewrittenFile = rewriteModuleSpecifiers(file, srcDtsFile, {
      rules: [...createViteRewriteRules(pkgJson.name), ...createRolldownRewriteRules(pkgJson.name)],
    });
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
      if (process.env.RELEASE_BUILD) {
        const rolldownBindingVersion = (
          await import(toPosixPath(relative(projectDir, join(rolldownSourceDir, 'package.json'))), {
            with: { type: 'json' },
          })
        ).default.version;
        // @rolldown/binding-darwin-arm64 → @voidzero-dev/vite-plus-darwin-arm64/binding
        source = source.replace(/@rolldown\/binding-([a-z0-9-]+)/g, 'vite-plus/binding');
        source = source.replaceAll(`${rolldownBindingVersion}`, pkgJson.version);
      }
      const newSource = rewriteModuleSpecifiers(source, file, { rules });
      await writeFile(file, newSource);
    }
  }
}

async function bundleTsdown() {
  await mkdir(join(projectDir, 'dist/tsdown/dist'), { recursive: true });

  const tsdownExternal = Object.keys(pkgJson.peerDependencies);

  const thirdPartyCjsModules = new Set<string>();

  // Re-build tsdown cli
  await build({
    input: {
      run: join(tsdownSourceDir, 'dist/run.mjs'),
      index: join(tsdownSourceDir, 'dist/index.mjs'),
    },
    output: {
      format: 'esm',
      cleanDir: true,
      dir: join(projectDir, 'dist/tsdown'),
    },
    platform: 'node',
    external: (id: string) => tsdownExternal.some((e) => id.startsWith(e)),
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
    external: (id: string) => tsdownExternal.some((e) => id.startsWith(e)),
    plugins: [
      RewriteImportsPlugin,
      dts({
        oxc: true,
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

async function brandTsdown() {
  const tsdownDistDir = join(projectDir, 'dist/tsdown');
  const buildFiles = await glob(toPosixPath(join(tsdownDistDir, 'build-*.js')), { absolute: true });
  const mainFiles = await glob(toPosixPath(join(tsdownDistDir, 'main-*.js')), { absolute: true });
  if (buildFiles.length === 0) {
    throw new Error('brandTsdown: no build chunk found in dist/tsdown/');
  }
  if (mainFiles.length === 0) {
    throw new Error('brandTsdown: no main chunk found in dist/tsdown/');
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

  for (const mainFile of mainFiles) {
    let content = await readFile(mainFile, 'utf-8');
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
    await writeFile(mainFile, content, 'utf-8');
    console.log(`Branded tsdown logger prefixes in ${mainFile}`);
    loggerPatched = true;
  }

  if (!loggerPatched) {
    throw new Error('brandTsdown: logger prefix patterns not found in any main chunk');
  }
}

// Actually do nothing now, we will polish it in the future when `vitepress` is ready
async function bundleVitepress() {
  const vitepressSourceDir = resolve(projectDir, 'node_modules/vitepress');
  const vitepressDestDir = join(projectDir, 'dist/vitepress');

  await mkdir(vitepressDestDir, { recursive: true });

  // Copy dist directory
  // Normalize glob pattern to use forward slashes on Windows
  const vitepressDistFiles = await glob(toPosixPath(join(vitepressSourceDir, 'dist', '**/*')), {
    absolute: true,
  });

  for (const file of vitepressDistFiles) {
    const stats = await stat(file);
    if (!stats.isFile()) {
      continue;
    }

    // Normalize paths to use forward slashes for consistent replacement on Windows
    const relativePath = toPosixPath(file).replace(
      toPosixPath(join(vitepressSourceDir, 'dist')),
      '',
    );
    const destPath = join(vitepressDestDir, relativePath);

    await mkdir(parse(destPath).dir, { recursive: true });

    // Rewrite vite imports in .js and .mjs files
    if (
      file.endsWith('.js') ||
      file.endsWith('.mjs') ||
      file.endsWith('.d.mts') ||
      file.endsWith('.d.ts')
    ) {
      const content = await readFile(file, 'utf-8');
      // Note: For vitepress, 'vite' -> 'pkgJson.name/vite' (vite subpath)
      const rewrittenContent = rewriteModuleSpecifiers(content, file, {
        rules: [{ from: 'vite', to: `${pkgJson.name}/vite` }],
      });
      await writeFile(destPath, rewrittenContent, 'utf-8');
    } else {
      await copyFile(file, destPath);
    }
  }

  // Copy top-level .d.ts files
  const vitepressTypeFiles = ['client.d.ts', 'theme.d.ts', 'theme-without-fonts.d.ts'];
  for (const typeFile of vitepressTypeFiles) {
    const sourcePath = join(vitepressSourceDir, typeFile);
    const destPath = join(vitepressDestDir, typeFile);
    try {
      await copyFile(sourcePath, destPath);
    } catch {
      // File might not exist, skip
    }
  }

  // Copy types directory
  const vitepressTypesDir = join(vitepressSourceDir, 'types');
  const vitepressTypesDestDir = join(vitepressDestDir, 'types');
  await mkdir(vitepressTypesDestDir, { recursive: true });

  // Normalize glob pattern to use forward slashes on Windows
  const vitepressTypesFiles = await glob(toPosixPath(join(vitepressTypesDir, '**/*')), {
    absolute: true,
  });

  for (const file of vitepressTypesFiles) {
    const stats = await stat(file);
    if (!stats.isFile()) {
      continue;
    }

    // Normalize paths to use forward slashes for consistent replacement on Windows
    const relativePath = toPosixPath(file).replace(toPosixPath(vitepressTypesDir), '');
    const destPath = join(vitepressTypesDestDir, relativePath);

    await mkdir(parse(destPath).dir, { recursive: true });
    await copyFile(file, destPath);
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

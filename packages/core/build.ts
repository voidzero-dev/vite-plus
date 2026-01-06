import { existsSync } from 'node:fs';
import { copyFile, cp, mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { dirname, join, parse, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { build, type BuildOptions } from 'rolldown';
import { dts } from 'rolldown-plugin-dts';
import { glob } from 'tinyglobby';

import { buildCjsDeps } from './build-support/build-cjs-deps';
import { replaceThirdPartyCjsRequires } from './build-support/find-create-require';
import { RewriteImportsPlugin } from './build-support/rewrite-imports';
import {
  createRolldownRewriteRules,
  createViteRewriteRules,
  rewriteModuleSpecifiers,
  type ReplacementRule,
} from './build-support/rewrite-module-specifiers';
import pkgJson from './package.json' with { type: 'json' };
import viteRolldownConfig from './vite-rolldown.config';

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

const rolldownViteSourceDir = resolve(projectDir, '..', '..', 'rolldown-vite', 'packages', 'vite');

const tsdownSourceDir = resolve(projectDir, 'node_modules/tsdown');

// Main build orchestration
await bundleRolldownPluginutils();
await bundleRolldown();
await buildVite();
await bundleTsdown();
await bundleVitepress();
await mergePackageJson();

async function buildVite() {
  const newViteRolldownConfig = viteRolldownConfig.map((config) => {
    config.tsconfig = join(projectDir, 'tsconfig.json');

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
          },
        },
        ...config.plugins.filter((plugin) => {
          return !(
            typeof plugin === 'object' &&
            plugin !== null &&
            'name' in plugin &&
            plugin.name === 'rollup-plugin-license'
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
  const dtsFiles = await glob(join(rolldownViteSourceDir, 'dist', 'node', '**/*.d.ts'), {
    absolute: true,
  });

  for (const dtsFile of dtsFiles) {
    const file = await readFile(dtsFile, 'utf-8');
    const dstFilePath = join(
      projectDir,
      'dist',
      'vite',
      'node',
      dtsFile.replace(join(rolldownViteSourceDir, 'dist', 'node'), ''),
    );
    const rewrittenFile = rewriteModuleSpecifiers(file, dtsFile, {
      rules: [...createViteRewriteRules(pkgJson.name), ...createRolldownRewriteRules(pkgJson.name)],
    });
    await writeFile(dstFilePath, rewrittenFile);
  }

  // Copy type files
  const srcTypeFiles = await glob(join(rolldownViteSourceDir, 'types', '**/*.d.ts'), {
    absolute: true,
  });

  await mkdir(join(projectDir, 'dist/vite/types'), { recursive: true });

  for (const srcDtsFile of srcTypeFiles) {
    const file = await readFile(srcDtsFile, 'utf-8');
    const dstFilePath = join(
      projectDir,
      'dist',
      'vite',
      'types',
      srcDtsFile.replace(join(rolldownViteSourceDir, 'types'), ''),
    );
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
      const source = await readFile(file, 'utf-8');
      const rules: ReplacementRule[] = [...createRolldownRewriteRules(pkgJson.name)];
      if (process.env.RELEASE_BUILD) {
        rules.push({ from: '../rolldown-binding', to: './rolldown-binding' });
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
    plugins: [
      RewriteImportsPlugin,
      dts({
        oxc: true,
        dtsInput: true,
      }),
    ],
  });
}

// Actually do nothing now, we will polish it in the future when `vitepress` is ready
async function bundleVitepress() {
  const vitepressSourceDir = resolve(projectDir, 'node_modules/vitepress');
  const vitepressDestDir = join(projectDir, 'dist/vitepress');

  await mkdir(vitepressDestDir, { recursive: true });

  // Copy dist directory
  const vitepressDistFiles = await glob(join(vitepressSourceDir, 'dist', '**/*'), {
    absolute: true,
  });

  for (const file of vitepressDistFiles) {
    const stats = await stat(file);
    if (!stats.isFile()) continue;

    const relativePath = file.replace(join(vitepressSourceDir, 'dist'), '');
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

  const vitepressTypesFiles = await glob(join(vitepressTypesDir, '**/*'), {
    absolute: true,
  });

  for (const file of vitepressTypesFiles) {
    const stats = await stat(file);
    if (!stats.isFile()) continue;

    const relativePath = file.replace(vitepressTypesDir, '');
    const destPath = join(vitepressTypesDestDir, relativePath);

    await mkdir(parse(destPath).dir, { recursive: true });
    await copyFile(file, destPath);
  }
}

async function mergePackageJson() {
  const tsdownPkgPath = join(tsdownSourceDir, 'package.json');
  const vitePkgPath = join(rolldownViteSourceDir, 'package.json');
  const destPkgPath = resolve(projectDir, 'package.json');

  const tsdownPkg = JSON.parse(await readFile(tsdownPkgPath, 'utf-8'));
  const vitePkg = JSON.parse(await readFile(vitePkgPath, 'utf-8'));
  const destPkg = JSON.parse(await readFile(destPkgPath, 'utf-8'));

  // Merge peerDependencies from tsdown and rolldown-vite
  destPkg.peerDependencies = {
    ...tsdownPkg.peerDependencies,
    ...vitePkg.peerDependencies,
  };

  // Merge peerDependenciesMeta from tsdown and rolldown-vite
  destPkg.peerDependenciesMeta = {
    ...tsdownPkg.peerDependenciesMeta,
    ...vitePkg.peerDependenciesMeta,
  };

  await writeFile(destPkgPath, JSON.stringify(destPkg, null, 2) + '\n');
}

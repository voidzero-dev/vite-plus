import { copyFile, cp, mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { join, parse, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { build, type BuildOptions } from 'rolldown';
import { glob } from 'tinyglobby';

import { RewriteImportsPlugin } from './build-support/rewrite-imports';
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
          (external instanceof RegExp && external.test('rolldown/'))
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
        ...config.plugins
          .filter((plugin) => {
            return !(
              typeof plugin === 'object' &&
              plugin !== null &&
              'name' in plugin &&
              plugin.name === 'rollup-plugin-license'
            );
          })
          .map((plugin) => {
            if (
              typeof plugin === 'object' &&
              plugin !== null &&
              'name' in plugin &&
              plugin.name === 'externalize-vite'
            ) {
              return RewriteImportsPlugin;
            }
            return plugin;
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
    await writeFile(
      dstFilePath,
      file
        .replaceAll(`"rolldown-vite/`, `"${pkgJson.name}/`)
        .replaceAll(`"rolldown-vite"`, `"${pkgJson.name}/vite"`)
        .replaceAll(`"rolldown/`, `"${pkgJson.name}/rolldown/`)
        .replaceAll(`"rolldown"`, `"${pkgJson.name}/rolldown"`),
    );
  }

  // Copy type files
  const srcTypeFiles = await glob(join(rolldownViteSourceDir, 'types', '**/*.d.ts'), {
    absolute: true,
  });

  await mkdir(join(projectDir, 'dist/vite/types'), { recursive: true });

  for (const srcDtsFile of srcTypeFiles) {
    await cp(
      srcDtsFile,
      join(
        projectDir,
        'dist/vite/types',
        srcDtsFile.replace(join(rolldownViteSourceDir, 'types'), ''),
      ),
    );
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

  // Rewrite @rolldown/pluginutils imports
  for (const file of rolldownFiles) {
    if (file.endsWith('.mjs') || file.endsWith('.js')) {
      const source = await readFile(file, 'utf-8');
      let newSource = source.replaceAll(
        '"@rolldown/pluginutils"',
        `"${pkgJson.name}/rolldown/pluginutils"`,
      );
      if (process.env.RELEASE_BUILD) {
        newSource = newSource.replaceAll(
          `__require("../rolldown-binding`,
          `__require("./rolldown-binding`,
        );
      }
      await writeFile(file, newSource);
    }
  }
}

async function bundleTsdown() {
  await mkdir(join(projectDir, 'dist/tsdown/dist'), { recursive: true });

  const tsdownExternal = Object.keys(pkgJson.peerDependencies);

  // Re-build tsdown cli
  await build({
    input: join(tsdownSourceDir, 'dist/run.mjs'),
    output: {
      format: 'esm',
      cleanDir: true,
      dir: join(projectDir, 'dist/tsdown'),
    },
    platform: 'node',
    external: (id: string) => tsdownExternal.some((e) => id.startsWith(e)),
    plugins: [RewriteImportsPlugin],
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
    if (file.endsWith('.js') || file.endsWith('.mjs')) {
      let content = await readFile(file, 'utf-8');
      content = content.replaceAll(/from ['"]vite['"]/g, `from '${pkgJson.name}/vite'`);
      content = content.replaceAll(/import\(['"]vite['"]\)/g, `import('${pkgJson.name}/vite')`);
      content = content.replaceAll(/require\(['"]vite['"]\)/g, `require('${pkgJson.name}/vite')`);
      await writeFile(destPath, content, 'utf-8');
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

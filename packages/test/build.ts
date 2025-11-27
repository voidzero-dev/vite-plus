import { copyFile, glob as fsGlob, mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { join, parse, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectDir = dirname(fileURLToPath(import.meta.url));
const vitestSourceDir = resolve(projectDir, 'node_modules/vitest-dev');

const CORE_PACKAGE_NAME = '@voidzero-dev/vite-plus-core';

await bundleVitest();
await mergePackageJson();

async function mergePackageJson() {
  const vitestPackageJsonPath = join(vitestSourceDir, 'package.json');
  const destPackageJsonPath = resolve(projectDir, 'package.json');

  const vitestPkg = JSON.parse(await readFile(vitestPackageJsonPath, 'utf-8'));
  const destPkg = JSON.parse(await readFile(destPackageJsonPath, 'utf-8'));

  // Fields to merge from vitest-dev package.json
  const fieldsToMerge = [
    'imports',
    'exports',
    'main',
    'module',
    'types',
    'engines',
    'peerDependencies',
    'peerDependenciesMeta',
    'dependencies',
  ] as const;

  for (const field of fieldsToMerge) {
    if (vitestPkg[field] !== undefined) {
      destPkg[field] = vitestPkg[field];
    }
  }

  // Replace vite dependency with @voidzero-dev/vite-plus-core
  if (destPkg.dependencies && destPkg.dependencies.vite) {
    delete destPkg.dependencies.vite;
    destPkg.dependencies[CORE_PACKAGE_NAME] = 'workspace:*';
  }

  await writeFile(destPackageJsonPath, JSON.stringify(destPkg, null, 2) + '\n');
}

async function bundleVitest() {
  const vitestDestDir = projectDir;

  await mkdir(vitestDestDir, { recursive: true });

  // Get all vitest files excluding node_modules and package.json
  const vitestFiles = fsGlob(join(vitestSourceDir, '**/*'), {
    exclude: [
      join(vitestSourceDir, 'node_modules/**'),
      join(vitestSourceDir, 'package.json'),
      join(vitestSourceDir, 'README.md'),
    ],
  });

  for await (const file of vitestFiles) {
    const stats = await stat(file);
    if (!stats.isFile()) continue;

    const relativePath = file.replace(vitestSourceDir, '');
    const destPath = join(vitestDestDir, relativePath);

    await mkdir(parse(destPath).dir, { recursive: true });

    // Rewrite vite imports in .js, .mjs, and .cjs files
    if (
      file.endsWith('.js') ||
      file.endsWith('.mjs') ||
      file.endsWith('.cjs') ||
      file.endsWith('.d.ts')
    ) {
      let content = await readFile(file, 'utf-8');
      content = content
        .replaceAll(/from ['"]vite['"]/g, `from '${CORE_PACKAGE_NAME}'`)
        .replaceAll(/import\(['"]vite['"]\)/g, `import('${CORE_PACKAGE_NAME}')`)
        .replaceAll(/require\(['"]vite['"]\)/g, `require('${CORE_PACKAGE_NAME}')`)
        .replaceAll(/require\("vite"\)/g, `require("${CORE_PACKAGE_NAME}")`)
        .replaceAll(`import 'vite';`, `import '${CORE_PACKAGE_NAME}';`)
        .replaceAll(`'vite/module-runner'`, `'${CORE_PACKAGE_NAME}/module-runner'`)
        .replaceAll(`declare module "vite"`, `declare module "${CORE_PACKAGE_NAME}"`);
      await writeFile(destPath, content, 'utf-8');
    } else {
      await copyFile(file, destPath);
    }
  }
}

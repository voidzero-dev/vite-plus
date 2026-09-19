import { execSync } from 'node:child_process';
import { readFile, rename, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

async function replaceOnce(file: string, original: string, replacement: string): Promise<void> {
  const source = await readFile(file, 'utf8');
  if (source.split(original).length !== 2) {
    throw new Error(`WebDriverIO fixture patch: expected one occurrence of ${original} in ${file}`);
  }
  await writeFile(file, source.replace(original, replacement));
}

export async function prepareWebdriverioProject(
  project: string,
  root: string,
  cli: string,
): Promise<void> {
  if (project === '10ten-ja-reader') {
    // Match upstream CI's Node pin for its locked WebDriverIO/Undici versions.
    // https://github.com/birchill/10ten-ja-reader/blob/7eabad2d5075ce278c580a59c4deb6b2d794383f/.github/workflows/ci.yml
    await writeFile(join(root, '.node-version'), '24.15.0\n');

    // Give formatter migration an existing, loadable Vite config. The app uses
    // Rspack; moving this test config does not change the browser project.
    await rename(join(root, 'vitest.config.ts'), join(root, 'vite.config.ts'));
    // Oxfmt does not support this Prettier plugin option, and its $DEFAULT
    // token is interpreted as an ast-grep variable when merging the config.
    // Remove this workaround when formatter migration drops unsupported options.
    const prettierPath = join(root, '.prettierrc.json');
    const prettier = JSON.parse(await readFile(prettierPath, 'utf8'));
    if (!prettier.attributeGroups?.includes('$DEFAULT')) {
      throw new Error('10ten-ja-reader patch: expected the pinned attributeGroups option');
    }
    delete prettier.attributeGroups;
    await writeFile(prettierPath, `${JSON.stringify(prettier, null, 2)}\n`);
  }

  if (project === 'sqlocal') {
    // Vitest 5's browser server does not run the SQLocal plugin's middleware.
    // Keep the same isolation headers explicitly; OPFS tests assert isolation.
    // Remove when the upstream plugin supports the separate browser server.
    await replaceOnce(
      join(root, 'vite.config.ts'),
      'export default defineConfig({',
      `export default defineConfig({
  server: {
    headers: {
      'Cross-Origin-Embedder-Policy': 'require-corp',
      'Cross-Origin-Opener-Policy': 'same-origin',
    },
  },`,
    );

    // The migrator conservatively leaves benchmarks inside describe.each for
    // manual review. Preserve the callbacks and use the v5 context fixture.
    // Remove these patches when SQLocal adopts the new benchmark API.
    // https://vitest.dev/guide/migration/#benchmarking-api-rewrite
    for (const [file, title] of [
      ['batch.bench.ts', 'batch large replace'],
      ['reactive-bulk-write.bench.ts', 'query after bulk write with reactive queries enabled'],
    ]) {
      const filePath = join(root, 'test', 'benchmarks', file);
      await replaceOnce(
        filePath,
        "import { bench, describe } from 'vitest';",
        "import { test, describe } from 'vitest';",
      );
      await replaceOnce(
        filePath,
        `bench('${title}', async () => {`,
        `test('${title}', async ({ bench }) => {\n\t\t\tawait bench('${title}', async () => {`,
      );
      await replaceOnce(filePath, '\n\t\t});\n\t}\n);', '\n\t\t\t}).run();\n\t\t});\n\t}\n);');
    }
  }

  if (project === 'brazilian-utils') {
    // npm's lockfile does not establish the bundled runner version by itself.
    // Install the original graph so preflight can identify Vite+ 0.x's Vitest.
    // This runs before changing any manifest and outside the local registry.
    execSync(`${cli} install --frozen-lockfile --ignore-scripts`, {
      cwd: root,
      stdio: 'inherit',
    });

    // This multi-runner adapter escapes the old bench API, so migration cannot
    // rewrite it automatically. Keep benchmarks as todo tests in normal mode.
    // Remove when upstream adopts the v5 fixture-based benchmark API.
    // https://vitest.dev/guide/migration/#benchmarking-api-rewrite
    const runtimePath = join(root, 'src', '_internals', 'test', 'runtime-vitest.ts');
    await replaceOnce(
      runtimePath,
      'import { bench as vitestBench, test } from "vite-plus/test";\n\nimport { bench as noopBench } from "./noop";',
      'import { test } from "vite-plus/test";',
    );
    await replaceOnce(
      runtimePath,
      `/**
 * In benchmark mode (\`npm run bench\`) this is vitest's own \`bench\`. In test mode every benchmark
 * is registered as a todo test instead, so a \`describe("<name> benchmarks")\` block is never an
 * empty suite (which vitest reports as a failure) and shows up in the run as todo.
 */
export const bench: typeof vitestBench = isBenchmarkMode()
\t? vitestBench
\t: Object.assign((name: string): void => {
\t\t\ttest.todo(name);
\t\t}, noopBench);`,
      `// Vitest 5 uses a test-context fixture; retain upstream's todo behavior in test mode.
export const bench = (name: string, callback: () => void | Promise<void>): void => {
  if (!isBenchmarkMode()) {
    test.todo(name);
    return;
  }
  test(name, async ({ bench }) => {
    await bench(name, callback).run();
  });
};`,
    );
    for (const file of ['runtime.ts', 'noop.ts']) {
      await replaceOnce(
        join(root, 'src', '_internals', 'test', file),
        'import { type bench as vitestBench, type expectTypeOf as vitestExpectTypeOf } from "vite-plus/test";',
        'import { type expectTypeOf as vitestExpectTypeOf } from "vite-plus/test";\nimport type { bench as vitestBench } from "./runtime-vitest";',
      );
    }
    await replaceOnce(
      join(root, 'package.json'),
      '"bench": "vp test bench --run"',
      '"bench": "vp test run --mode benchmark"',
    );
  }
}

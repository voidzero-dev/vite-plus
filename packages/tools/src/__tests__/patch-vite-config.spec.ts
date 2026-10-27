import { describe, expect, test } from 'vitest';

import { assertViteConfigPatched, patchViteConfig } from '../patch-vite-config.ts';

const upstream = `import path from 'node:path'

export async function resolveConfig(
  inlineConfig: InlineConfig,
  command: 'build' | 'serve',
  defaultMode = 'development',
  defaultNodeEnv = 'development',
  isPreview = false,
  /** @internal */
  patchConfig?: (config: ResolvedConfig) => void,
  /** @internal */
  patchPlugins?: (resolvedPlugins: Plugin[]) => void,
): Promise<ResolvedConfig> {
  let config = inlineConfig
  let { configFile } = config
  let configFileDependencies: string[] = []
  if (configFile !== false) {
    const loadResult = await loadConfigFromFile(configEnv, configFile)
    if (loadResult) {
      config = mergeConfig(loadResult.config, config)
      configFile = loadResult.path
      configFileDependencies = loadResult.dependencies
    }
  }
  mode = inlineConfig.mode || config.mode || mode
  config = await runConfigHook(config, userPlugins, configEnv)
}
`;

const internalParameter = `  /** @internal */
  internalOptions?: {
    rootResolution?: 'cwd' | 'config-file'
  },
`;

const rootBlock = `      if (
        internalOptions?.rootResolution === 'config-file' &&
        inlineConfig.root == null
      ) {
        config.root = path.resolve(
          path.dirname(loadResult.path),
          config.root ?? '',
        )
      }
`;

describe('patchViteConfig', () => {
  test('adds an internal option and resolves root after recording loaded config metadata', () => {
    const patched = patchViteConfig(upstream);

    expect(patched).toContain(internalParameter);
    expect(patched).toContain(
      `      configFileDependencies = loadResult.dependencies\n${rootBlock}`,
    );
    // All other source, including the first seven parameters and plugin order, stays intact.
    expect(patched.replace(internalParameter, '').replace(rootBlock, '')).toBe(upstream);
    expect(() => assertViteConfigPatched(patched)).not.toThrow();
    expect(patchViteConfig(patched)).toBe(patched);
  });

  test.each([
    ['changed signature', upstream.replace('defaultNodeEnv', 'defaultEnvironment')],
    [
      'changed load branch',
      upstream.replace(
        'mergeConfig(loadResult.config, config)',
        'mergeConfig(config, loadResult.config)',
      ),
    ],
    ['duplicate resolver', upstream + upstream],
    ['missing resolver', 'export const unrelated = true\n'],
    ['stray partial patch', `${upstream}\nconst rootResolution = 'config-file'\n`],
  ])('rejects %s', (_name, source) => {
    expect(() => patchViteConfig(source)).toThrow(/changed, duplicate, or partial patch anchors/);
  });

  test('rejects partially applied or modified patches', () => {
    const patched = patchViteConfig(upstream);
    const incomplete = [
      patched.replace(internalParameter, ''),
      patched.replace(rootBlock, ''),
      patched.replace('inlineConfig.root == null', '!inlineConfig.root'),
      patched + patched,
    ];
    for (const source of incomplete) {
      expect(() => patchViteConfig(source)).toThrow(/changed, duplicate, or partial patch anchors/);
      expect(() => assertViteConfigPatched(source)).toThrow();
    }
  });
});

test('the build assertion rejects unpatched source with pin-update guidance', () => {
  expect(() => assertViteConfigPatched(upstream)).toThrow(/has not been applied/);
  expect(() => assertViteConfigPatched(upstream)).toThrow(/\.upstream-versions\.json/);
});

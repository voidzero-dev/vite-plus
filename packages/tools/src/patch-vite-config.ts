/**
 * Extend the pinned Vite resolver without replacing its config-loading pipeline.
 * Keep this transform free of dependencies: branding also runs before installation.
 */
const upstreamSignature = `export async function resolveConfig(
  inlineConfig: InlineConfig,
  command: 'build' | 'serve',
  defaultMode = 'development',
  defaultNodeEnv = 'development',
  isPreview = false,
  /** @internal */
  patchConfig?: (config: ResolvedConfig) => void,
  /** @internal */
  patchPlugins?: (resolvedPlugins: Plugin[]) => void,
): Promise<ResolvedConfig> {`;

const patchedSignature = upstreamSignature.replace(
  '): Promise<ResolvedConfig> {',
  `  /** @internal */
  internalOptions?: {
    rootBase?: string
  },
): Promise<ResolvedConfig> {`,
);

const upstreamLoadResult = `    if (loadResult) {
      config = mergeConfig(loadResult.config, config)
      configFile = loadResult.path
      configFileDependencies = loadResult.dependencies
    }`;

const patchedLoadResult = upstreamLoadResult.replace(
  '\n    }',
  `
      if (internalOptions?.rootBase !== undefined && inlineConfig.root == null) {
        config.root = path.resolve(internalOptions.rootBase, config.root ?? '')
      }
    }`,
);

function patchError(detail: string): Error {
  return new Error(
    `[patch-vite-config] ${detail} in vite/packages/vite/src/node/config.ts. ` +
      'Check the Vite pin in packages/tools/.upstream-versions.json and update patch-vite-config.ts.',
  );
}

function count(source: string, anchor: string): number {
  return source.split(anchor).length - 1;
}

function patchState(source: string): 'upstream' | 'patched' {
  const upstreamSignatures = count(source, upstreamSignature);
  const patchedSignatures = count(source, patchedSignature);
  const upstreamLoads = count(source, upstreamLoadResult);
  const patchedLoads = count(source, patchedLoadResult);
  if (
    upstreamSignatures === 1 &&
    upstreamLoads === 1 &&
    patchedSignatures === 0 &&
    patchedLoads === 0 &&
    !/\b(?:internalOptions|rootBase)\b/.test(source)
  ) {
    return 'upstream';
  }
  if (
    patchedSignatures === 1 &&
    patchedLoads === 1 &&
    upstreamSignatures === 0 &&
    upstreamLoads === 0 &&
    source.match(/\binternalOptions\b/g)?.length === 3 &&
    source.match(/\brootBase\b/g)?.length === 3
  ) {
    return 'patched';
  }
  throw patchError(
    'Expected one complete upstream or patched resolver; found changed, duplicate, or partial patch anchors',
  );
}

/** Validate the complete patch before a build can produce a core that ignores it. */
export function assertViteConfigPatched(source: string): void {
  if (patchState(source) !== 'patched') {
    throw patchError('The root base resolution patch has not been applied');
  }
}

/** Apply both edits together; reject upstream drift and incomplete prior edits. */
export function patchViteConfig(source: string): string {
  if (patchState(source) === 'patched') {
    return source;
  }
  const patched = source
    .replace(upstreamSignature, patchedSignature)
    .replace(upstreamLoadResult, patchedLoadResult);
  assertViteConfigPatched(patched);
  return patched;
}

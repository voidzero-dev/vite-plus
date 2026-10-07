import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join } from 'node:path';

type DependencyField = 'dependencies' | 'optionalDependencies';

export function getBundledDependencyRanges(
  root: string,
): { name: string; field: DependencyField; range: string }[] {
  // Core's direct rolldown-plugin-dts can differ from the copy bundled with tsdown.
  const coreRequire = createRequire(join(root, 'packages/core/package.json'));
  const tsdownRequire = createRequire(coreRequire.resolve('tsdown/package.json'));
  const pluginPath = tsdownRequire.resolve('rolldown-plugin-dts/package.json');
  const vitePath = join(root, 'vite/packages/vite/package.json');
  // These packages stay external in packages/core/build.ts. Mirror their
  // consumers' ranges rather than independently selecting the latest releases.
  const sources: { path: string; field: DependencyField; names: string[] }[] = [
    { path: pluginPath, field: 'dependencies', names: ['yuku-codegen', 'yuku-parser'] },
    { path: vitePath, field: 'dependencies', names: ['postcss'] },
    { path: vitePath, field: 'optionalDependencies', names: ['fsevents'] },
  ];
  return sources.flatMap(({ path, field, names }) => {
    const pkg = JSON.parse(readFileSync(path, 'utf8')) as Partial<
      Record<DependencyField, Record<string, string>>
    >;
    return names.map((name) => {
      const range = pkg[field]?.[name];
      if (typeof range !== 'string' || range.trim() === '') {
        throw new Error(`Missing ${field}.${name} in ${path}; review core's externals`);
      }
      return { name, field, range };
    });
  });
}

const STABLE_VERSION_RE = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;

type ParsedVersion = {
  major: number;
  minor: number;
  patch: number;
  version: string;
};

function parseStableVersion(version: string): ParsedVersion | undefined {
  const match = STABLE_VERSION_RE.exec(version);
  if (!match) {
    return undefined;
  }
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    version,
  };
}

function isNewer(candidate: ParsedVersion, current: ParsedVersion): boolean {
  return (
    candidate.minor > current.minor ||
    (candidate.minor === current.minor && candidate.patch > current.patch)
  );
}

export function findLatestStableVersionForMajor(
  versions: Iterable<string>,
  major: number,
): string | undefined {
  let latest: ParsedVersion | undefined;
  for (const version of versions) {
    const parsed = parseStableVersion(version);
    if (parsed?.major === major && (!latest || isNewer(parsed, latest))) {
      latest = parsed;
    }
  }
  return latest?.version;
}

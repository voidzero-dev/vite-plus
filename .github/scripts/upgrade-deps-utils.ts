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
  // These packages stay external in packages/core/build.ts. Follow their
  // bundled consumers rather than independently selecting the latest releases.
  const sources: { path: string; field: DependencyField; names: string[] }[] = [
    { path: pluginPath, field: 'dependencies', names: ['yuku-parser'] },
    { path: vitePath, field: 'dependencies', names: ['postcss'] },
    { path: vitePath, field: 'optionalDependencies', names: ['fsevents'] },
  ];
  return sources.flatMap(({ path, field, names }) => {
    const pkg = JSON.parse(readFileSync(path, 'utf8')) as Partial<
      Record<DependencyField, Record<string, string>>
    >;
    return names.map((name) => {
      let range = pkg[field]?.[name];
      if (typeof range !== 'string' || range.trim() === '') {
        throw new Error(`Missing ${field}.${name} in ${path}; review core's externals`);
      }
      if (name === 'yuku-parser') {
        // The AST helpers and codegen are bundled. Pin the runtime parser to
        // the exact version resolved by the same plugin so it cannot drift.
        const parserPath = createRequire(path).resolve('yuku-parser/package.json');
        const { version } = JSON.parse(readFileSync(parserPath, 'utf8')) as { version?: unknown };
        if (typeof version !== 'string' || version.trim() === '') {
          throw new Error(`Missing version in ${parserPath}; review core's externals`);
        }
        range = version;
      }
      return { name, field, range };
    });
  });
}

const VITE_TASK_GIT = 'git = "https://github.com/voidzero-dev/vite-task.git"';
const VITE_TASK_REV_RE =
  /(git = "https:\/\/github\.com\/voidzero-dev\/vite-task\.git", rev = ")([0-9a-f]{40})"/g;

// Every crate from vite-task shares one revision. Match the git source rather
// than crate names so crates added upstream are moved together.
export function setViteTaskRevision(
  cargoToml: string,
  revision: string,
): { content: string; previous: string } {
  const dependencyCount = cargoToml.split(VITE_TASK_GIT).length - 1;
  const revisions = Array.from(cargoToml.matchAll(VITE_TASK_REV_RE), (match) => match[2]);
  const [previous, ...rest] = new Set(revisions);
  if (!previous || rest.length || revisions.length !== dependencyCount) {
    throw new Error(
      `Expected every vite-task dependency in Cargo.toml to use \`${VITE_TASK_GIT}, rev = "<sha>"\` ` +
        'with one shared revision',
    );
  }
  return { content: cargoToml.replaceAll(VITE_TASK_REV_RE, `$1${revision}"`), previous };
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

export async function getLatestNodeVersion(major: number): Promise<string> {
  const response = await fetch('https://nodejs.org/dist/index.json');
  if (!response.ok) {
    throw new Error(`Failed to fetch Node.js releases: ${response.status} ${response.statusText}`);
  }
  const releases: unknown = await response.json();
  if (!Array.isArray(releases)) {
    throw new Error('Invalid Node.js release index: expected an array');
  }
  const versions = releases.flatMap((release: { version?: unknown } | null) =>
    typeof release?.version === 'string' ? [release.version.replace(/^v/, '')] : [],
  );
  const version = findLatestStableVersionForMajor(versions, major);
  if (!version) {
    throw new Error(`No stable ${major}.x version found for Node.js`);
  }
  return version;
}

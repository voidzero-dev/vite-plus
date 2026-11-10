import { readFileSync, writeFileSync } from 'node:fs';
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

export async function getPnpmInstallerIntegrity(version: string): Promise<string> {
  // Use the official registry regardless of the workspace's npm configuration.
  const response = await fetch(`https://registry.npmjs.org/pnpm/${version}`);
  if (!response.ok) {
    throw new Error(`Failed to fetch pnpm ${version}: ${response.status} ${response.statusText}`);
  }
  const data = (await response.json()) as {
    version?: unknown;
    dist?: { integrity?: unknown };
  } | null;
  if (data?.version !== version) {
    throw new Error(`Invalid npm response for pnpm ${version}: version mismatch`);
  }
  const integrity = data.dist?.integrity;
  const match = typeof integrity === 'string' && /^sha512-([A-Za-z0-9+/]{86}==)$/.exec(integrity);
  if (!match) {
    throw new Error(`Invalid npm response for pnpm ${version}: expected SHA-512 dist.integrity`);
  }
  const digest = Buffer.from(match[1], 'base64');
  // Buffer.from tolerates malformed base64, so require a canonical 64-byte digest.
  if (digest.length !== 64 || digest.toString('base64') !== match[1]) {
    throw new Error(`Invalid npm response for pnpm ${version}: malformed SHA-512 dist.integrity`);
  }
  return `sha512.${digest.toString('hex')}`;
}

export function updatePnpmInstallerPins(
  root: string,
  version: string,
  integrity: string,
): { oldVersion: string; oldIntegrity: string } {
  const files = [
    {
      path: 'crates/vp_setup/src/install.rs',
      pattern: /const PINNED_PNPM_VERSION: &str = "([^"]+)";/g,
    },
    {
      path: 'packages/cli/generate-install-lockfile.ts',
      pattern: /export const INSTALL_PNPM_VERSION = '([^']+)';/g,
    },
    {
      path: 'crates/vp_pm_cli/tests/package_manager_pin_verification.rs',
      pattern: /const PINNED_PNPM_VERSION: &str = "([^"]+)";/g,
    },
  ].map(({ path, pattern }) => {
    const filePath = join(root, path);
    const content = readFileSync(filePath, 'utf8');
    const matches = [...content.matchAll(pattern)];
    if (matches.length !== 1 || !parseStableVersion(matches[0][1])) {
      throw new Error(`Expected exactly one stable pnpm installer version pin in ${path}`);
    }
    return {
      filePath,
      content,
      oldVersion: matches[0][1],
      updated: content.replace(pattern, (match: string, old: string) =>
        match.replace(old, version),
      ),
    };
  });
  const installer = files[0];
  if (files.some((file) => file.oldVersion !== installer.oldVersion)) {
    throw new Error(
      'pnpm installer version pins are out of sync; update all three copies together',
    );
  }
  const integrityPattern = /const PINNED_PNPM_INTEGRITY: &str = "(sha512\.[a-f0-9]{128})";/g;
  const integrityMatches = [...installer.content.matchAll(integrityPattern)];
  if (integrityMatches.length !== 1) {
    throw new Error(
      `Expected exactly one SHA-512 pnpm installer integrity pin in ${installer.filePath}`,
    );
  }
  const oldIntegrity = integrityMatches[0][1];
  installer.updated = installer.updated.replace(integrityPattern, (match: string) =>
    match.replace(oldIntegrity, integrity),
  );
  // Validate every copy before writing so a stale pattern cannot leave partial pins.
  for (const file of files) {
    if (file.content !== file.updated) {
      writeFileSync(file.filePath, file.updated);
    }
  }
  return { oldVersion: installer.oldVersion, oldIntegrity };
}

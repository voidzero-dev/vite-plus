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

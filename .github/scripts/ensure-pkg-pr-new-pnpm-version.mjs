import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const LATEST_PNPM_10_VERSION = '10.34.4';
const SAFE_PNPM_11_VERSION = '11.9.0';
const SUPPORTED_PACKAGE_MANAGERS = new Set(['pnpm', 'yarn', 'npm', 'bun']);

function parseExactVersion(version) {
  const match = /^v?(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?$/.exec(version);
  if (!match) {
    return undefined;
  }
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    prerelease: match[4]?.split('.'),
  };
}

function compareIdentifiers(left, right) {
  const leftNumber = /^\d+$/.test(left) ? Number(left) : undefined;
  const rightNumber = /^\d+$/.test(right) ? Number(right) : undefined;
  if (leftNumber !== undefined && rightNumber !== undefined) {
    return leftNumber - rightNumber;
  }
  if (leftNumber !== undefined) {
    return -1;
  }
  if (rightNumber !== undefined) {
    return 1;
  }
  return left.localeCompare(right);
}

function compareVersions(left, right) {
  for (const key of ['major', 'minor', 'patch']) {
    if (left[key] !== right[key]) {
      return left[key] - right[key];
    }
  }
  if (!left.prerelease && !right.prerelease) {
    return 0;
  }
  if (!left.prerelease) {
    return 1;
  }
  if (!right.prerelease) {
    return -1;
  }
  const length = Math.max(left.prerelease.length, right.prerelease.length);
  for (let index = 0; index < length; index++) {
    const leftIdentifier = left.prerelease[index];
    const rightIdentifier = right.prerelease[index];
    if (leftIdentifier === undefined) {
      return -1;
    }
    if (rightIdentifier === undefined) {
      return 1;
    }
    const compared = compareIdentifiers(leftIdentifier, rightIdentifier);
    if (compared !== 0) {
      return compared;
    }
  }
  return 0;
}

function safePnpmVersionFor(version) {
  const parsed = parseExactVersion(version);
  if (!parsed) {
    return undefined;
  }

  // pnpm before 10.2.0 rewrites non-semver overrides into peerDependencies,
  // causing pkg.pr.new URLs to fail peer-spec validation. Stay on the same
  // major and use the latest v10 release containing pnpm/pnpm#9000.
  if (parsed.major === 10 && compareVersions(parsed, parseExactVersion('10.2.0')) < 0) {
    return LATEST_PNPM_10_VERSION;
  }

  const pnpm11Lower = parseExactVersion('11.0.0');
  const pnpm11Upper = parseExactVersion(SAFE_PNPM_11_VERSION);
  if (compareVersions(parsed, pnpm11Lower) >= 0 && compareVersions(parsed, pnpm11Upper) < 0) {
    return SAFE_PNPM_11_VERSION;
  }

  return undefined;
}

function parsePackageManagerSpec(spec) {
  const match = /^([^@]+)@(.+)$/.exec(spec);
  return match ? { name: match[1], version: match[2] } : undefined;
}

function devEngineEntries(pkg) {
  const value = pkg.devEngines?.packageManager;
  if (Array.isArray(value)) {
    return value.filter((entry) => entry && typeof entry === 'object');
  }
  return value && typeof value === 'object' ? [value] : [];
}

function selectedDevEngineEntry(pkg) {
  return devEngineEntries(pkg).find(
    (entry) => typeof entry.name === 'string' && SUPPORTED_PACKAGE_MANAGERS.has(entry.name),
  );
}

function serializeLike(source, pkg) {
  const indentMatch = source.match(/\n([\t ]+)"/);
  const indent = indentMatch?.[1].startsWith('\t') ? '\t' : (indentMatch?.[1].length ?? 2);
  const newline = source.includes('\r\n') ? '\r\n' : '\n';
  const finalNewline = /\r?\n$/.test(source) ? newline : '';
  return JSON.stringify(pkg, null, indent).replaceAll('\n', newline) + finalNewline;
}

function replacePackageManagerSpec(source, previousSpec, targetVersion) {
  const pattern = /("packageManager"\s*:\s*)("(?:\\.|[^"\\])*")/g;
  return source.replace(pattern, (match, prefix, value) => {
    if (JSON.parse(value) !== previousSpec) {
      return match;
    }
    return `${prefix}${JSON.stringify(`pnpm@${targetVersion}`)}`;
  });
}

export function ensureSafePkgPrNewPnpmVersion(source) {
  const pkg = JSON.parse(source);
  const previousVersions = [];
  let packageManagerSpec;
  let targetVersion;
  let devEnginesChanged = false;

  if (typeof pkg.packageManager === 'string') {
    const parsed = parsePackageManagerSpec(pkg.packageManager);
    targetVersion = parsed?.name === 'pnpm' ? safePnpmVersionFor(parsed.version) : undefined;
    if (!targetVersion) {
      return { changed: false, source, previousVersions };
    }
    packageManagerSpec = pkg.packageManager;
    previousVersions.push(parsed.version);
    pkg.packageManager = `pnpm@${targetVersion}`;

    // Keep exact pnpm devEngines constraints in sync with the authoritative
    // packageManager field so the two declarations do not conflict.
    for (const entry of devEngineEntries(pkg)) {
      if (
        entry.name === 'pnpm' &&
        typeof entry.version === 'string' &&
        safePnpmVersionFor(entry.version)
      ) {
        previousVersions.push(entry.version);
        entry.version = targetVersion;
        devEnginesChanged = true;
      }
    }
  } else {
    const selected = selectedDevEngineEntry(pkg);
    targetVersion =
      selected?.name === 'pnpm' && typeof selected.version === 'string'
        ? safePnpmVersionFor(selected.version)
        : undefined;
    if (!targetVersion || selected?.name !== 'pnpm' || typeof selected.version !== 'string') {
      return { changed: false, source, previousVersions };
    }
    previousVersions.push(selected.version);
    selected.version = targetVersion;
    devEnginesChanged = true;
  }

  const updatedSource = devEnginesChanged
    ? serializeLike(source, pkg)
    : replacePackageManagerSpec(source, packageManagerSpec, targetVersion);
  return {
    changed: true,
    source: updatedSource,
    previousVersions: [...new Set(previousVersions)],
    version: targetVersion,
  };
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : undefined;
if (invokedPath === import.meta.url) {
  const packageJsonPath = process.argv[2];
  if (!packageJsonPath) {
    console.error('Usage: ensure-pkg-pr-new-pnpm-version.mjs <package.json>');
    process.exit(2);
  }
  const source = fs.readFileSync(packageJsonPath, 'utf8');
  const result = ensureSafePkgPrNewPnpmVersion(source);
  if (result.changed) {
    fs.writeFileSync(packageJsonPath, result.source);
    console.log(
      `Updating project pnpm ${result.previousVersions.join(', ')} -> ${result.version} to avoid pkg.pr.new install failures`,
    );
  }
}

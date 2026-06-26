import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const SAFE_PNPM_VERSION = '11.9.0';
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

function isAffectedPnpmVersion(version) {
  const parsed = parseExactVersion(version);
  const lower = parseExactVersion('11.0.0');
  const upper = parseExactVersion(SAFE_PNPM_VERSION);
  return (
    parsed !== undefined &&
    compareVersions(parsed, lower) >= 0 &&
    compareVersions(parsed, upper) < 0
  );
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

function replacePackageManagerSpec(source, previousSpec) {
  const pattern = /("packageManager"\s*:\s*)("(?:\\.|[^"\\])*")/g;
  return source.replace(pattern, (match, prefix, value) => {
    if (JSON.parse(value) !== previousSpec) {
      return match;
    }
    return `${prefix}${JSON.stringify(`pnpm@${SAFE_PNPM_VERSION}`)}`;
  });
}

export function ensureSafePkgPrNewPnpmVersion(source) {
  const pkg = JSON.parse(source);
  const previousVersions = [];
  let packageManagerSpec;
  let devEnginesChanged = false;

  if (typeof pkg.packageManager === 'string') {
    const parsed = parsePackageManagerSpec(pkg.packageManager);
    if (parsed?.name !== 'pnpm' || !isAffectedPnpmVersion(parsed.version)) {
      return { changed: false, source, previousVersions };
    }
    packageManagerSpec = pkg.packageManager;
    previousVersions.push(parsed.version);
    pkg.packageManager = `pnpm@${SAFE_PNPM_VERSION}`;

    // Keep exact pnpm devEngines constraints in sync with the authoritative
    // packageManager field so the two declarations do not conflict.
    for (const entry of devEngineEntries(pkg)) {
      if (
        entry.name === 'pnpm' &&
        typeof entry.version === 'string' &&
        isAffectedPnpmVersion(entry.version)
      ) {
        previousVersions.push(entry.version);
        entry.version = SAFE_PNPM_VERSION;
        devEnginesChanged = true;
      }
    }
  } else {
    const selected = selectedDevEngineEntry(pkg);
    if (
      selected?.name !== 'pnpm' ||
      typeof selected.version !== 'string' ||
      !isAffectedPnpmVersion(selected.version)
    ) {
      return { changed: false, source, previousVersions };
    }
    previousVersions.push(selected.version);
    selected.version = SAFE_PNPM_VERSION;
    devEnginesChanged = true;
  }

  const updatedSource = devEnginesChanged
    ? serializeLike(source, pkg)
    : replacePackageManagerSpec(source, packageManagerSpec);
  return {
    changed: true,
    source: updatedSource,
    previousVersions: [...new Set(previousVersions)],
    version: SAFE_PNPM_VERSION,
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
      `Updating project pnpm ${result.previousVersions.join(', ')} -> ${result.version} to avoid pkg.pr.new tarball integrity failures`,
    );
  }
}

import fs from 'node:fs';
import path from 'node:path';

import { parse as parseJsonc, type ParseError } from 'jsonc-parser';
import semver from 'semver';
import { parseDocument } from 'yaml';

import { PackageManager } from '../../types/index.ts';

function record(value: unknown): Record<string, unknown> | undefined {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : undefined;
}

function declaredVitest(pkg: unknown): unknown {
  const manifest = record(pkg);
  for (const field of [
    'dependencies',
    'devDependencies',
    'optionalDependencies',
    'peerDependencies',
  ]) {
    const spec = record(manifest?.[field])?.vitest;
    if (typeof spec === 'string') {
      return spec;
    }
  }
  return undefined;
}

/** Read a direct upstream Vitest edge without installing or changing a lockfile.
 * Unsupported protocols, ambiguous layouts and stale entries remain unknown.
 * In particular, never infer a runner from an unrelated transitive package. */
export function lockedVitestVersion(
  root: string,
  directory: string,
  manager: PackageManager,
  spec: string,
): string | undefined {
  const range = spec.replace(/^npm:vitest@/, '');
  const validRange = semver.validRange(range);
  if (!validRange && !/^[\w.-]+$/.test(range)) {
    return undefined;
  }
  function valid(version: unknown): string | undefined {
    if (typeof version !== 'string' || !semver.valid(version)) {
      return undefined;
    }
    return !validRange || semver.satisfies(version, range, { includePrerelease: true })
      ? version
      : undefined;
  }
  const relative = path.relative(root, directory).replaceAll('\\', '/');
  const read = (name: string) =>
    fs.readFileSync(path.join(root, name), 'utf8').replace(/^\uFEFF/, '');
  try {
    if (manager === PackageManager.npm) {
      const lock = JSON.parse(
        read(
          fs.existsSync(path.join(root, 'npm-shrinkwrap.json'))
            ? 'npm-shrinkwrap.json'
            : 'package-lock.json',
        ),
      );
      const packages = record(lock.packages);
      if (!packages) {
        // npm v1 locks have no importer specifiers to prove this direct edge.
        return undefined;
      }
      if (declaredVitest(packages[relative]) !== spec) {
        return undefined;
      }
      let owner = relative;
      while (true) {
        const entry = record(packages[path.posix.join(owner, 'node_modules/vitest')]);
        if (entry) {
          return !entry.link && (!entry.name || entry.name === 'vitest')
            ? valid(entry.version)
            : undefined;
        }
        if (!owner) {
          return undefined;
        }
        const parent = path.posix.dirname(owner);
        owner = parent === '.' ? '' : parent;
      }
    }
    if (manager === PackageManager.yarn) {
      const text = read('yarn.lock');
      if (/^__metadata:/m.test(text)) {
        const document = parseDocument(text);
        if (document.errors.length) {
          return undefined;
        }
        const descriptors = new Set([`vitest@${spec}`, `vitest@npm:${range}`]);
        const entries = Object.entries(record(document.toJS()) ?? {}).filter(([keys]) =>
          keys.split(/,\s+/).some((key) => descriptors.has(key)),
        );
        if (entries.length !== 1) {
          return undefined;
        }
        const entry = record(entries[0][1]);
        return typeof entry?.resolution === 'string' && entry.resolution.startsWith('vitest@npm:')
          ? valid(entry.resolution.slice('vitest@npm:'.length))
          : undefined;
      }
      // Classic's quoted descriptor lists are YAML flow scalars, but its entry
      // bodies are not YAML. Read only the exact descriptor and version field.
      const matches: string[] = [];
      for (const block of text.split(/\r?\n(?=[^\s#])/)) {
        const [header] = block.split(/\r?\n/);
        if (!header.endsWith(':')) {
          continue;
        }
        const document = parseDocument(`[${header.slice(0, -1)}]`);
        const keys: unknown = document.errors.length ? undefined : document.toJS();
        if (Array.isArray(keys) && keys.includes(`vitest@${spec}`)) {
          const version = /^  version "([^"]+)"\r?$/m.exec(block)?.[1];
          if (!version) {
            return undefined;
          }
          matches.push(version);
        }
      }
      return matches.length === 1 ? valid(matches[0]) : undefined;
    }
    if (manager === PackageManager.bun) {
      const errors: ParseError[] = [];
      const lock = record(parseJsonc(read('bun.lock'), errors, { allowTrailingComma: true }));
      if (errors.length || declaredVitest(record(lock?.workspaces)?.[relative]) !== spec) {
        return undefined;
      }
      const packages = record(lock?.packages);
      const direct = packages?.vitest;
      if (
        !Array.isArray(direct) ||
        typeof direct[0] !== 'string' ||
        !direct[0].startsWith('vitest@')
      ) {
        return undefined;
      }
      const directVersion = valid(direct[0].slice('vitest@'.length));
      if (!directVersion) {
        return undefined;
      }
      // Bun can encode multiple nested/isolated package layouts. Only accept a
      // hoisted edge with a unique matching upstream version; otherwise require
      // the original install. A lone unrelated nested entry is not evidence.
      const versions = new Set<string>();
      for (const entry of Object.values(packages ?? {})) {
        if (
          !Array.isArray(entry) ||
          typeof entry[0] !== 'string' ||
          !entry[0].startsWith('vitest@')
        ) {
          continue;
        }
        const version = valid(entry[0].slice('vitest@'.length));
        if (version) {
          versions.add(version);
        }
      }
      return versions.size === 1 ? directVersion : undefined;
    }
  } catch {
    // Missing, malformed or unsupported locks are not proof of a runner version.
  }
  return undefined;
}

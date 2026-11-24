import fs from 'node:fs';
import path from 'node:path';

import type { ParseError } from 'jsonc-parser';
import { parse as parseJsonc } from 'jsonc-parser';
import { normalize, normalizeRange, satisfies } from 'verkit';
import { parseDocument } from 'yaml';

import { PackageManager } from '../../types/index.ts';
import type { YamlMapping } from '../../utils/yaml-value.ts';
import { isYamlMapping } from '../../utils/yaml-value.ts';

function record(value: unknown): YamlMapping | undefined {
  return isYamlMapping(value) ? value : undefined;
}

function cacheParsedFiles<T>() {
  const locks = new Map<string, T | undefined>();
  return (file: string, parse: (source: string) => T) => {
    if (!locks.has(file)) {
      // Cache failed reads and parses too, until the next preflight.
      locks.set(file, undefined);
      try {
        locks.set(file, parse(fs.readFileSync(file, 'utf8').replace(/^\uFEFF/, '')));
      } catch {
        // Missing or malformed locks are not evidence of a runner version.
      }
    }
    return locks.get(file);
  };
}

/** Cache parsed manifests and Yarn's different lock format for one preflight. */
export function createLockfileReader() {
  return {
    manifest: cacheParsedFiles<YamlMapping | undefined>(),
    yarn: cacheParsedFiles<ReturnType<typeof parseYarnLockfile>>(),
  };
}
export type LockfileReader = ReturnType<typeof createLockfileReader>;

function parseYarnLockfile(text: string) {
  if (/^__metadata:/m.test(text)) {
    const document = parseDocument(text);
    if (document.errors.length) {
      return undefined;
    }
    return { modern: true as const, entries: record(document.toJS()) ?? {} };
  }
  // Classic's quoted descriptor lists are YAML flow scalars, but its entry
  // bodies are not YAML. Read only the exact descriptor and version field.
  const entries: Array<{ keys: Array<unknown>; version: string | undefined }> = [];
  for (const block of text.split(/\r?\n(?=[^\s#])/)) {
    const [header] = block.split(/\r?\n/);
    if (!header.endsWith(':')) {
      continue;
    }
    const document = parseDocument(`[${header.slice(0, -1)}]`);
    const keys: unknown = document.errors.length ? undefined : document.toJS();
    if (Array.isArray(keys)) {
      entries.push({ keys, version: /^ {2}version "([^"]+)"\r?$/m.exec(block)?.[1] });
    }
  }
  return { modern: false as const, entries };
}

function declaredVitest(pkg: unknown): string | undefined {
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
  readLockfile: LockfileReader,
): string | undefined {
  const range = spec.replace(/^npm:vitest@/, '');
  const validRange = normalizeRange(range);
  if (!validRange && !/^[\w.-]+$/.test(range)) {
    return undefined;
  }
  function valid(version: unknown): string | undefined {
    if (typeof version !== 'string' || !normalize(version)) {
      return undefined;
    }
    return !validRange || satisfies(version, range, { includePrerelease: true })
      ? version
      : undefined;
  }
  const relative = path.relative(root, directory).replaceAll('\\', '/');
  try {
    if (manager === PackageManager.npm) {
      const lock = readLockfile.manifest(
        path.join(
          root,
          fs.existsSync(path.join(root, 'npm-shrinkwrap.json'))
            ? 'npm-shrinkwrap.json'
            : 'package-lock.json',
        ),
        (source) => record(JSON.parse(source)),
      );
      const packages = record(lock?.packages);
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
      const lock = readLockfile.yarn(path.join(root, 'yarn.lock'), parseYarnLockfile);
      if (!lock) {
        return undefined;
      }
      if (lock.modern) {
        const descriptors = new Set([`vitest@${spec}`, `vitest@npm:${range}`]);
        const entries = Object.entries(lock.entries).filter(([keys]) =>
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
      const matches: Array<string> = [];
      for (const { keys, version } of lock.entries) {
        if (keys.includes(`vitest@${spec}`)) {
          if (!version) {
            return undefined;
          }
          matches.push(version);
        }
      }
      return matches.length === 1 ? valid(matches[0]) : undefined;
    }
    if (manager === PackageManager.bun) {
      const lock = readLockfile.manifest(path.join(root, 'bun.lock'), (source) => {
        const errors: Array<ParseError> = [];
        const parsed = record(parseJsonc(source, errors, { allowTrailingComma: true }));
        return errors.length ? undefined : parsed;
      });
      if (declaredVitest(record(lock?.workspaces)?.[relative]) !== spec) {
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

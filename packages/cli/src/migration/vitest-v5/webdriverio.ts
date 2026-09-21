import fs from 'node:fs';
import path from 'node:path';

import { applyEdits, modify, parse } from 'jsonc-parser';
import semver from 'semver';
import { parseDocument } from 'yaml';

import { PackageManager, type WorkspacePackage } from '../../types/index.ts';
import { VITE_PLUS_OVERRIDE_PACKAGES } from '../../utils/constants.ts';
import { extractOverrideTargetName } from '../../utils/package-overrides.ts';
import { detectPackageMetadata } from '../../utils/package.ts';
import { extractOverrideParentSegments, usesWebdriverioProvider } from '../migrator.ts';
import { WEBDRIVERIO_PROVIDER } from '../migrator/shared.ts';
import type { VitestV5Finding } from './ast.ts';

// This is a migration floor, not a lockstep Vitest version. The community
// provider has its own releases; never downgrade or repin a newer user version.
// https://vitest.dev/guide/migration/#package-migration
const MIN_VERSION = '5.0.0';
const DEFAULT_SPEC = '^5.0.0';
const REGISTRY_ALIAS = `npm:${WEBDRIVERIO_PROVIDER}@`;
const INSTALL_FIELDS = ['dependencies', 'devDependencies', 'optionalDependencies'] as const;

/** Narrow mixed ranges without discarding the user's newer allowed versions. */
export function webdriverioMigrationSpec(spec: string): string | undefined {
  if (spec.startsWith(REGISTRY_ALIAS)) {
    const migrated = webdriverioMigrationSpec(spec.slice(REGISTRY_ALIAS.length));
    return migrated === undefined ? undefined : `${REGISTRY_ALIAS}${migrated}`;
  }
  if (!semver.validRange(spec)) {
    return undefined;
  }
  const range = new semver.Range(spec);
  const branches = range.set.map((set) => set.map((item) => item.value).join(' '));
  if (
    branches.every((branch) => {
      const min = semver.minVersion(branch);
      return min && semver.gte(min, MIN_VERSION);
    })
  ) {
    return spec;
  }
  const narrowed = branches
    .map((branch) => `${branch} >=${MIN_VERSION}`.trim())
    .filter((branch) => semver.minVersion(branch) !== null);
  return narrowed.length ? narrowed.join(' || ') : DEFAULT_SPEC;
}

function object(value: unknown): Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

/** Compose dependency edits into the versioned preflight's in-memory sources.
 * No writes or network resolution: unresolved custom specs block before install. */
export function migrateWebdriverioDependencies(
  workspace: { rootDir: string; packageManager?: PackageManager; packages?: WorkspacePackage[] },
  sources: Map<string, string>,
  inputs: Map<string, string | null>,
): VitestV5Finding[] {
  const findings: VitestV5Finding[] = [];
  const rootManifest = path.join(workspace.rootDir, 'package.json');
  const manifests = [
    rootManifest,
    ...(workspace.packages ?? []).map((pkg) =>
      path.resolve(workspace.rootDir, pkg.path, 'package.json'),
    ),
  ];
  const read = (file: string): Record<string, unknown> => {
    const source = sources.get(file);
    if (source === undefined) {
      return {};
    }
    return object(file.endsWith('.json') ? parse(source) : parseDocument(source).toJS());
  };
  const get = (file: string, keys: string[]): unknown =>
    keys.reduce<unknown>((value, key) => object(value)[key], read(file));
  const set = (file: string, keys: string[], value: string | undefined) => {
    if (get(file, keys) === value) {
      return;
    }
    const source = sources.get(file)!;
    if (file.endsWith('.json')) {
      sources.set(
        file,
        applyEdits(
          source,
          modify(source, keys, value, {
            formattingOptions: { insertSpaces: true, tabSize: 2 },
          }),
        ),
      );
    } else {
      const doc = parseDocument(source);
      if (value === undefined) {
        doc.deleteIn(keys);
      } else {
        doc.setIn(keys, value);
      }
      sources.set(file, doc.toString());
    }
  };
  const block = (file: string, label: string, spec: string) => {
    findings.push({
      file,
      line: 1,
      column: 1,
      code: 'browser-provider',
      severity: 'block',
      message: `Cannot ensure ${label} (${spec}) uses ${WEBDRIVERIO_PROVIDER} >=${MIN_VERSION}. Select a compatible provider version, then re-run migration.`,
    });
  };
  const catalogEntry = (
    spec: string,
    name: string,
  ): { file: string; keys: string[] } | undefined => {
    const catalog = spec.slice('catalog:'.length);
    const keys = catalog && catalog !== 'default' ? ['catalogs', catalog, name] : ['catalog', name];
    if (workspace.packageManager === PackageManager.bun) {
      for (const prefix of [['workspaces'], []]) {
        if (typeof get(rootManifest, [...prefix, ...keys]) === 'string') {
          return { file: rootManifest, keys: [...prefix, ...keys] };
        }
      }
      return undefined;
    }
    const file = path.join(
      workspace.rootDir,
      workspace.packageManager === PackageManager.yarn ? '.yarnrc.yml' : 'pnpm-workspace.yaml',
    );
    return typeof get(file, keys) === 'string' ? { file, keys } : undefined;
  };
  const update = (file: string, keys: string[], directory: string, seen = new Set<string>()) => {
    const spec = get(file, keys);
    if (typeof spec !== 'string') {
      return;
    }
    const location = `${file}:${keys.join('.')}`;
    if (seen.has(location)) {
      block(file, keys.join('.'), spec);
      return;
    }
    seen.add(location);
    if (spec.startsWith('catalog:')) {
      const entry = catalogEntry(spec, WEBDRIVERIO_PROVIDER);
      if (entry) {
        update(entry.file, entry.keys, directory, seen);
      } else {
        block(file, keys.join('.'), spec);
      }
      return;
    }
    if (spec.startsWith('$')) {
      const name = spec.slice(1);
      const field = INSTALL_FIELDS.find(
        (field) => typeof get(rootManifest, [field, name]) === 'string',
      );
      if (field && name === WEBDRIVERIO_PROVIDER) {
        update(rootManifest, [field, name], workspace.rootDir, seen);
      } else {
        block(file, keys.join('.'), spec);
      }
      return;
    }
    const migrated = webdriverioMigrationSpec(spec);
    if (migrated !== undefined) {
      set(file, keys, migrated);
      return;
    }
    // Installed metadata is useful for local/workspace sources, whose version
    // can be checked without replacing a user's fork with a registry package.
    // A tag or Git branch is mutable: an unrelated old install cannot prove it.
    if (/^(?:file:|link:|workspace:)/.test(spec)) {
      const local = spec.startsWith('workspace:')
        ? manifests.find((manifest) => read(manifest).name === WEBDRIVERIO_PROVIDER)
        : path.resolve(directory, spec.slice(spec.indexOf(':') + 1), 'package.json');
      if (local && fs.existsSync(local)) {
        const text = fs.readFileSync(local, 'utf8');
        inputs.set(local, text);
        const pkg = object(parse(text));
        if (
          pkg.name === WEBDRIVERIO_PROVIDER &&
          typeof pkg.version === 'string' &&
          semver.valid(pkg.version) &&
          semver.gte(pkg.version, MIN_VERSION)
        ) {
          return;
        }
      }
    }
    block(file, keys.join('.'), spec);
  };

  const selected = manifests.filter((file) => {
    const pkg = read(file);
    return (
      [...INSTALL_FIELDS, 'peerDependencies'].some(
        (field) => typeof object(pkg[field])[WEBDRIVERIO_PROVIDER] === 'string',
      ) || usesWebdriverioProvider(path.dirname(file))
    );
  });
  if (!selected.length) {
    return findings;
  }

  for (const file of selected) {
    const directory = path.dirname(file);
    let installed = false;
    for (const field of INSTALL_FIELDS) {
      if (typeof get(file, [field, WEBDRIVERIO_PROVIDER]) === 'string') {
        update(file, [field, WEBDRIVERIO_PROVIDER], directory);
        installed = true;
      }
    }
    if (!installed) {
      // A peer range is a public consumer contract, not a project install.
      // Add a dev dependency without narrowing that public range.
      const peer = get(file, ['peerDependencies', WEBDRIVERIO_PROVIDER]);
      const inherited =
        file !== rootManifest
          ? INSTALL_FIELDS.map((field) => get(rootManifest, [field, WEBDRIVERIO_PROVIDER])).find(
              (spec): spec is string => typeof spec === 'string',
            )
          : undefined;
      let spec = typeof peer === 'string' ? peer : inherited;
      if (spec?.startsWith('catalog:') && typeof peer === 'string') {
        const entry = catalogEntry(spec, WEBDRIVERIO_PROVIDER);
        const resolved = entry && get(entry.file, entry.keys);
        // Do not change a shared catalog that still describes the public peer.
        spec = typeof resolved === 'string' ? resolved : spec;
      }
      set(file, ['devDependencies', WEBDRIVERIO_PROVIDER], spec ?? DEFAULT_SPEC);
      update(file, ['devDependencies', WEBDRIVERIO_PROVIDER], directory);
    }
    if (!INSTALL_FIELDS.some((field) => typeof get(file, [field, 'webdriverio']) === 'string')) {
      let peer = '*';
      const pkg = read(file);
      for (const name of ['webdriverio', '@wdio/cli', '@wdio/globals']) {
        const spec = [...INSTALL_FIELDS, 'peerDependencies']
          .map((field) => object(pkg[field])[name])
          .find((spec): spec is string => typeof spec === 'string');
        if (!spec) {
          continue;
        }
        const entry = spec.startsWith('catalog:') ? catalogEntry(spec, name) : undefined;
        const resolved = entry ? get(entry.file, entry.keys) : spec;
        if (typeof resolved === 'string' && semver.validRange(resolved)) {
          peer = name === 'webdriverio' ? spec : resolved;
          break;
        }
      }
      // An existing installed framework is another safe fallback, e.g. a
      // WebDriverIO CLI that brings its framework transitively.
      const metadata = peer === '*' ? detectPackageMetadata(directory, 'webdriverio') : undefined;
      if (metadata && semver.valid(metadata.version)) {
        const metadataFile = path.join(metadata.path, 'package.json');
        inputs.set(metadataFile, fs.readFileSync(metadataFile, 'utf8'));
        peer = `^${metadata.version}`;
      }
      set(file, ['devDependencies', 'webdriverio'], peer);
    }
    // npm needs the provider's Vite peer reachable independently of Vitest's
    // overridden subtree. Keep the same installation support as official providers.
    if (
      workspace.packageManager === PackageManager.npm &&
      VITE_PLUS_OVERRIDE_PACKAGES.vite &&
      !INSTALL_FIELDS.some((field) => get(file, [field, 'vite']) !== undefined)
    ) {
      set(file, ['devDependencies', 'vite'], VITE_PLUS_OVERRIDE_PACKAGES.vite);
    }
  }

  const projectNames = selected
    .map((file) => read(file).name)
    .filter((name): name is string => typeof name === 'string');
  function matchesProject(parents: string[]): boolean {
    const concrete = parents.filter((parent) => parent !== '**');
    if (concrete.length === 0) {
      return true;
    }
    if (concrete.length !== 1) {
      return false;
    }
    const pattern = concrete[0]
      .split('*')
      .map((part) => part.replace(/[.+?^${}()|[\]\\]/g, '\\$&'))
      .join('.*');
    const expression = new RegExp(`^${pattern}$`);
    return projectNames.some((name) => expression.test(name));
  }
  const overrideMap = (file: string, keys: string[], parents: string[] = []) => {
    for (const [key, value] of Object.entries(object(get(file, keys)))) {
      const target = extractOverrideTargetName(key);
      const ancestors = [...parents, ...(extractOverrideParentSegments(key) ?? [])];
      const entry = [...keys, key];
      if (target === WEBDRIVERIO_PROVIDER && matchesProject(ancestors)) {
        const versionPath = typeof value === 'string' ? entry : [...entry, '.'];
        const before = get(file, versionPath);
        const catalog =
          typeof before === 'string' && before.startsWith('catalog:')
            ? catalogEntry(before, WEBDRIVERIO_PROVIDER)
            : undefined;
        const pin = catalog ? get(catalog.file, catalog.keys) : before;
        const pinnedRange =
          typeof pin === 'string'
            ? semver.validRange(
                pin.startsWith(REGISTRY_ALIAS) ? pin.slice(REGISTRY_ALIAS.length) : pin,
              )
            : null;
        // An old forcing pin must not override a newer direct declaration.
        // Remove v4-only pins and let the migrated dependency choose its version.
        // Preserve the children of npm's long-form override objects.
        if (pinnedRange && !semver.intersects(pinnedRange, `>=${MIN_VERSION}`)) {
          set(file, versionPath, undefined);
          continue;
        }
        update(file, versionPath, workspace.rootDir);
        // npm requires a direct dependency override to use the same spec.
        // A $ reference preserves a valid existing pin without EOVERRIDE.
        if (
          workspace.packageManager === PackageManager.npm &&
          keys[0] === 'overrides' &&
          ancestors.length === 0 &&
          selected.includes(rootManifest) &&
          typeof before === 'string'
        ) {
          const after = get(file, versionPath);
          const field = INSTALL_FIELDS.find(
            (field) => get(rootManifest, [field, WEBDRIVERIO_PROVIDER]) !== undefined,
          )!;
          if (
            typeof after === 'string' &&
            !after.startsWith('$') &&
            webdriverioMigrationSpec(after) === after
          ) {
            const direct = get(rootManifest, [field, WEBDRIVERIO_PROVIDER]);
            if (
              typeof direct === 'string' &&
              webdriverioMigrationSpec(before) !== before &&
              webdriverioMigrationSpec(direct) === direct
            ) {
              set(file, versionPath, `$${WEBDRIVERIO_PROVIDER}`);
            } else if (direct === after) {
              continue;
            } else if (
              typeof direct === 'string' &&
              semver.validRange(direct) &&
              semver.validRange(after) &&
              semver.intersects(direct, after)
            ) {
              set(rootManifest, [field, WEBDRIVERIO_PROVIDER], after);
              set(file, versionPath, `$${WEBDRIVERIO_PROVIDER}`);
            } else {
              block(file, versionPath.join('.'), after);
            }
          }
        }
      }
      if (value !== null && typeof value === 'object') {
        overrideMap(file, entry, [...ancestors, target]);
      }
    }
  };
  overrideMap(rootManifest, ['overrides']);
  overrideMap(rootManifest, ['resolutions']);
  overrideMap(rootManifest, ['pnpm', 'overrides']);
  overrideMap(path.join(workspace.rootDir, 'pnpm-workspace.yaml'), ['overrides']);
  return findings;
}

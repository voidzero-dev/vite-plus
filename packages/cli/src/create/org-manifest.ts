import path from 'node:path';

import { normalize } from 'verkit';

import type { JsonObject } from '../utils/json-value.ts';
import { isJsonObject, isObjectValue, isStringRecord } from '../utils/json-value.ts';
import { fetchNpmResource, getNpmRegistry } from '../utils/npm-config.ts';
import { readPackageJsonFromTarball } from './org-tarball.ts';

/**
 * A single template entry shared by org manifests (`createConfig.templates`)
 * and local monorepo config (`create.templates` in `vite.config.ts`).
 */
export interface CreateTemplateEntry {
  name: string;
  description: string;
  template: string;
}

/**
 * A single entry in an org's template manifest. Extends the shared
 * {@link CreateTemplateEntry} with the org-only `monorepo` flag.
 */
export interface OrgTemplateEntry extends CreateTemplateEntry {
  monorepo?: boolean;
}

/**
 * The resolved manifest for an `@scope/create` package — the subset of the
 * registry response that the create flow actually needs.
 */
export interface OrgManifest {
  scope: string;
  packageName: string;
  version: string;
  tarballUrl: string;
  integrity?: string;
  templates: Array<OrgTemplateEntry>;
}

/**
 * Parse the org picker specifier: `@scope` (scope only → picker) or
 * `@scope:name` (direct manifest-entry selection). Colon mirrors the
 * existing `vite:monorepo` / `vite:library` builtin-template syntax and
 * keeps manifest entries syntactically distinct from real
 * `@scope/package-name` npm specifiers.
 *
 * Returns `null` for anything else — including the plain `@scope/name`
 * form, which routes to the existing `@scope/create-name` shorthand as
 * it did before the org-manifest feature.
 *
 * The optional `version` suffix (`@scope@1.2.3`, `@scope:name@1.2.3`)
 * pins `@scope/create` to a specific release rather than `dist-tags.latest`.
 */
export function parseOrgScopedSpec(
  spec: string,
): { scope: string; name?: string; version?: string } | null {
  if (!spec.startsWith('@')) {
    return null;
  }
  // Reject `@scope/anything` — let that form fall through to the
  // pre-feature shorthand path in `expandCreateShorthand`.
  if (spec.includes('/')) {
    return null;
  }
  const colonIndex = spec.indexOf(':');
  if (colonIndex === -1) {
    // `@scope` or `@scope@version` → scope-only picker.
    const atIndex = spec.indexOf('@', 1);
    if (atIndex === -1) {
      return { scope: spec };
    }
    const version = spec.slice(atIndex + 1);
    return version ? { scope: spec.slice(0, atIndex), version } : { scope: spec.slice(0, atIndex) };
  }
  const scope = spec.slice(0, colonIndex);
  const rest = spec.slice(colonIndex + 1);
  // `@scope:name@version` — split out the optional version suffix.
  const atIndex = rest.indexOf('@');
  const name = atIndex === -1 ? rest : rest.slice(0, atIndex);
  const version = atIndex === -1 ? '' : rest.slice(atIndex + 1);
  if (!name) {
    return version ? { scope, version } : { scope };
  }
  return version ? { scope, name, version } : { scope, name };
}

/**
 * Schema-level failure. Never falls through silently — a maintainer who
 * shipped an invalid manifest should see the offending field.
 */
export class OrgManifestSchemaError extends Error {
  constructor(
    message: string,
    public readonly packageName: string,
  ) {
    super(`${packageName}: ${message}`);
    this.name = 'OrgManifestSchemaError';
  }
}

export function isRelativePath(spec: string): boolean {
  return spec.startsWith('./') || spec.startsWith('../');
}

/**
 * Validate the `{ name, description, template }` fields shared by org manifest
 * entries and local `create.templates` entries. `label` is the config path
 * used in error messages (e.g. `createConfig.templates` or `create.templates`)
 * and `makeError` builds the thrown error so each source uses its own type.
 */
export function validateTemplateEntry(
  entry: unknown,
  index: number,
  label: string,
  makeError: (message: string) => Error,
): CreateTemplateEntry {
  if (!entry || typeof entry !== 'object') {
    throw makeError(`${label}[${index}] must be an object`);
  }
  const requireString = (field: string, value: unknown): string => {
    if (typeof value !== 'string' || value.length === 0) {
      throw makeError(`${label}[${index}].${field} must be a non-empty string`);
    }
    return value;
  };
  const name = requireString('name', 'name' in entry ? entry.name : undefined);
  // `__vp_` is reserved for internal sentinel values (e.g. the
  // org-picker's escape-hatch nonce in `org-picker.ts`). Reject the
  // prefix at schema time so an entry can never collide with those
  // sentinels regardless of what the picker does internally.
  if (name.startsWith('__vp_')) {
    throw makeError(`${label}[${index}].name uses the reserved \`__vp_\` prefix`);
  }
  const description = requireString(
    'description',
    'description' in entry ? entry.description : undefined,
  );
  const template = requireString('template', 'template' in entry ? entry.template : undefined);

  if (isRelativePath(template)) {
    // Defense-in-depth only: `resolveBundledPath` enforces the authoritative
    // check after extraction. We reject obvious root-escapes here so schema
    // errors surface before any tarball download happens.
    const resolved = path.posix.resolve('/root', template.replaceAll('\\', '/'));
    if (resolved !== '/root' && !resolved.startsWith('/root/')) {
      throw makeError(`${label}[${index}].template escapes the package root: ${template}`);
    }
  }

  return { name, description, template };
}

/**
 * Validate a list of entries, rejecting duplicate `name`s. Shared by org
 * manifests and local `create.templates`.
 */
export function validateTemplateEntries<T extends CreateTemplateEntry>(
  templates: ReadonlyArray<unknown>,
  label: string,
  makeError: (message: string) => Error,
  validateOne: (entry: unknown, index: number) => T,
): Array<T> {
  const entries: Array<T> = [];
  const seen = new Set<string>();
  for (let index = 0; index < templates.length; index += 1) {
    const entry = validateOne(templates[index], index);
    if (seen.has(entry.name)) {
      throw makeError(`${label}[${index}].name duplicates an earlier entry: "${entry.name}"`);
    }
    seen.add(entry.name);
    entries.push(entry);
  }
  return entries;
}

function validateEntry(entry: unknown, index: number, packageName: string): OrgTemplateEntry {
  const makeError = (message: string) => new OrgManifestSchemaError(message, packageName);
  const base = validateTemplateEntry(entry, index, 'createConfig.templates', makeError);

  if (entry && typeof entry === 'object' && 'monorepo' in entry && entry.monorepo !== undefined) {
    if (typeof entry.monorepo !== 'boolean') {
      throw makeError(`createConfig.templates[${index}].monorepo must be a boolean`);
    }
    return { ...base, monorepo: entry.monorepo };
  }
  return base;
}

function validateManifest(raw: unknown, packageName: string): Array<OrgTemplateEntry> | null {
  if (!raw || typeof raw !== 'object') {
    return null;
  }
  const createConfig = 'createConfig' in raw ? raw.createConfig : undefined;
  if (!createConfig || typeof createConfig !== 'object') {
    return null;
  }
  const templates = 'templates' in createConfig ? createConfig.templates : undefined;
  if (templates === undefined) {
    return null;
  }
  if (!Array.isArray(templates)) {
    throw new OrgManifestSchemaError('createConfig.templates must be an array', packageName);
  }
  if (templates.length === 0) {
    // Treat empty array as "no manifest" — fall through to normal @org/create behavior.
    return null;
  }
  return validateTemplateEntries(
    templates,
    'createConfig.templates',
    (message) => new OrgManifestSchemaError(message, packageName),
    (entry, index) => validateEntry(entry, index, packageName),
  );
}

/**
 * Schema-level failure for `create.templates` in `vite.config.ts`. A misconfigured
 * local template should surface clearly rather than silently disappear.
 */
export class CreateConfigSchemaError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'CreateConfigSchemaError';
  }
}

const makeCreateConfigError = (message: string) => new CreateConfigSchemaError(message);

/**
 * Validate `create.templates` from `vite.config.ts`. Returns `[]` when the field
 * is absent or an empty array; throws {@link CreateConfigSchemaError} when present
 * but malformed.
 */
export function validateCreateTemplates(templates: unknown): Array<CreateTemplateEntry> {
  if (templates === undefined) {
    return [];
  }
  if (!Array.isArray(templates)) {
    throw new CreateConfigSchemaError('create.templates must be an array');
  }
  return validateTemplateEntries(
    templates,
    'create.templates',
    makeCreateConfigError,
    (entry, index) => {
      const validated = validateTemplateEntry(
        entry,
        index,
        'create.templates',
        makeCreateConfigError,
      );
      // `vite:*` names are builtin templates; a local entry resolves before the
      // builtin in `vp create <name>`, so allowing the prefix would let config
      // silently shadow e.g. `vite:application`.
      if (validated.name.startsWith('vite:')) {
        throw makeCreateConfigError(
          `create.templates[${index}].name uses the reserved \`vite:\` prefix`,
        );
      }
      return validated;
    },
  );
}

async function fetchPackument(scope: string, packageName: string): Promise<JsonObject | null> {
  // npm's registry URLs keep `@` and `/` unencoded
  // (`https://registry.npmjs.org/@scope/name`). Match that — private
  // registries often route on the literal path.
  const url = `${getNpmRegistry(scope)}/${packageName}`;
  const response = await fetchNpmResource(url, {
    headers: { accept: 'application/json' },
    timeoutMs: 5000,
  });
  if (response.status === 404) {
    return null;
  }
  if (!response.ok) {
    throw new Error(`npm registry responded with ${response.status} for ${packageName}`);
  }
  const value: unknown = await response.json();
  return isJsonObject(value) ? value : null;
}

/**
 * Fetch `@scope/create` from the npm registry and parse its `createConfig.templates`
 * manifest.
 *
 * Returns `null` when:
 * - the package does not exist on the registry (404), or
 * - the package exists but has no `createConfig.templates` field
 *
 * When the packument version metadata lacks `createConfig` entirely, the
 * published tarball's package.json is consulted before giving up — some
 * registries (GitHub Packages among them) strip custom fields from version
 * metadata while preserving the tarball byte-for-byte.
 *
 * Throws when:
 * - the `createConfig.templates` field is present but malformed (`OrgManifestSchemaError`), or
 * - the registry request fails for any non-404 reason
 *
 * `requestedVersion` pins the lookup to a specific `versions[...]` entry
 * (equivalent to `vp create @scope@1.2.3`); omit it to resolve `dist-tags.latest`.
 */
export async function readOrgManifest(
  scope: string,
  requestedVersion?: string,
): Promise<OrgManifest | null> {
  if (!scope.startsWith('@')) {
    return null;
  }
  const packageName = `${scope}/create`;
  const packument = await fetchPackument(scope, packageName);
  if (!packument) {
    return null;
  }
  const tags = isStringRecord(packument['dist-tags']) ? packument['dist-tags'] : undefined;
  const versions = isObjectValue(packument.versions) ? packument.versions : undefined;
  let resolvedVersion: string | undefined;
  if (requestedVersion) {
    resolvedVersion =
      tags?.[requestedVersion] ?? (versions?.[requestedVersion] ? requestedVersion : undefined);
    if (!resolvedVersion) {
      throw new OrgManifestSchemaError(
        `version "${requestedVersion}" not found (known tags: ${Object.keys(tags ?? {}).join(', ') || 'none'})`,
        packageName,
      );
    }
  } else {
    resolvedVersion = tags?.latest;
    if (!resolvedVersion) {
      return null;
    }
  }
  // Registry versions become cache-path components, so reject malformed
  // values even when the registry has matching version metadata.
  if (normalize(resolvedVersion) === null) {
    throw new OrgManifestSchemaError(
      `invalid version "${resolvedVersion}" (expected a semantic version)`,
      packageName,
    );
  }
  const meta = versions?.[resolvedVersion];
  if (!isObjectValue(meta)) {
    return null;
  }
  const dist = isObjectValue(meta.dist) ? meta.dist : undefined;
  const tarball = typeof dist?.tarball === 'string' ? dist.tarball : undefined;
  const integrity = typeof dist?.integrity === 'string' ? dist.integrity : undefined;
  let templates = validateManifest(meta, packageName);
  if (!templates && meta.createConfig === undefined && tarball) {
    // `createConfig` absent (not merely empty) means the registry either
    // stripped it from the version metadata or the package has no manifest;
    // probe the published tarball to tell the two apart (see
    // `readPackageJsonFromTarball`). The probe is best-effort: any
    // download/integrity/parse failure degrades to "no manifest" so a normal
    // `@scope/create` package still reaches the passthrough path. A manifest
    // that IS present but malformed still throws, because `validateManifest`
    // runs outside the catch.
    const packageJson = await readPackageJsonFromTarball(tarball, integrity).catch(() => null);
    templates = validateManifest(packageJson, packageName);
  }
  if (!templates) {
    return null;
  }
  if (!tarball) {
    throw new OrgManifestSchemaError(`missing dist.tarball for ${resolvedVersion}`, packageName);
  }
  return {
    scope,
    packageName,
    version: resolvedVersion,
    tarballUrl: tarball,
    integrity,
    templates,
  };
}

/**
 * Apply the in-monorepo filter rule from the RFC: entries with
 * `monorepo: true` are hidden when the command is invoked inside an
 * existing monorepo, mirroring `initial-template-options.ts:9-31`.
 */
export function filterManifestForContext(
  templates: ReadonlyArray<OrgTemplateEntry>,
  isMonorepo: boolean,
): Array<OrgTemplateEntry> {
  if (!isMonorepo) {
    return [...templates];
  }
  return templates.filter((entry) => !entry.monorepo);
}

import path from 'node:path';

import { fetchNpmResource, getNpmRegistry } from '../utils/npm-config.ts';

/**
 * A single entry in an org's template manifest.
 */
export interface OrgTemplateEntry {
  name: string;
  description: string;
  template: string;
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
  templates: OrgTemplateEntry[];
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

function validateEntry(entry: unknown, index: number, packageName: string): OrgTemplateEntry {
  if (!entry || typeof entry !== 'object') {
    throw new OrgManifestSchemaError(
      `createConfig.templates[${index}] must be an object`,
      packageName,
    );
  }
  const raw = entry as Record<string, unknown>;
  const requireString = (field: string): string => {
    const value = raw[field];
    if (typeof value !== 'string' || value.length === 0) {
      throw new OrgManifestSchemaError(
        `createConfig.templates[${index}].${field} must be a non-empty string`,
        packageName,
      );
    }
    return value;
  };
  const name = requireString('name');
  // `__vp_` is reserved for internal sentinel values (e.g. the
  // org-picker's escape-hatch nonce in `org-picker.ts`). Reject the
  // prefix at schema time so a manifest entry can never collide with
  // those sentinels regardless of what the picker does internally.
  if (name.startsWith('__vp_')) {
    throw new OrgManifestSchemaError(
      `createConfig.templates[${index}].name uses the reserved \`__vp_\` prefix`,
      packageName,
    );
  }
  const description = requireString('description');
  const template = requireString('template');

  let monorepo: boolean | undefined;
  if (raw.monorepo !== undefined) {
    if (typeof raw.monorepo !== 'boolean') {
      throw new OrgManifestSchemaError(
        `createConfig.templates[${index}].monorepo must be a boolean`,
        packageName,
      );
    }
    monorepo = raw.monorepo;
  }

  if (isRelativePath(template)) {
    // Defense-in-depth only: `resolveBundledPath` enforces the authoritative
    // check after extraction. We reject obvious root-escapes here so schema
    // errors surface before any tarball download happens.
    const resolved = path.posix.resolve('/root', template.replaceAll('\\', '/'));
    if (resolved !== '/root' && !resolved.startsWith('/root/')) {
      throw new OrgManifestSchemaError(
        `createConfig.templates[${index}].template escapes the package root: ${template}`,
        packageName,
      );
    }
  }

  return {
    name,
    description,
    template,
    ...(monorepo !== undefined ? { monorepo } : {}),
  };
}

function validateManifest(raw: unknown, packageName: string): OrgTemplateEntry[] | null {
  if (!raw || typeof raw !== 'object') {
    return null;
  }
  const createConfig = (raw as { createConfig?: unknown }).createConfig;
  if (!createConfig || typeof createConfig !== 'object') {
    return null;
  }
  const templates = (createConfig as { templates?: unknown }).templates;
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
  const entries: OrgTemplateEntry[] = [];
  const seen = new Set<string>();
  for (let index = 0; index < templates.length; index += 1) {
    const entry = validateEntry(templates[index], index, packageName);
    if (seen.has(entry.name)) {
      throw new OrgManifestSchemaError(
        `createConfig.templates[${index}].name duplicates an earlier entry: "${entry.name}"`,
        packageName,
      );
    }
    seen.add(entry.name);
    entries.push(entry);
  }
  return entries;
}

interface RegistryPackument {
  name?: string;
  'dist-tags'?: Record<string, string>;
  versions?: Record<string, RegistryVersionMeta>;
}

interface RegistryVersionMeta {
  version?: string;
  createConfig?: unknown;
  dist?: {
    tarball?: string;
    integrity?: string;
  };
}

async function fetchPackument(
  scope: string,
  packageName: string,
): Promise<RegistryPackument | null> {
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
  return (await response.json()) as RegistryPackument;
}

/**
 * Fetch `@scope/create` from the npm registry and parse its `createConfig.templates`
 * manifest.
 *
 * Returns `null` when:
 * - the package does not exist on the registry (404), or
 * - the package exists but has no `createConfig.templates` field
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
  let resolvedVersion: string | undefined;
  if (requestedVersion) {
    resolvedVersion =
      packument['dist-tags']?.[requestedVersion] ??
      (packument.versions?.[requestedVersion] ? requestedVersion : undefined);
    if (!resolvedVersion) {
      throw new OrgManifestSchemaError(
        `version "${requestedVersion}" not found (known tags: ${Object.keys(packument['dist-tags'] ?? {}).join(', ') || 'none'})`,
        packageName,
      );
    }
  } else {
    resolvedVersion = packument['dist-tags']?.latest;
    if (!resolvedVersion) {
      return null;
    }
  }
  const meta = packument.versions?.[resolvedVersion];
  if (!meta) {
    return null;
  }
  const templates = validateManifest(meta, packageName);
  if (!templates) {
    return null;
  }
  if (!meta.dist?.tarball) {
    throw new OrgManifestSchemaError(`missing dist.tarball for ${resolvedVersion}`, packageName);
  }
  return {
    scope,
    packageName,
    version: resolvedVersion,
    tarballUrl: meta.dist.tarball,
    integrity: meta.dist.integrity,
    templates,
  };
}

/**
 * Apply the in-monorepo filter rule from the RFC: entries with
 * `monorepo: true` are hidden when the command is invoked inside an
 * existing monorepo, mirroring `initial-template-options.ts:9-31`.
 */
export function filterManifestForContext(
  templates: readonly OrgTemplateEntry[],
  isMonorepo: boolean,
): OrgTemplateEntry[] {
  if (!isMonorepo) {
    return [...templates];
  }
  return templates.filter((entry) => !entry.monorepo);
}

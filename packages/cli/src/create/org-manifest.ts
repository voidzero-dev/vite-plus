import path from 'node:path';

import { getNpmRegistry } from '../utils/package.ts';

/**
 * A single entry in an org's template manifest.
 */
export interface OrgTemplateEntry {
  name: string;
  description: string;
  template: string;
  keywords?: string[];
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
 * Parse the leading `@scope[/name]` segment of a template specifier, ignoring
 * any trailing `@version` suffix on the name.
 *
 * Returns `null` if the input does not look like an org-scoped reference.
 */
export function parseOrgScopedSpec(spec: string): { scope: string; name?: string } | null {
  if (!spec.startsWith('@')) {
    return null;
  }
  const slashIndex = spec.indexOf('/');
  if (slashIndex === -1) {
    // `@scope` or `@scope@version`
    const atIndex = spec.indexOf('@', 1);
    const scope = atIndex === -1 ? spec : spec.slice(0, atIndex);
    return { scope };
  }
  const scope = spec.slice(0, slashIndex);
  const rest = spec.slice(slashIndex + 1);
  const atIndex = rest.indexOf('@');
  const name = atIndex === -1 ? rest : rest.slice(0, atIndex);
  if (!name) {
    return { scope };
  }
  return { scope, name };
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
    throw new OrgManifestSchemaError(`vp.templates[${index}] must be an object`, packageName);
  }
  const raw = entry as Record<string, unknown>;
  const requireString = (field: string): string => {
    const value = raw[field];
    if (typeof value !== 'string' || value.length === 0) {
      throw new OrgManifestSchemaError(
        `vp.templates[${index}].${field} must be a non-empty string`,
        packageName,
      );
    }
    return value;
  };
  const name = requireString('name');
  const description = requireString('description');
  const template = requireString('template');

  let keywords: string[] | undefined;
  if (raw.keywords !== undefined) {
    if (!Array.isArray(raw.keywords) || raw.keywords.some((k) => typeof k !== 'string')) {
      throw new OrgManifestSchemaError(
        `vp.templates[${index}].keywords must be an array of strings`,
        packageName,
      );
    }
    keywords = raw.keywords as string[];
  }

  let monorepo: boolean | undefined;
  if (raw.monorepo !== undefined) {
    if (typeof raw.monorepo !== 'boolean') {
      throw new OrgManifestSchemaError(
        `vp.templates[${index}].monorepo must be a boolean`,
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
        `vp.templates[${index}].template escapes the package root: ${template}`,
        packageName,
      );
    }
  }

  return {
    name,
    description,
    template,
    ...(keywords !== undefined ? { keywords } : {}),
    ...(monorepo !== undefined ? { monorepo } : {}),
  };
}

function validateManifest(raw: unknown, packageName: string): OrgTemplateEntry[] | null {
  if (!raw || typeof raw !== 'object') {
    return null;
  }
  const vp = (raw as { vp?: unknown }).vp;
  if (!vp || typeof vp !== 'object') {
    return null;
  }
  const templates = (vp as { templates?: unknown }).templates;
  if (templates === undefined) {
    return null;
  }
  if (!Array.isArray(templates)) {
    throw new OrgManifestSchemaError('vp.templates must be an array', packageName);
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
        `vp.templates[${index}].name duplicates an earlier entry: "${entry.name}"`,
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
  vp?: unknown;
  dist?: {
    tarball?: string;
    integrity?: string;
  };
}

async function fetchPackument(packageName: string): Promise<RegistryPackument | null> {
  // npm's registry URLs keep `@` and `/` unencoded
  // (`https://registry.npmjs.org/@scope/name`). Match that — private
  // registries often route on the literal path.
  const url = `${getNpmRegistry()}/${packageName}`;
  const response = await fetch(url, {
    headers: { accept: 'application/json' },
    signal: AbortSignal.timeout(5000),
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
 * Fetch `@scope/create` from the npm registry and parse its `vp.templates`
 * manifest.
 *
 * Returns `null` when:
 * - the package does not exist on the registry (404), or
 * - the package exists but has no `vp.templates` field
 *
 * Throws when:
 * - the `vp.templates` field is present but malformed (`OrgManifestSchemaError`), or
 * - the registry request fails for any non-404 reason
 */
export async function readOrgManifest(scope: string): Promise<OrgManifest | null> {
  if (!scope.startsWith('@')) {
    return null;
  }
  const packageName = `${scope}/create`;
  const packument = await fetchPackument(packageName);
  if (!packument) {
    return null;
  }
  const latestTag = packument['dist-tags']?.latest;
  if (!latestTag) {
    return null;
  }
  const meta = packument.versions?.[latestTag];
  if (!meta) {
    return null;
  }
  const templates = validateManifest(meta, packageName);
  if (!templates) {
    return null;
  }
  if (!meta.dist?.tarball) {
    throw new OrgManifestSchemaError(`missing dist.tarball for ${latestTag}`, packageName);
  }
  return {
    scope,
    packageName,
    version: latestTag,
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

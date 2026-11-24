import { coerce, isGreaterThanOrEqual } from 'verkit';

import { PackageManager } from '../types/index.ts';
import { VITE_PLUS_NAME } from './constants.ts';
import type { NpmWorkspaces } from './workspace.ts';
import type { YamlMapping } from './yaml-value.ts';
import type { YamlDocument } from './yaml.ts';

export type CatalogDependencyResolver = ((
  catalogSpec: string,
  dependencyName: string,
) => string | undefined) & {
  preferredCatalogSpec: string;
};

// Whether `version` is at least `minVersion`. A non-coercible version passes
// only when it is one of the given dist-tags, which always point at a current
// (feature-capable) release.
export function versionAtLeast(version: string, minVersion: string, tags: Array<string>): boolean {
  const coerced = coerce(version);
  if (coerced) {
    return isGreaterThanOrEqual(coerced, minVersion);
  }
  return tags.includes(version);
}

export function bunSupportsMergedDefaultCatalogs(version: string): boolean {
  // Bun 1.4.0 unified the default catalogs: https://github.com/oven-sh/bun/pull/38333.
  return versionAtLeast(version, '1.4.0', ['latest', 'canary']);
}

export function isNamedCatalog(
  catalogName: string,
  packageManager: PackageManager,
  version = 'latest',
): boolean {
  return (
    catalogName !== '' &&
    (catalogName !== 'default' ||
      packageManager === PackageManager.yarn ||
      (packageManager === PackageManager.bun && !bunSupportsMergedDefaultCatalogs(version)))
  );
}

export function createCatalogDependencyResolverFromCatalogs(
  catalog: Record<string, string> | undefined,
  catalogs: Record<string, Record<string, string>> | undefined,
  packageManager: PackageManager,
  version = 'latest',
): CatalogDependencyResolver {
  let defaultCatalog = catalog ?? catalogs?.default;
  if (isNamedCatalog('default', packageManager, version)) {
    defaultCatalog = catalog;
  } else if (packageManager === PackageManager.bun) {
    defaultCatalog = { ...catalog, ...catalogs?.default };
  }
  const preferredCatalogSpec = selectPreferredCatalogSpec(
    packageManager === PackageManager.bun ? defaultCatalog : catalog,
    catalogs,
    packageManager,
    version,
  );
  const resolver = (catalogSpec: string, dependencyName: string) => {
    const catalogName = catalogSpec.slice('catalog:'.length);
    if (isNamedCatalog(catalogName, packageManager, version)) {
      return catalogs?.[catalogName]?.[dependencyName];
    }
    return defaultCatalog?.[dependencyName];
  };
  return Object.assign(resolver, { preferredCatalogSpec });
}

export function selectPreferredCatalogSpec(
  catalog: YamlMapping | undefined,
  catalogs: Record<string, YamlMapping> | undefined,
  packageManager: PackageManager,
  version = 'latest',
): string {
  const candidates: Array<{ spec: string; values: YamlMapping }> = [];
  if (catalog) {
    candidates.push({ spec: 'catalog:', values: catalog });
  }
  for (const [name, values] of Object.entries(catalogs ?? {})) {
    candidates.push({
      spec: isNamedCatalog(name, packageManager, version) ? `catalog:${name}` : 'catalog:',
      values,
    });
  }

  // Keep the managed toolchain together when a project already has a catalog
  // for it (for example Vize's `catalogs.vite-stack` and Rari's
  // `catalogs.build`). Prefer vite-plus as the strongest signal, followed by
  // vite and vitest. Existing dependency references keep their exact catalog
  // spec; this choice is for newly injected dependencies and overrides.
  for (const dependencyName of [VITE_PLUS_NAME, 'vite', 'vitest']) {
    const matching = candidates.find(({ values }) => Object.hasOwn(values, dependencyName));
    if (matching) {
      return matching.spec;
    }
  }

  if (!catalog && catalogs?.default && isNamedCatalog('default', packageManager, version)) {
    return 'catalog:default';
  }
  return 'catalog:';
}

export function getYamlCatalogPath(
  doc: YamlDocument,
  catalogSpec: string,
  packageManager: PackageManager,
): Array<string> {
  const catalogName = catalogSpec.slice('catalog:'.length);
  if (isNamedCatalog(catalogName, packageManager)) {
    return ['catalogs', catalogName];
  }
  if (packageManager === PackageManager.yarn || doc.has('catalog')) {
    return ['catalog'];
  }
  if (doc.hasIn(['catalogs', 'default'])) {
    return ['catalogs', 'default'];
  }
  return ['catalog'];
}

export function getBunCatalogConfig(pkg: {
  workspaces?: NpmWorkspaces;
  catalog?: Record<string, string>;
  catalogs?: Record<string, Record<string, string>>;
}) {
  const { workspaces } = pkg;
  if (workspaces && !Array.isArray(workspaces)) {
    // Bun uses the nested catalog configuration as a whole, without falling back to root entries.
    const catalog = workspaces.catalog ?? workspaces.catalogs;
    if (catalog !== undefined && catalog !== null) {
      return workspaces;
    }
  }
  return pkg;
}

export function readBunCatalogDependencyResolver(
  pkg: {
    workspaces?: NpmWorkspaces;
    catalog?: Record<string, string>;
    catalogs?: Record<string, Record<string, string>>;
  },
  version = 'latest',
): CatalogDependencyResolver {
  const { catalog, catalogs } = getBunCatalogConfig(pkg);
  return createCatalogDependencyResolverFromCatalogs(
    catalog,
    catalogs,
    PackageManager.bun,
    version,
  );
}

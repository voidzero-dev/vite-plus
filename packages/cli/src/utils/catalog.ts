import { PackageManager } from '../types/index.ts';
import { VITE_PLUS_NAME } from './constants.ts';
import type { NpmWorkspaces } from './workspace.ts';
import type { YamlDocument } from './yaml.ts';

export type CatalogDependencyResolver = ((
  catalogSpec: string,
  dependencyName: string,
) => string | undefined) & {
  preferredCatalogSpec: string;
};

export function createCatalogDependencyResolverFromCatalogs(
  catalog: Record<string, string> | undefined,
  catalogs: Record<string, Record<string, string>> | undefined,
  packageManager: PackageManager,
): CatalogDependencyResolver {
  // Bun merges the two default catalogs; Yarn keeps them separate.
  const defaultCatalog =
    packageManager === PackageManager.bun
      ? { ...catalog, ...catalogs?.default }
      : packageManager === PackageManager.yarn
        ? catalog
        : (catalog ?? catalogs?.default);
  const preferredCatalogSpec = selectPreferredCatalogSpec(defaultCatalog, catalogs, packageManager);
  const resolver = (catalogSpec: string, dependencyName: string) => {
    const catalogName = catalogSpec.slice('catalog:'.length);
    if (catalogName && (catalogName !== 'default' || packageManager === PackageManager.yarn)) {
      return catalogs?.[catalogName]?.[dependencyName];
    }
    return defaultCatalog?.[dependencyName];
  };
  return Object.assign(resolver, { preferredCatalogSpec });
}

export function selectPreferredCatalogSpec(
  catalog: Record<string, string> | undefined,
  catalogs: Record<string, Record<string, string>> | undefined,
  packageManager: PackageManager,
): string {
  const candidates: Array<{ spec: string; values: Record<string, string> }> = [];
  if (catalog) {
    candidates.push({ spec: 'catalog:', values: catalog });
  }
  for (const [name, values] of Object.entries(catalogs ?? {})) {
    candidates.push({
      spec:
        name === 'default' && packageManager !== PackageManager.yarn
          ? 'catalog:'
          : `catalog:${name}`,
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

  if (!catalog && catalogs?.default && packageManager === PackageManager.yarn) {
    return 'catalog:default';
  }
  return 'catalog:';
}

export function getYamlCatalogPath(
  doc: YamlDocument,
  catalogSpec: string,
  packageManager: PackageManager,
): string[] {
  const catalogName = catalogSpec.slice('catalog:'.length);
  return catalogName && (catalogName !== 'default' || packageManager === PackageManager.yarn)
    ? ['catalogs', catalogName]
    : packageManager === PackageManager.yarn ||
        doc.has('catalog') ||
        !doc.hasIn(['catalogs', 'default'])
      ? ['catalog']
      : ['catalogs', 'default'];
}

export function getBunCatalogConfig(pkg: {
  workspaces?: NpmWorkspaces;
  catalog?: Record<string, string>;
  catalogs?: Record<string, Record<string, string>>;
}) {
  const nested = pkg.workspaces && !Array.isArray(pkg.workspaces) ? pkg.workspaces : undefined;
  // Bun uses the nested catalog configuration as a whole, without falling back to root entries.
  return nested?.catalog != null || nested?.catalogs != null ? nested : pkg;
}

export function readBunCatalogDependencyResolver(pkg: {
  workspaces?: NpmWorkspaces;
  catalog?: Record<string, string>;
  catalogs?: Record<string, Record<string, string>>;
}): CatalogDependencyResolver {
  const { catalog, catalogs } = getBunCatalogConfig(pkg);
  return createCatalogDependencyResolverFromCatalogs(catalog, catalogs, PackageManager.bun);
}

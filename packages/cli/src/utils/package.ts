import fs from 'node:fs';
import { createRequire } from 'node:module';
import os from 'node:os';
import path from 'node:path';

import { VITE_PLUS_NAME } from './constants.ts';
import { readJsonFile } from './json.ts';

export function getScopeFromPackageName(packageName: string): string {
  if (packageName.startsWith('@')) {
    return packageName.split('/')[0];
  }
  return '';
}

interface PackageMetadata {
  name: string;
  version: string;
  path: string;
}

export function detectPackageMetadata(
  projectPath: string,
  packageName: string,
): PackageMetadata | void {
  try {
    // Create require from the project path so resolution only searches
    // the project's node_modules, not the global installation's
    const require = createRequire(path.join(projectPath, 'noop.js'));
    const pkgFilePath = require.resolve(`${packageName}/package.json`);
    const pkg = JSON.parse(fs.readFileSync(pkgFilePath, 'utf8'));
    return {
      name: pkg.name,
      version: pkg.version,
      path: path.dirname(pkgFilePath),
    };
  } catch {
    // ignore MODULE_NOT_FOUND error
    return;
  }
}

/**
 * Read the nearest package.json file from the current directory up to the root directory.
 * @param currentDir - The current directory to start searching from.
 * @returns The package.json content as a JSON object, or null if no package.json is found.
 */
export function readNearestPackageJson(currentDir: string): Record<string, unknown> | null {
  do {
    const packageJsonPath = path.join(currentDir, 'package.json');
    if (fs.existsSync(packageJsonPath)) {
      return readJsonFile(packageJsonPath);
    }
    currentDir = path.dirname(currentDir);
  } while (currentDir !== path.dirname(currentDir));
  return null;
}

export function hasVitePlusDependency(
  pkg?: {
    dependencies?: Record<string, string>;
    devDependencies?: Record<string, string>;
  } | null,
) {
  return Boolean(pkg?.dependencies?.[VITE_PLUS_NAME] || pkg?.devDependencies?.[VITE_PLUS_NAME]);
}

type NpmConfig = Map<string, string>;

function expandNpmrcValue(raw: string): string {
  // Strip surrounding quotes and expand `${VAR}` references — matches what
  // npm does when reading `.npmrc`.
  let value = raw.trim();
  if (
    (value.startsWith('"') && value.endsWith('"')) ||
    (value.startsWith("'") && value.endsWith("'"))
  ) {
    value = value.slice(1, -1);
  }
  return value.replaceAll(/\$\{([A-Z0-9_]+)\}/gi, (_, name) => process.env[name] ?? '');
}

function parseNpmrc(contents: string, into: NpmConfig): void {
  for (const rawLine of contents.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith('#') || line.startsWith(';')) {
      continue;
    }
    const eq = line.indexOf('=');
    if (eq === -1) {
      continue;
    }
    const key = line.slice(0, eq).trim();
    const value = expandNpmrcValue(line.slice(eq + 1));
    if (key) {
      into.set(key, value);
    }
  }
}

function loadFileInto(filePath: string, config: NpmConfig): void {
  try {
    parseNpmrc(fs.readFileSync(filePath, 'utf8'), config);
  } catch {
    // Missing / unreadable .npmrc is fine — nothing to layer in.
  }
}

function getNpmConfig(): NpmConfig {
  const config: NpmConfig = new Map();
  // Layer in order of increasing precedence: user → project → env.
  loadFileInto(path.join(os.homedir(), '.npmrc'), config);
  let dir = process.cwd();
  const seen = new Set<string>();
  while (dir && !seen.has(dir)) {
    seen.add(dir);
    const candidate = path.join(dir, '.npmrc');
    if (fs.existsSync(candidate)) {
      loadFileInto(candidate, config);
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      break;
    }
    dir = parent;
  }
  for (const [envKey, envValue] of Object.entries(process.env)) {
    if (envValue === undefined) {
      continue;
    }
    if (envKey.startsWith('npm_config_')) {
      config.set(envKey.slice('npm_config_'.length), envValue);
    } else if (envKey.startsWith('NPM_CONFIG_')) {
      config.set(envKey.slice('NPM_CONFIG_'.length).toLowerCase(), envValue);
    }
  }
  return config;
}

function normalizeRegistryUrl(url: string): string {
  return url.replace(/\/+$/, '');
}

/**
 * Resolve the npm registry base URL for the given scope (or the default
 * registry when `scope` is omitted). Honors `@scope:registry=...` entries
 * in `.npmrc` files and the matching `npm_config_@scope:registry` env
 * vars so private / mirrored registries work for org manifest fetches.
 */
export function getNpmRegistry(scope?: string): string {
  const config = getNpmConfig();
  if (scope) {
    const normalizedScope = scope.startsWith('@') ? scope : `@${scope}`;
    const scoped = config.get(`${normalizedScope}:registry`);
    if (scoped) {
      return normalizeRegistryUrl(scoped);
    }
  }
  const registry = config.get('registry') || 'https://registry.npmjs.org';
  return normalizeRegistryUrl(registry);
}

/**
 * Build the `Authorization` header value for a registry URL by matching
 * the URL against `//host[/path]/:_authToken=...` / `:_auth=...` entries
 * in `.npmrc`. Returns `undefined` when no credential is configured.
 */
export function getNpmAuthHeader(registryOrUrl: string): string | undefined {
  let parsed: URL;
  try {
    parsed = new URL(registryOrUrl);
  } catch {
    return undefined;
  }
  const config = getNpmConfig();
  // npm keys a credential by the protocol-less URL with a trailing slash,
  // e.g. `//registry.example.com/foo/:_authToken`. Walk up the path so
  // `/foo/bar` also matches a credential set for `/foo` or the host root.
  const segments = parsed.pathname.split('/').filter(Boolean);
  const candidates: string[] = [];
  for (let i = segments.length; i >= 0; i -= 1) {
    const subPath = i === 0 ? '/' : `/${segments.slice(0, i).join('/')}/`;
    candidates.push(`//${parsed.host}${subPath}`);
  }
  for (const prefix of candidates) {
    const token = config.get(`${prefix}:_authToken`);
    if (token) {
      return `Bearer ${token}`;
    }
    const basic = config.get(`${prefix}:_auth`);
    if (basic) {
      return `Basic ${basic}`;
    }
    const username = config.get(`${prefix}:username`);
    const passwordB64 = config.get(`${prefix}:_password`);
    if (username && passwordB64) {
      const password = Buffer.from(passwordB64, 'base64').toString('utf8');
      return `Basic ${Buffer.from(`${username}:${password}`).toString('base64')}`;
    }
  }
  return undefined;
}

/**
 * Check if an npm package exists in the public registry.
 * Returns true if the package exists or if the check could not be performed (network error, timeout).
 * Returns false only if the registry definitively responds with 404.
 */
export async function checkNpmPackageExists(packageName: string): Promise<boolean> {
  const atIndex = packageName.indexOf('@', 2);
  const name = atIndex === -1 ? packageName : packageName.slice(0, atIndex);
  try {
    const response = await fetch(`${getNpmRegistry()}/${name}`, {
      method: 'HEAD',
      signal: AbortSignal.timeout(3000),
    });
    return response.status !== 404;
  } catch {
    return true; // Network error or timeout - let the package manager handle it
  }
}

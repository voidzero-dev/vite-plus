import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

type NpmConfig = Map<string, string>;

function expandNpmrcValue(raw: string): string {
  // Strip surrounding quotes and expand `${VAR}` references. Covers only
  // the value shapes used by the keys we actually read (registry /
  // @scope:registry / :_authToken / :_auth / :username / :_password).
  // Intentionally NOT handled: `\$` backslash escapes, `${VAR-default}`
  // fallbacks, inline `;` comments after a value, and `key[]=` list
  // syntax. Extend when a caller needs any of those.
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

/**
 * Rebuilt on every call so tests that mutate `process.env` mid-run see
 * fresh config. Each `vp create` hits this ≤4 times (registry + auth on
 * packument + auth on tarball), which is cheap enough vs. the network
 * work that a cache isn't worth the test-determinism cost.
 */
function getNpmConfig(): NpmConfig {
  const config: NpmConfig = new Map();
  // Layer in order of increasing precedence: user → project → env.
  const homeNpmrc = path.resolve(os.homedir(), '.npmrc');
  loadFileInto(homeNpmrc, config);
  // Collect project `.npmrc` paths from cwd up to the filesystem root,
  // then apply them in reverse (root-side first, cwd last) so the
  // innermost file wins. Skip the `$HOME/.npmrc` we already loaded so
  // it doesn't re-overwrite project-level settings when cwd is under
  // `$HOME`.
  const projectRcs: string[] = [];
  let dir = path.resolve(process.cwd());
  const seen = new Set<string>();
  while (dir && !seen.has(dir)) {
    seen.add(dir);
    const candidate = path.resolve(dir, '.npmrc');
    if (candidate !== homeNpmrc && fs.existsSync(candidate)) {
      projectRcs.push(candidate);
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      break;
    }
    dir = parent;
  }
  for (let i = projectRcs.length - 1; i >= 0; i -= 1) {
    loadFileInto(projectRcs[i], config);
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
 * `fetch` wrapper for npm registry URLs that retries with an
 * `Authorization` header on 401/403. Public registries never see the
 * token — we only reach into `.npmrc` when the server challenges us.
 *
 * `init.headers` is forwarded verbatim on both attempts (the retry
 * merges in the discovered auth header on top).
 */
export async function fetchNpmResource(
  url: string,
  init: Omit<RequestInit, 'signal'> & { timeoutMs: number },
): Promise<Response> {
  const { timeoutMs, headers: callerHeaders, ...rest } = init;
  const first = await fetch(url, {
    ...rest,
    headers: callerHeaders,
    signal: AbortSignal.timeout(timeoutMs),
  });
  if (first.status !== 401 && first.status !== 403) {
    return first;
  }
  const authorization = getNpmAuthHeader(url);
  if (!authorization) {
    return first;
  }
  return fetch(url, {
    ...rest,
    headers: { ...(callerHeaders as Record<string, string> | undefined), authorization },
    signal: AbortSignal.timeout(timeoutMs),
  });
}

import { resolveNpmAuthorization, resolveNpmRegistry } from '../../binding/index.js';

/**
 * Resolve the npm registry base URL for the given scope (or the default
 * registry when `scope` is omitted). Honors `@scope:registry=...` entries
 * in `.npmrc` files and the matching `npm_config_@scope:registry` env
 * vars so private / mirrored registries work for org manifest fetches.
 */
export function getNpmRegistry(scope?: string): string {
  return resolveNpmRegistry(process.cwd(), scope?.replace(/^@/, ''));
}

/**
 * Build the `Authorization` header value for a registry URL from the
 * matching `//host[/path]/:_authToken=...` / `:_auth=...` entries in
 * `.npmrc`. Returns `undefined` when no credential is configured.
 */
export function getNpmAuthHeader(registryOrUrl: string): string | undefined {
  return resolveNpmAuthorization(process.cwd(), registryOrUrl) ?? undefined;
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

// Minimal mock npm registry used by `create-org-*` snap-tests.
//
// Reads `./mock-manifest.json` (keyed by URL path, e.g. `"@your-org/create"`) and
// optionally serves `.tgz` tarballs from `./tarballs/<name>`. Picks an
// ephemeral port, sets `NPM_CONFIG_REGISTRY` on the child environment, spawns
// the wrapped command, and tears down when the child exits.
//
// Usage: node mock-server.mjs -- <command> [args...]

import { spawn } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { get as httpsGet } from 'node:https';
import path from 'node:path';

const manifest = JSON.parse(readFileSync('./mock-manifest.json', 'utf-8'));
const UPSTREAM_REGISTRY = 'https://registry.npmjs.org';

function rewriteRegistry(value, registry) {
  return JSON.parse(JSON.stringify(value).replaceAll('{REGISTRY}', registry));
}

// Stream the upstream response byte-for-byte. Unlike `fetch`, `https` does not
// auto-decompress, so the tarball reaches the client exactly as the registry
// served it (content-encoding and all). bun verifies tarball integrity and
// rejects any re-encoded body, so faithful streaming is what lets bun installs
// work through the proxy (pnpm fetches tarballs from the upstream URL directly,
// so it was never affected).
function proxyToUpstream(req, res) {
  // Stream errors can fire after headers are already sent (e.g. the upstream
  // connection resets mid-tarball), so guard against a second writeHead.
  const fail = (error) => {
    if (!res.headersSent) {
      res.writeHead(502);
    }
    res.end(`proxy error: ${error.message}`);
  };
  const fetchUrl = (url, redirectsLeft) => {
    httpsGet(url, { headers: { accept: req.headers.accept ?? 'application/json' } }, (upstream) => {
      const status = upstream.statusCode ?? 502;
      if (status >= 300 && status < 400 && upstream.headers.location && redirectsLeft > 0) {
        // Draining can still emit 'error' (e.g. the socket resets mid-redirect),
        // so guard it here too — otherwise it's uncaught and crashes the server.
        upstream.on('error', fail);
        upstream.resume();
        fetchUrl(new URL(upstream.headers.location, url).toString(), redirectsLeft - 1);
        return;
      }
      const headers = {};
      // Forward `location` too, so a 3xx we stop following (or one with no
      // location to follow) still reaches the client with its redirect target.
      for (const name of ['content-type', 'content-encoding', 'content-length', 'location']) {
        if (upstream.headers[name] !== undefined) {
          headers[name] = upstream.headers[name];
        }
      }
      // Default a missing content-type (parity with the prior fetch-based proxy)
      // so clients that key off it still recognize a proxied tarball.
      headers['content-type'] ??= 'application/octet-stream';
      res.writeHead(status, headers);
      // `pipe` does not forward source errors, so listen on the response stream
      // directly; otherwise a mid-stream upstream error is uncaught and crashes
      // the mock server.
      upstream.on('error', fail);
      upstream.pipe(res);
    }).on('error', fail);
  };
  fetchUrl(`${UPSTREAM_REGISTRY}${req.url ?? '/'}`, 5);
}

const server = createServer(async (req, res) => {
  const key = decodeURIComponent(req.url ?? '/').replace(/^\/+/, '');
  if (Object.hasOwn(manifest, key)) {
    const address = server.address();
    const registry =
      address && typeof address !== 'string' ? `http://127.0.0.1:${address.port}` : '';
    res.writeHead(200, { 'content-type': 'application/json' });
    res.end(JSON.stringify(rewriteRegistry(manifest[key], registry)));
    return;
  }
  const tarMatch = key.match(/\/-\/([^/]+\.tgz)$/);
  if (tarMatch) {
    try {
      const bytes = readFileSync(path.resolve('./tarballs', tarMatch[1]));
      res.writeHead(200, { 'content-type': 'application/octet-stream' });
      res.end(bytes);
      return;
    } catch {
      // fall through to proxy
    }
  }
  // Proxy anything we don't mock (pnpm/latest, tarball downloads for real
  // packages, etc.) to the upstream registry. Keeps the fixture scoped to
  // just the @org/create manifest while letting vp's other startup work
  // (package-manager download) succeed normally.
  await proxyToUpstream(req, res);
});

server.listen(0, '127.0.0.1', () => {
  const address = server.address();
  if (!address || typeof address === 'string') {
    console.error('mock-server: failed to bind');
    process.exit(1);
  }
  const registry = `http://127.0.0.1:${address.port}`;
  const separatorIndex = process.argv.indexOf('--');
  if (separatorIndex === -1) {
    console.error('usage: node mock-server.mjs -- <command> [args...]');
    server.close(() => process.exit(2));
    return;
  }
  const [cmd, ...args] = process.argv.slice(separatorIndex + 1);
  const child = spawn(cmd, args, {
    env: { ...process.env, NPM_CONFIG_REGISTRY: registry },
    stdio: 'inherit',
  });
  child.on('exit', (code, signal) => {
    const exitCode = code ?? (signal ? 128 : 0);
    server.close(() => process.exit(exitCode));
  });
});

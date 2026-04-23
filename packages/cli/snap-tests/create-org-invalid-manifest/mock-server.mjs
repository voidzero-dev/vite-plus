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
import path from 'node:path';

const manifest = JSON.parse(readFileSync('./mock-manifest.json', 'utf-8'));
const UPSTREAM_REGISTRY = 'https://registry.npmjs.org';

function rewriteRegistry(value, registry) {
  return JSON.parse(JSON.stringify(value).replaceAll('{REGISTRY}', registry));
}

async function proxyToUpstream(req, res) {
  try {
    const upstream = await fetch(`${UPSTREAM_REGISTRY}${req.url ?? '/'}`, {
      method: req.method,
      headers: { accept: req.headers.accept ?? 'application/json' },
    });
    const body = Buffer.from(await upstream.arrayBuffer());
    res.writeHead(upstream.status, {
      'content-type': upstream.headers.get('content-type') ?? 'application/octet-stream',
    });
    res.end(body);
  } catch (error) {
    res.writeHead(502);
    res.end(`proxy error: ${error.message}`);
  }
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

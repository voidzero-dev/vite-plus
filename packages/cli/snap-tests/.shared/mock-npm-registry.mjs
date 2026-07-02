// Minimal mock npm registry used by snap-tests that need install-time control
// over specific packages (`create-org-*`, and every fixture that installs the
// locally packed Vite+ packages).
//
// Reads `./mock-manifest.json` if present (keyed by URL path, e.g.
// `"@your-org/create"`) and optionally serves `.tgz` tarballs from
// `./tarballs/<name>`. When `SNAP_LOCAL_VP_PACKAGES_DIR` is set (see
// `localVitePlusPackages` in the snap-test harness), every tarball in that
// directory is served as a single-version packument, so package managers can
// install the checkout's own `vite-plus` / `@voidzero-dev/vite-plus-core`
// even when that version is not published on npm. Picks an ephemeral port,
// points the package managers of the child environment at it, spawns the
// wrapped command, and tears down when the child exits.
//
// Usage: node mock-server.mjs -- <command> [args...]

import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync } from 'node:fs';
import { createServer } from 'node:http';
import { Agent as HttpsAgent, get as httpsGet } from 'node:https';
import { homedir, tmpdir } from 'node:os';
import path from 'node:path';
import { gunzipSync } from 'node:zlib';

const manifest = existsSync('./mock-manifest.json')
  ? JSON.parse(readFileSync('./mock-manifest.json', 'utf-8'))
  : {};

// Proxy through the user's configured registry (e.g. a local mirror in
// `~/.npmrc`) when there is one, so local runs stay as fast as direct
// installs. CI has no user registry config and uses npmjs.
function resolveUpstreamRegistry() {
  try {
    const npmrc = readFileSync(path.join(homedir(), '.npmrc'), 'utf-8');
    const registryLine = npmrc.split('\n').find((line) => line.trim().startsWith('registry='));
    const registry = registryLine?.split('=')[1]?.trim().replace(/\/+$/, '');
    // The proxy fetches with node:https, so only accept https upstreams.
    if (registry?.startsWith('https://')) {
      return registry;
    }
  } catch {
    // no user .npmrc: use the default registry
  }
  return 'https://registry.npmjs.org';
}

const UPSTREAM_REGISTRY = resolveUpstreamRegistry();

// Reuse upstream connections: an install fetches hundreds of packuments, and
// a fresh TLS handshake per request multiplies into minutes of overhead.
const upstreamAgent = new HttpsAgent({ keepAlive: true, maxSockets: 64 });

// Minimal ustar walk. The manifest is always the `package/package.json` entry
// in a pnpm-packed tarball, so long-name (pax header) handling is unnecessary.
function readPackageJsonFromTarball(tgzBytes, sourcePath) {
  const tar = gunzipSync(tgzBytes);
  for (let offset = 0; offset + 512 <= tar.length; ) {
    const name = tar
      .subarray(offset, offset + 100)
      .toString()
      .replace(/\0[^]*$/, '');
    if (!name) {
      break; // end-of-archive marker
    }
    const size =
      Number.parseInt(
        tar
          .subarray(offset + 124, offset + 136)
          .toString()
          .trim(),
        8,
      ) || 0;
    if (name === 'package/package.json') {
      return JSON.parse(tar.subarray(offset + 512, offset + 512 + size).toString());
    }
    offset += 512 + Math.ceil(size / 512) * 512;
  }
  throw new Error(`package/package.json not found in ${sourcePath}`);
}

const localPackagesDir = process.env.SNAP_LOCAL_VP_PACKAGES_DIR;
const localTarballs = new Map(); // tarball basename -> absolute path
// Far in the past so package-manager minimum-release-age gates never
// quarantine the locally served versions.
const LOCAL_PACKAGE_TIME = '2020-01-01T00:00:00.000Z';
if (localPackagesDir) {
  for (const basename of readdirSync(localPackagesDir)) {
    if (!basename.endsWith('.tgz')) {
      continue;
    }
    const tgzPath = path.join(localPackagesDir, basename);
    const bytes = readFileSync(tgzPath);
    const pkg = readPackageJsonFromTarball(bytes, tgzPath);
    localTarballs.set(basename, tgzPath);
    manifest[pkg.name] = {
      name: pkg.name,
      'dist-tags': { latest: pkg.version },
      versions: {
        [pkg.version]: {
          ...pkg,
          dist: {
            tarball: `{REGISTRY}/${pkg.name}/-/${basename}`,
            // Integrity over the exact bytes served, so every package manager
            // that verifies it (npm, pnpm, yarn, bun) gets a match.
            integrity: `sha512-${createHash('sha512').update(bytes).digest('base64')}`,
            shasum: createHash('sha1').update(bytes).digest('hex'),
          },
        },
      },
      time: {
        created: LOCAL_PACKAGE_TIME,
        modified: LOCAL_PACKAGE_TIME,
        [pkg.version]: LOCAL_PACKAGE_TIME,
      },
    };
  }
}

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
    // Tarballs: `accept-encoding: identity` keeps mirrors/CDNs from wrapping
    // them in an extra content-encoding layer, which some package managers
    // cannot unwrap when verifying/extracting. Metadata: honor the client's
    // own accept-encoding — the body is streamed through verbatim with its
    // content-encoding header, so it must be exactly what the client can
    // decode (package managers ask for gzip, which keeps huge packuments like
    // vite/typescript fast; vp's Rust registry client asks for identity).
    const headers = {
      accept: req.headers.accept ?? 'application/json',
      'accept-encoding': req.url?.endsWith('.tgz')
        ? 'identity'
        : (req.headers['accept-encoding'] ?? 'identity'),
    };
    httpsGet(url, { headers, agent: upstreamAgent }, (upstream) => {
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
      const bytes = readFileSync(
        localTarballs.get(tarMatch[1]) ?? path.resolve('./tarballs', tarMatch[1]),
      );
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
  // Yarn Berry persists packuments (with our ephemeral-port tarball URLs) in
  // its global folder, and bun's install cache trusts name@version without
  // refetching, so a later invocation would reuse dead URLs or stale local
  // package bytes. Give each invocation throwaway caches instead.
  const yarnGlobalFolder = mkdtempSync(path.join(tmpdir(), 'vp-mock-registry-yarn-'));
  const bunCacheDir = mkdtempSync(path.join(tmpdir(), 'vp-mock-registry-bun-'));
  const child = spawn(cmd, args, {
    env: {
      ...process.env,
      // Every package manager reads its own env spelling: npm and bun honor
      // NPM_CONFIG_REGISTRY, pnpm >= 10.6 only reads PNPM_CONFIG_* (older
      // pnpm read the lowercase npm_config_* form), and Yarn Berry only reads
      // YARN_-prefixed settings and refuses plain-http registries unless the
      // host is whitelisted.
      NPM_CONFIG_REGISTRY: registry,
      npm_config_registry: registry,
      PNPM_CONFIG_REGISTRY: registry,
      YARN_NPM_REGISTRY_SERVER: registry,
      YARN_UNSAFE_HTTP_WHITELIST: '127.0.0.1',
      YARN_GLOBAL_FOLDER: yarnGlobalFolder,
      BUN_INSTALL_CACHE_DIR: bunCacheDir,
    },
    stdio: 'inherit',
  });
  child.on('exit', (code, signal) => {
    rmSync(yarnGlobalFolder, { recursive: true, force: true });
    rmSync(bunCacheDir, { recursive: true, force: true });
    const exitCode = code ?? (signal ? 128 : 0);
    server.close(() => process.exit(exitCode));
  });
});

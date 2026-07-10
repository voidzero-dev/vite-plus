// Local npm registry for testing Vite+ installs against the checkout.
//
// Serves locally packed packages (vite-plus, @voidzero-dev/vite-plus-core,
// or any other tgz) as single-version packuments behind a real registry HTTP
// interface, and proxies everything else upstream. Package managers then
// install the checkout's own version even when it is not published on npm,
// with no `file:` specs, pkg.pr.new publish, or registry-bridge round-trip.
//
// Runs directly with `node` (erasable-syntax TypeScript, no loader needed).
//
// Used by:
// - snap tests: the harness packs the checkout once per run (see
//   `localVitePlusPackages` in `snap-test.ts`) and fixtures wrap commands with
//   `node $SNAP_LOCAL_REGISTRY -- vp ...`. Fixtures may also provide a
//   `./mock-manifest.json` (keyed by URL path, e.g. `"@your-org/create"`) and
//   `./tarballs/<name>` in their case directory (the `create-org-*` cases).
// - ecosystem e2e: `patch-project.ts` serves the e2e tgz artifacts with
//   `--packages-dir` so `vp migrate` / `vp install` resolve the local build
//   through the standard registry code paths.
// - local development: run `vp migrate` / `vp create` against the checkout
//   from any project directory without publishing anything:
//     node <repo>/packages/tools/src/local-npm-registry.ts --pack -- vp migrate --no-interactive
//   or keep a server running and export the printed env for repeated runs:
//     node <repo>/packages/tools/src/local-npm-registry.ts --pack --serve
//
// Usage:
//   node local-npm-registry.ts [--packages-dir <dir>] [--pack] [--serve] [-- <command> [args...]]
//   node local-npm-registry.ts --pack-to <dir>
//   node local-npm-registry.ts --ps | --kill
//
//   --packages-dir <dir>  serve every *.tgz in <dir> (defaults to
//                         $SNAP_LOCAL_VP_PACKAGES_DIR when set)
//   --pack                pack the checkout's vite-plus and
//                         @voidzero-dev/vite-plus-core into a temp dir first
//   --pack-to <dir>       pack the checkout packages into <dir> and exit
//                         (no server); the snapshot runner packs once per run
//   --serve               keep the server running and print the registry URL
//                         and env exports instead of wrapping a command
//   --ps                  list running local registry processes
//   --kill                kill them all and remove their leftover temp caches

import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import {
  createServer,
  type IncomingMessage,
  type OutgoingHttpHeaders,
  type ServerResponse,
} from 'node:http';
import { Agent as HttpsAgent, get as httpsGet, request as httpsRequest } from 'node:https';
import { homedir, tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { gunzipSync } from 'node:zlib';

import { packLocalVitePlusPackages } from './pack-local-vite-plus.ts';

interface PackageManifest {
  name: string;
  version: string;
  [key: string]: unknown;
}

const args = process.argv.slice(2);
const separatorIndex = args.indexOf('--');
const flags = separatorIndex === -1 ? args : args.slice(0, separatorIndex);
const command = separatorIndex === -1 ? [] : args.slice(separatorIndex + 1);

let packagesDir = process.env.SNAP_LOCAL_VP_PACKAGES_DIR;
let pack = false;
let packTo: string | undefined;
let serve = false;
let listProcesses = false;
let killProcesses = false;
for (let i = 0; i < flags.length; i++) {
  if (flags[i] === '--packages-dir' && flags[i + 1]) {
    packagesDir = flags[++i];
  } else if (flags[i] === '--pack') {
    pack = true;
  } else if (flags[i] === '--pack-to' && flags[i + 1]) {
    packTo = flags[++i];
  } else if (flags[i] === '--serve') {
    serve = true;
  } else if (flags[i] === '--ps') {
    listProcesses = true;
  } else if (flags[i] === '--kill') {
    killProcesses = true;
  } else {
    console.error(`local-npm-registry: unknown option ${flags[i]}`);
    process.exit(2);
  }
}

// `--ps` / `--kill`: troubleshoot registry processes left behind by
// interrupted runs (the wrapper and --serve clean up after themselves, but a
// hard kill skips the handlers). A registry process is a `node` executable
// running this script; requiring `node` as the executing binary (start of
// the command line or preceded by a path separator) keeps shells whose
// command STRING merely mentions the script (`sh -c 'node ...'`, this very
// invocation's pnpm wrapper) out of the kill list.
const REGISTRY_PROCESS_RE = /(^|[/\\])node(\.exe)?['"]?\s[^\n]*local-npm-registry\.ts/;

function findRegistryProcesses(): { pid: number; command: string }[] {
  const result =
    process.platform === 'win32'
      ? spawnSync(
          'powershell.exe',
          [
            '-NoProfile',
            '-Command',
            'Get-CimInstance Win32_Process -Filter "Name = \'node.exe\'" | ForEach-Object { "$($_.ProcessId) $($_.CommandLine)" }',
          ],
          { encoding: 'utf8' },
        )
      : spawnSync('ps', ['-eo', 'pid=,args='], { encoding: 'utf8' });
  return (result.stdout ?? '')
    .split('\n')
    .map((line) => line.trim())
    .map((line) => {
      const space = line.indexOf(' ');
      return { pid: Number(line.slice(0, space)), command: line.slice(space + 1).trim() };
    })
    .filter(
      ({ pid, command: cmd }) =>
        Number.isFinite(pid) && pid !== process.pid && REGISTRY_PROCESS_RE.test(cmd),
    );
}

if (listProcesses || killProcesses) {
  const processes = findRegistryProcesses();
  const lines = processes.map(
    ({ pid, command: cmd }) => `${killProcesses ? 'killed ' : ''}${pid} ${cmd}`,
  );
  if (processes.length === 0) {
    lines.push('No local registry processes running');
  }
  if (killProcesses) {
    for (const { pid } of processes) {
      try {
        process.kill(pid);
      } catch {
        // already gone
      }
    }
    let removed = 0;
    for (const entry of readdirSync(tmpdir())) {
      if (entry.startsWith('vp-local-registry-')) {
        rmSync(path.join(tmpdir(), entry), { recursive: true, force: true });
        removed++;
      }
    }
    lines.push(`Removed ${removed} leftover temp cache dir(s)`);
  }
  process.stdout.write(`${lines.join('\n')}\n`);
  process.exit(0);
}

// The script lives at `packages/tools/src/`, so the repo root is three up.
const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..');

// `--pack-to <dir>`: pack the checkout packages into <dir> and exit, without
// serving. The PTY snapshot runner calls this once per run, then points each
// case's `--serve` registry at the shared dir via SNAP_LOCAL_VP_PACKAGES_DIR
// (the packed-once-per-run optimization the legacy harness did in-process).
if (packTo) {
  await packLocalVitePlusPackages(repoRoot, packTo);
  process.exit(0);
}

if (!serve && command.length === 0) {
  console.error(
    'usage: node local-npm-registry.ts [--packages-dir <dir>] [--pack | --pack-to <dir>] [--serve] [-- <command> [args...]] | --ps | --kill',
  );
  process.exit(2);
}

// Tracked so the normal cleanup paths remove the packed tarballs; a hard
// kill leaves it behind, which `--kill` sweeps up with the other
// vp-local-registry-* leftovers.
let packedDir: string | undefined;
if (pack && !packagesDir) {
  packedDir = mkdtempSync(path.join(tmpdir(), 'vp-local-registry-pack-'));
  await packLocalVitePlusPackages(repoRoot, packedDir);
  packagesDir = packedDir;
}

const manifest: Record<string, unknown> = existsSync('./mock-manifest.json')
  ? (JSON.parse(readFileSync('./mock-manifest.json', 'utf-8')) as Record<string, unknown>)
  : {};

// Proxy through the configured registry (the project's `.npmrc` in cwd, then
// the user's `~/.npmrc`, e.g. a local mirror) when there is one, so runs stay
// as fast as direct installs and projects that rely on a custom registry keep
// resolving from it. CI has no registry config and uses npmjs. Deliberately
// reads only `.npmrc` files and NOT registry env vars (unlike the CLI's
// getNpmRegistry): a leftover NPM_CONFIG_REGISTRY export from a previous
// `--serve` session would otherwise become this server's own upstream (the
// https-only guard below rejects such http URLs for the same reason).
// Known out-of-scope for this test tool: HTTP(S)_PROXY tunneling and
// authenticated upstream registries; the environments it serves (repo CI and
// local dev) need neither.
function resolveUpstreamRegistry(): string {
  for (const npmrcPath of [path.resolve('.npmrc'), path.join(homedir(), '.npmrc')]) {
    try {
      const npmrc = readFileSync(npmrcPath, 'utf-8');
      const registryLine = npmrc.split('\n').find((line) => line.trim().startsWith('registry='));
      const registry = registryLine?.split('=')[1]?.trim().replace(/\/+$/, '');
      // The proxy fetches with node:https, so only accept https upstreams.
      if (registry?.startsWith('https://')) {
        return registry;
      }
    } catch {
      // no such .npmrc: try the next one
    }
  }
  return 'https://registry.npmjs.org';
}

const UPSTREAM_REGISTRY = resolveUpstreamRegistry();

// Reuse upstream connections: an install fetches hundreds of packuments, and
// a fresh TLS handshake per request multiplies into minutes of overhead.
const upstreamAgent = new HttpsAgent({ keepAlive: true, maxSockets: 64 });

// Minimal ustar walk. The manifest is always the `package/package.json` entry
// in a pnpm-packed tarball, so long-name (pax header) handling is unnecessary.
function readPackageJsonFromTarball(tgzBytes: Buffer, sourcePath: string): PackageManifest {
  const tar = gunzipSync(tgzBytes);
  for (let offset = 0; offset + 512 <= tar.length;) {
    const rawName = tar.subarray(offset, offset + 100).toString();
    const nulIndex = rawName.indexOf('\0');
    const name = nulIndex === -1 ? rawName : rawName.slice(0, nulIndex);
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
      return JSON.parse(
        tar.subarray(offset + 512, offset + 512 + size).toString(),
      ) as PackageManifest;
    }
    offset += 512 + Math.ceil(size / 512) * 512;
  }
  throw new Error(`package/package.json not found in ${sourcePath}`);
}

interface Packument {
  name: string;
  'dist-tags': Record<string, string>;
  versions: Record<string, unknown>;
  time: Record<string, string>;
}

const localTarballs = new Map<string, string>(); // tarball basename -> absolute path
const localPackuments = new Map<string, Packument>(); // package name -> local-only packument
// Far in the past so package-manager minimum-release-age gates never
// quarantine the locally served versions.
const LOCAL_PACKAGE_TIME = '2020-01-01T00:00:00.000Z';
if (packagesDir) {
  for (const basename of readdirSync(packagesDir)) {
    if (!basename.endsWith('.tgz')) {
      continue;
    }
    const tgzPath = path.join(packagesDir, basename);
    const bytes = readFileSync(tgzPath);
    const pkg = readPackageJsonFromTarball(bytes, tgzPath);
    localTarballs.set(basename, tgzPath);
    localPackuments.set(pkg.name, {
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
    });
  }
}

// Serialize a manifest/packument with its `{REGISTRY}` placeholders (only our
// local tarball URLs carry them) pointed at the live server address.
function serializeForRegistry(value: unknown, registry: string): string {
  return JSON.stringify(value).replaceAll('{REGISTRY}', registry);
}

function fetchUpstreamPackument(name: string): Promise<Packument | null> {
  return new Promise((resolve) => {
    httpsGet(
      `${UPSTREAM_REGISTRY}/${name.replace('/', '%2F')}`,
      { headers: { accept: 'application/json', 'accept-encoding': 'gzip' }, agent: upstreamAgent },
      (upstream) => {
        if (upstream.statusCode !== 200) {
          upstream.resume();
          resolve(null);
          return;
        }
        const chunks: Buffer[] = [];
        upstream.on('data', (chunk: Buffer) => chunks.push(chunk));
        upstream.on('error', () => resolve(null));
        upstream.on('end', () => {
          try {
            const body = Buffer.concat(chunks);
            const json = upstream.headers['content-encoding'] === 'gzip' ? gunzipSync(body) : body;
            resolve(JSON.parse(json.toString()) as Packument);
          } catch {
            resolve(null);
          }
        });
      },
    ).on('error', () => resolve(null));
  });
}

// Serve local packages as an overlay on the real upstream packument (like the
// production registry bridge): upstream versions, times, and dist-tags stay
// visible, the local version is injected (winning over a published version
// with the same number), and `latest` points at it. Projects that already
// reference PUBLISHED Vite+ versions in their committed lockfiles (ecosystem
// repos on a previous release) can then still verify those entries, e.g.
// pnpm's time-based resolution policies need `time` for every lockfile entry.
// Falls back to the local-only packument when upstream is unreachable or the
// package has never been published.
const mergedPackuments = new Map<string, Packument>();

async function resolveLocalPackument(name: string): Promise<Packument> {
  const local = localPackuments.get(name) as Packument;
  const cached = mergedPackuments.get(name);
  if (cached) {
    return cached;
  }
  const upstream = await fetchUpstreamPackument(name);
  const localVersion = local['dist-tags'].latest;
  const merged: Packument = upstream
    ? {
        ...upstream,
        name,
        'dist-tags': { ...upstream['dist-tags'], ...local['dist-tags'] },
        versions: { ...upstream.versions, ...local.versions },
        // Keep upstream created/modified; only the local version's publish
        // time is ours.
        time: { ...local.time, ...upstream.time, [localVersion]: LOCAL_PACKAGE_TIME },
      }
    : local;
  mergedPackuments.set(name, merged);
  return merged;
}

// Stream the upstream response byte-for-byte. Unlike `fetch`, `https` does not
// auto-decompress, so the tarball reaches the client exactly as the registry
// served it (content-encoding and all). bun verifies tarball integrity and
// rejects any re-encoded body, so faithful streaming is what lets bun installs
// work through the proxy (pnpm fetches tarballs from the upstream URL directly,
// so it was never affected).
function copyHeaders(
  from: IncomingMessage['headers'],
  to: OutgoingHttpHeaders,
  names: readonly string[],
): void {
  for (const name of names) {
    if (from[name] !== undefined) {
      to[name] = from[name];
    }
  }
}

function proxyToUpstream(req: IncomingMessage, res: ServerResponse): void {
  // Stream errors can fire after headers are already sent (e.g. the upstream
  // connection resets mid-tarball), so guard against a second writeHead.
  const fail = (error: Error) => {
    if (!res.headersSent) {
      res.writeHead(502);
    }
    res.end(`proxy error: ${error.message}`);
  };
  // Forward the original method and body: npm-based clients POST to registry
  // endpoints too (e.g. the audit bulk advisories). The body can only be
  // streamed into the FIRST upstream request, so redirects are followed only
  // for body-less methods; a redirect on anything else is forwarded to the
  // client as-is (its `location` header survives below).
  const method = req.method ?? 'GET';
  const bodyless = method === 'GET' || method === 'HEAD';
  const fetchUrl = (url: string, redirectsLeft: number) => {
    // Tarballs: `accept-encoding: identity` keeps mirrors/CDNs from wrapping
    // them in an extra content-encoding layer, which some package managers
    // cannot unwrap when verifying/extracting. Metadata: honor the client's
    // own accept-encoding: the body is streamed through verbatim with its
    // content-encoding header, so it must be exactly what the client can
    // decode (package managers ask for gzip, which keeps huge packuments like
    // vite/typescript fast; vp's Rust registry client asks for identity).
    const headers: OutgoingHttpHeaders = {
      accept: req.headers.accept ?? 'application/json',
      'accept-encoding': req.url?.endsWith('.tgz')
        ? 'identity'
        : (req.headers['accept-encoding'] ?? 'identity'),
    };
    copyHeaders(req.headers, headers, ['content-type', 'content-length']);
    const upstreamRequest = httpsRequest(
      url,
      { method, headers, agent: upstreamAgent },
      (upstream) => {
        const status = upstream.statusCode ?? 502;
        if (
          status >= 300 &&
          status < 400 &&
          upstream.headers.location &&
          redirectsLeft > 0 &&
          bodyless
        ) {
          // Draining can still emit 'error' (e.g. the socket resets mid-redirect),
          // so guard it here too — otherwise it's uncaught and crashes the server.
          upstream.on('error', fail);
          upstream.resume();
          fetchUrl(new URL(upstream.headers.location, url).toString(), redirectsLeft - 1);
          return;
        }
        const responseHeaders: OutgoingHttpHeaders = {};
        // Forward `location` too, so a 3xx we stop following (or one with no
        // location to follow) still reaches the client with its redirect target.
        copyHeaders(upstream.headers, responseHeaders, [
          'content-type',
          'content-encoding',
          'content-length',
          'location',
        ]);
        // Default a missing content-type (parity with the prior fetch-based proxy)
        // so clients that key off it still recognize a proxied tarball.
        responseHeaders['content-type'] ??= 'application/octet-stream';
        res.writeHead(status, responseHeaders);
        // `pipe` does not forward source errors, so listen on the response stream
        // directly; otherwise a mid-stream upstream error is uncaught and crashes
        // the mock server.
        upstream.on('error', fail);
        upstream.pipe(res);
      },
    );
    upstreamRequest.on('error', fail);
    if (bodyless) {
      upstreamRequest.end();
    } else {
      req.pipe(upstreamRequest);
    }
  };
  fetchUrl(`${UPSTREAM_REGISTRY}${req.url ?? '/'}`, 5);
}

const server = createServer(async (req, res) => {
  const key = decodeURIComponent(req.url ?? '/').replace(/^\/+/, '');
  if (localPackuments.has(key) || Object.hasOwn(manifest, key)) {
    const address = server.address();
    const registry =
      address && typeof address !== 'string' ? `http://127.0.0.1:${address.port}` : '';
    const packument = localPackuments.has(key) ? await resolveLocalPackument(key) : manifest[key];
    res.writeHead(200, { 'content-type': 'application/json' });
    res.end(serializeForRegistry(packument, registry));
    return;
  }
  const tarMatch = key.match(/\/-\/([^/]+\.tgz)$/);
  if (tarMatch) {
    try {
      // Async read: local tarballs run tens of MB, and a synchronous read
      // would stall the proxied requests of the same in-flight install.
      const bytes = await readFile(
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
  // packages, etc.) to the upstream registry. Keeps the local packages in
  // charge while letting everything else (package-manager download, real
  // dependencies) resolve normally.
  proxyToUpstream(req, res);
});

// Registry env for every package manager, each of which reads its own
// spelling: npm and bun honor NPM_CONFIG_REGISTRY, pnpm >= 10.6 only reads
// PNPM_CONFIG_* (older pnpm read the lowercase npm_config_* form), and Yarn
// Berry only reads YARN_-prefixed settings and refuses plain-http registries
// unless the host is whitelisted.
//
// Yarn Berry persists packuments (with our ephemeral-port tarball URLs) in
// its global folder, and bun's install cache trusts name@version without
// refetching, so a later run would reuse dead URLs or stale local package
// bytes. Give each run throwaway caches instead.
function buildRegistryEnv(registry: string): Record<string, string> {
  // In a proxied environment (HTTP_PROXY/HTTPS_PROXY without a loopback
  // no-proxy entry), package managers would send the 127.0.0.1 registry
  // requests through the proxy and never reach this server; extend the
  // no-proxy list so loopback traffic always goes direct.
  const noProxy = [process.env.NO_PROXY ?? process.env.no_proxy, '127.0.0.1']
    .filter(Boolean)
    .join(',');
  return {
    NPM_CONFIG_REGISTRY: registry,
    npm_config_registry: registry,
    PNPM_CONFIG_REGISTRY: registry,
    YARN_NPM_REGISTRY_SERVER: registry,
    YARN_UNSAFE_HTTP_WHITELIST: '127.0.0.1',
    NO_PROXY: noProxy,
    no_proxy: noProxy,
    NPM_CONFIG_NOPROXY: noProxy,
    npm_config_noproxy: noProxy,
    YARN_GLOBAL_FOLDER: mkdtempSync(path.join(tmpdir(), 'vp-local-registry-yarn-')),
    BUN_INSTALL_CACHE_DIR: mkdtempSync(path.join(tmpdir(), 'vp-local-registry-bun-')),
  };
}

function cleanupRegistryEnv(env: Record<string, string>): void {
  rmSync(env.YARN_GLOBAL_FOLDER, { recursive: true, force: true });
  rmSync(env.BUN_INSTALL_CACHE_DIR, { recursive: true, force: true });
  if (packedDir) {
    rmSync(packedDir, { recursive: true, force: true });
  }
}

server.listen(0, '127.0.0.1', () => {
  const address = server.address();
  if (!address || typeof address === 'string') {
    console.error('local-npm-registry: failed to bind');
    process.exit(1);
  }
  const registry = `http://127.0.0.1:${address.port}`;
  const registryEnv = buildRegistryEnv(registry);

  if (serve) {
    // First line is machine-readable so callers (e.g. patch-project.ts) can
    // spawn `--serve` and read the URL and env; the rest is copy-paste for
    // humans.
    const lines = [
      JSON.stringify({ registry, env: registryEnv }),
      '',
      `# serving local packages${packagesDir ? ` from ${packagesDir}` : ''}`,
      ...[...localPackuments.keys(), ...Object.keys(manifest)].map((name) => `#   ${name}`),
      '# run commands against it with:',
      ...Object.entries(registryEnv).map(([key, value]) => `export ${key}=${value}`),
    ];
    process.stdout.write(`${lines.join('\n')}\n`);
    const shutdown = () => {
      cleanupRegistryEnv(registryEnv);
      // Exit without waiting for `server.close()`: idle keep-alive
      // connections from package managers would delay the callback
      // indefinitely, leaking the process.
      process.exit(0);
    };
    process.on('SIGINT', shutdown);
    process.on('SIGTERM', shutdown);
    return;
  }

  const [cmd, ...cmdArgs] = command as [string, ...string[]];
  const child = spawn(cmd, cmdArgs, {
    env: { ...process.env, ...registryEnv },
    stdio: 'inherit',
  });
  child.on('exit', (code, signal) => {
    cleanupRegistryEnv(registryEnv);
    const exitCode = code ?? (signal ? 128 : 0);
    server.close(() => process.exit(exitCode));
  });
});

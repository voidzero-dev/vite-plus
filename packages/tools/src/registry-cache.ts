// Run-scoped response storage for snapshot registries. Each case keeps its
// own package-manager caches; only successful upstream GETs are
// shared. Atomic publication and a directory lock also support nextest workers.
import { createHash, randomUUID } from 'node:crypto';
import { appendFileSync } from 'node:fs';
import { mkdir, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { request as httpRequest, type IncomingMessage } from 'node:http';
import { Agent, request as httpsRequest } from 'node:https';
import path from 'node:path';
import { setTimeout } from 'node:timers/promises';
import { brotliDecompressSync, gunzipSync, inflateSync } from 'node:zlib';

interface Response {
  status: number;
  contentType: string;
  body: Buffer;
}

const tarballPrefix = '/-/snapshot-tarball/';

export function registryTarballUrl(url: string, registry: string): string {
  const parsed = new URL(url);
  if (['registry.npmjs.org', 'registry.yarnpkg.com'].includes(parsed.hostname)) {
    return `${registry}${parsed.pathname}${parsed.search}`;
  }
  return `${registry}${tarballPrefix}${Buffer.from(url).toString('base64url')}`;
}

export function upstreamUrl(requestPath: string, upstream: string): string {
  if (requestPath.startsWith(tarballPrefix)) {
    const url = new URL(
      Buffer.from(requestPath.slice(tarballPrefix.length), 'base64url').toString(),
    );
    if (url.protocol !== 'https:') {
      throw new Error('Snapshot tarballs must use HTTPS');
    }
    return url.href;
  }
  return `${upstream}${requestPath}`;
}

// Rewrite only dist.tarball fields. Other URLs (readmes, repository links,
// provenance, etc.) must retain their meaning. Tarball bytes stay unchanged.
export function rewriteTarballs(value: unknown, registry: string): unknown {
  if (!value || typeof value !== 'object') {
    return value;
  }
  const object = value as Record<string, unknown>;
  const dist = object.dist as Record<string, unknown> | undefined;
  if (dist && typeof dist.tarball === 'string' && dist.tarball.startsWith('https://')) {
    dist.tarball = registryTarballUrl(dist.tarball, registry);
  }
  if (object.versions && typeof object.versions === 'object') {
    for (const version of Object.values(object.versions)) {
      rewriteTarballs(version, registry);
    }
  }
  return value;
}

export class RegistryCache {
  private readonly agent = new Agent({ keepAlive: true, maxSockets: 64 });
  private readonly pending = new Map<string, Promise<Response>>();

  private readonly directory: string;
  private readonly enabled: boolean;
  private readonly logFile?: string;
  private readonly timeoutMs: number;
  private readonly idleTimeoutMs: number;

  constructor(
    directory: string,
    enabled = true,
    logFile?: string,
    timeoutMs = 60_000,
    idleTimeoutMs = 15_000,
  ) {
    this.directory = directory;
    this.enabled = enabled;
    this.logFile = logFile;
    this.timeoutMs = timeoutMs;
    this.idleTimeoutMs = idleTimeoutMs;
  }

  record(event: string, url: string): void {
    if (this.logFile) {
      appendFileSync(this.logFile, `${JSON.stringify({ event, url })}\n`);
    }
  }

  // Prepare large tool archives before snapshot command deadlines start.
  // Only HTTP response bytes are shared; installations remain case-local.
  async prewarm(url: string): Promise<void> {
    for (let attempt = 0; ; attempt++) {
      try {
        const response = await this.get(url, 'application/octet-stream');
        if (response.status !== 200) {
          throw new Error(`Cannot prepare registry response: ${url} (HTTP ${response.status})`);
        }
        return;
      } catch (error) {
        if (attempt === 2) {
          throw error;
        }
        await setTimeout(1000 * 2 ** attempt);
      }
    }
  }

  async get(url: string, accept: string): Promise<Response> {
    this.record('request', url);
    if (!this.enabled) {
      return this.download(url, accept);
    }
    // Full and abbreviated packuments are different representations.
    const representation = new URL(url).pathname.endsWith('.tgz') ? 'tarball' : accept;
    const key = createHash('sha256')
      .update(JSON.stringify([url, representation]))
      .digest('hex');
    const pending = this.pending.get(key);
    if (pending) {
      this.record('hit', url);
      return pending;
    }
    const response = this.load(key, url, accept);
    this.pending.set(key, response);
    try {
      return await response;
    } finally {
      this.pending.delete(key);
    }
  }

  private async load(key: string, url: string, accept: string): Promise<Response> {
    await mkdir(this.directory, { recursive: true });
    const file = path.join(this.directory, key);
    const lock = `${file}.lock`;
    const deadline = Date.now() + 120_000;
    for (;;) {
      try {
        const bytes = await readFile(file);
        const separator = bytes.indexOf(10);
        const header = JSON.parse(bytes.subarray(0, separator).toString()) as Omit<
          Response,
          'body'
        >;
        this.record('hit', url);
        return { ...header, body: bytes.subarray(separator + 1) };
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== 'ENOENT') {
          throw error;
        }
      }
      try {
        await mkdir(lock);
        break;
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== 'EEXIST') {
          throw error;
        }
        if (Date.now() >= deadline) {
          throw new Error(`Timed out waiting for registry response: ${url}`, { cause: error });
        }
        await setTimeout(25);
      }
    }
    const temporary = `${file}.${randomUUID()}.tmp`;
    try {
      // Another worker may have published between our read and mkdir.
      try {
        const bytes = await readFile(file);
        const separator = bytes.indexOf(10);
        const header = JSON.parse(bytes.subarray(0, separator).toString()) as Omit<
          Response,
          'body'
        >;
        this.record('hit', url);
        return { ...header, body: bytes.subarray(separator + 1) };
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== 'ENOENT') {
          throw error;
        }
      }
      const response = await this.download(url, accept);
      if (response.status === 200) {
        await writeFile(
          temporary,
          Buffer.concat([
            Buffer.from(
              `${JSON.stringify({ status: response.status, contentType: response.contentType })}\n`,
            ),
            response.body,
          ]),
        );
        await rename(temporary, file);
      }
      return response;
    } finally {
      await rm(temporary, { force: true });
      await rm(lock, { recursive: true, force: true });
    }
  }

  private async download(url: string, accept: string, redirects = 5): Promise<Response> {
    this.record('upstream', url);
    return new Promise((resolve, reject) => {
      const stalled = new AbortController();
      let idleTimer: ReturnType<typeof globalThis.setTimeout> | undefined;
      const progress = () => {
        clearTimeout(idleTimer);
        idleTimer = globalThis.setTimeout(() => {
          this.record('upstream-stalled', url);
          stalled.abort(new Error('Registry request stalled'));
        }, this.idleTimeoutMs);
      };
      const request = url.startsWith('https:') ? httpsRequest : httpRequest;
      const req = request(
        url,
        {
          headers: {
            accept,
            'accept-encoding': new URL(url).pathname.endsWith('.tgz') ? 'identity' : 'gzip',
          },
          agent: url.startsWith('https:') ? this.agent : undefined,
          // Bound the whole request, including connection setup and responses
          // that keep sending data. Release the cache lock before a case times out.
          signal: AbortSignal.any([AbortSignal.timeout(this.timeoutMs), stalled.signal]),
        },
        (res: IncomingMessage) => {
          progress();
          res.on('error', reject);
          const status = res.statusCode ?? 502;
          if (status >= 300 && status < 400 && res.headers.location) {
            res.resume();
            if (redirects === 0) {
              reject(new Error(`Too many registry redirects: ${url}`));
            } else {
              resolve(
                this.download(new URL(res.headers.location, url).href, accept, redirects - 1),
              );
            }
            return;
          }
          const chunks: Buffer[] = [];
          res.on('data', (chunk: Buffer) => {
            progress();
            chunks.push(chunk);
          });
          res.on('end', () => {
            try {
              let body = Buffer.concat(chunks);
              switch (res.headers['content-encoding']) {
                case 'gzip':
                  body = gunzipSync(body);
                  break;
                case 'br':
                  body = brotliDecompressSync(body);
                  break;
                case 'deflate':
                  body = inflateSync(body);
                  break;
              }
              resolve({
                status,
                contentType: res.headers['content-type'] ?? 'application/octet-stream',
                body,
              });
            } catch (error) {
              reject(error);
            }
          });
        },
      );
      // Start before DNS/connect, unlike a socket timeout. Return a failed
      // request promptly so the client's retries fit inside the case deadline.
      progress();
      req.once('close', () => clearTimeout(idleTimer));
      req.on('error', reject);
      req.end();
    });
  }
}

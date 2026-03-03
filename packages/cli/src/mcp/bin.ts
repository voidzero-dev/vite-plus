import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

interface DocPage {
  slug: string;
  relativePath: string;
  title: string;
  content: string;
}

interface SearchResult {
  slug: string;
  title: string;
  snippet: string;
  score: number;
}

interface JsonRpcRequest {
  jsonrpc: string;
  id?: number | string | null;
  method: string;
  params?: Record<string, unknown>;
}

interface DocIndex {
  pages: DocPage[];
  byAlias: Map<string, DocPage>;
}

const PROTOCOL_VERSION = '2024-11-05';

const TOOLS = [
  {
    name: 'list_pages',
    description: 'List all Vite+ documentation pages with their slugs and titles',
    inputSchema: { type: 'object' as const, properties: {} },
  },
  {
    name: 'get_page',
    description: 'Get the full content of a Vite+ documentation page by slug',
    inputSchema: {
      type: 'object' as const,
      properties: {
        slug: {
          type: 'string',
          description: "Page slug, e.g. 'vite/guide/cli' or 'cli'",
        },
      },
      required: ['slug'],
    },
  },
  {
    name: 'search_docs',
    description:
      'Search Vite+ documentation by keyword query. Returns top 5 matching pages with snippets.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        query: {
          type: 'string',
          description: "Search query, e.g. 'dev server' or 'testing'",
        },
      },
      required: ['query'],
    },
  },
];

function findPackageRoot(from: string): string {
  let dir = from;
  while (true) {
    if (existsSync(join(dir, 'package.json'))) {
      return dir;
    }
    const parent = dirname(dir);
    if (parent === dir) {
      break;
    }
    dir = parent;
  }
  throw new Error('Could not find package.json from: ' + from);
}

function readPackageVersion(pkgRoot: string): string {
  const raw = readFileSync(join(pkgRoot, 'package.json'), 'utf8');
  const pkg = JSON.parse(raw) as { version?: string };
  return pkg.version ?? '0.0.0';
}

function resolveDocsDir(pkgRoot: string): string {
  const bundledDocsDir = join(pkgRoot, 'skills', 'vite-plus', 'docs');
  if (existsSync(bundledDocsDir)) {
    return bundledDocsDir;
  }

  const workspaceDocsDir = join(pkgRoot, '..', '..', 'docs');
  if (existsSync(workspaceDocsDir)) {
    return workspaceDocsDir;
  }

  throw new Error(`Vite+ docs directory not found. Expected bundled docs at: ${bundledDocsDir}`);
}

function collectMarkdownFiles(rootDir: string, relativeDir = ''): string[] {
  const currentDir = join(rootDir, relativeDir);
  const entries = readdirSync(currentDir, { withFileTypes: true });
  const files: string[] = [];

  for (const entry of entries) {
    const relPath = relativeDir ? `${relativeDir}/${entry.name}` : entry.name;
    if (entry.isDirectory()) {
      if (entry.name === 'node_modules') {
        continue;
      }
      files.push(...collectMarkdownFiles(rootDir, relPath));
      continue;
    }
    if (entry.isFile() && entry.name.endsWith('.md')) {
      files.push(relPath);
    }
  }

  // eslint-disable-next-line unicorn/no-array-sort -- deterministic ordering for stable alias resolution
  files.sort();
  return files;
}

function normalizeDocId(value: string): string {
  const normalized = value
    .trim()
    .replace(/\\/g, '/')
    .replace(/^docs\//, '')
    .replace(/\.md$/, '')
    .replace(/\/index$/, '')
    .replace(/^\/+|\/+$/g, '');
  return normalized || 'index';
}

function createSlug(relativePath: string): string {
  const withoutExt = relativePath.replace(/\.md$/, '');
  if (withoutExt === 'index') {
    return 'index';
  }
  if (withoutExt.endsWith('/index')) {
    return withoutExt.slice(0, -'/index'.length);
  }
  return withoutExt;
}

function buildAliases(page: DocPage, basenameCounts: Map<string, number>): string[] {
  const aliases = new Set<string>();
  const relativeNoExt = page.relativePath.replace(/\.md$/, '');
  const flatSlug = page.slug.replaceAll('/', '-');
  const baseName = page.slug.split('/').at(-1);

  aliases.add(page.slug);
  aliases.add(`${page.slug}.md`);
  aliases.add(relativeNoExt);
  aliases.add(page.relativePath);
  aliases.add(`docs/${relativeNoExt}`);
  aliases.add(`docs/${page.relativePath}`);
  aliases.add(flatSlug);
  aliases.add(`${flatSlug}.md`);

  if (baseName && (basenameCounts.get(baseName) ?? 0) === 1) {
    aliases.add(baseName);
    aliases.add(`${baseName}.md`);
  }

  return [...aliases];
}

function loadDocs(pkgRoot: string): DocIndex {
  const docsDir = resolveDocsDir(pkgRoot);
  const files = collectMarkdownFiles(docsDir);
  const pages: DocPage[] = files.map((relativePath) => {
    const raw = readFileSync(join(docsDir, relativePath), 'utf8');
    const content = raw.replace(/^---\n[\s\S]*?\n---\n/, '');
    const titleMatch = content.match(/^#\s+(.+)/m);
    const slug = createSlug(relativePath);
    const title = titleMatch ? titleMatch[1].trim() : slug;
    return { slug, relativePath, title, content };
  });

  const basenameCounts = new Map<string, number>();
  for (const page of pages) {
    const baseName = page.slug.split('/').at(-1);
    if (!baseName) {
      continue;
    }
    basenameCounts.set(baseName, (basenameCounts.get(baseName) ?? 0) + 1);
  }

  const aliasCounts = new Map<string, number>();
  const aliasSources = new Map<DocPage, string[]>();
  for (const page of pages) {
    const aliases = [...new Set(buildAliases(page, basenameCounts).map(normalizeDocId))];
    aliasSources.set(page, aliases);
    for (const alias of aliases) {
      aliasCounts.set(alias, (aliasCounts.get(alias) ?? 0) + 1);
    }
  }

  const byAlias = new Map<string, DocPage>();
  for (const page of pages) {
    for (const alias of aliasSources.get(page) ?? []) {
      if ((aliasCounts.get(alias) ?? 0) !== 1) {
        continue;
      }
      byAlias.set(alias, page);
    }
  }

  return { pages, byAlias };
}

function searchDocs(pages: DocPage[], query: string): SearchResult[] {
  const terms = query
    .toLowerCase()
    .split(/\s+/)
    .filter((term) => term.length > 0);
  if (terms.length === 0) {
    return [];
  }

  const scored: SearchResult[] = [];
  for (const page of pages) {
    const titleLower = page.title.toLowerCase();
    const contentLower = page.content.toLowerCase();
    let score = 0;
    let firstMatchIndex = -1;

    for (const term of terms) {
      let idx = 0;
      while ((idx = titleLower.indexOf(term, idx)) !== -1) {
        score += 3;
        idx += term.length;
      }

      idx = 0;
      while ((idx = contentLower.indexOf(term, idx)) !== -1) {
        score += 1;
        if (firstMatchIndex === -1) {
          firstMatchIndex = idx;
        }
        idx += term.length;
      }
    }

    if (score === 0) {
      continue;
    }

    let snippet: string;
    if (firstMatchIndex !== -1) {
      const start = Math.max(0, firstMatchIndex - 80);
      const end = Math.min(page.content.length, firstMatchIndex + 120);
      snippet =
        (start > 0 ? '...' : '') +
        page.content.slice(start, end).trim() +
        (end < page.content.length ? '...' : '');
    } else {
      snippet = page.content.slice(0, 200).trim() + '...';
    }

    scored.push({ slug: page.slug, title: page.title, snippet, score });
  }

  scored.sort((a, b) => b.score - a.score);
  return scored.slice(0, 5);
}

function resolvePageBySlug(index: DocIndex, rawSlug: string): DocPage | undefined {
  return index.byAlias.get(normalizeDocId(rawSlug));
}

function makeErrorResponse(id: number | string | null, code: number, message: string): object {
  return {
    jsonrpc: '2.0',
    id,
    error: { code, message },
  };
}

function handleRequest(index: DocIndex, serverVersion: string, req: JsonRpcRequest): object | null {
  const { method, id, params } = req;

  if (method === 'initialize') {
    return {
      jsonrpc: '2.0',
      id: id ?? null,
      result: {
        protocolVersion: PROTOCOL_VERSION,
        capabilities: { tools: {} },
        serverInfo: { name: 'vite-plus', version: serverVersion },
      },
    };
  }

  if (method === 'notifications/initialized' || method === '$/cancelRequest') {
    return null;
  }

  if (method === 'ping') {
    return { jsonrpc: '2.0', id: id ?? null, result: {} };
  }

  if (method === 'tools/list') {
    return { jsonrpc: '2.0', id: id ?? null, result: { tools: TOOLS } };
  }

  if (method === 'tools/call') {
    const toolName = (params?.name as string | undefined) ?? '';
    const toolArgs = (params?.arguments as Record<string, unknown> | undefined) ?? {};

    if (toolName === 'list_pages') {
      const list = index.pages.map((page) => ({
        slug: page.slug,
        title: page.title,
        path: `docs/${page.relativePath}`,
      }));
      return {
        jsonrpc: '2.0',
        id: id ?? null,
        result: { content: [{ type: 'text', text: JSON.stringify(list, null, 2) }] },
      };
    }

    if (toolName === 'get_page') {
      const slug = toolArgs.slug;
      if (typeof slug !== 'string' || slug.trim().length === 0) {
        return {
          jsonrpc: '2.0',
          id: id ?? null,
          result: {
            content: [{ type: 'text', text: 'Missing required string argument: slug' }],
            isError: true,
          },
        };
      }
      const page = resolvePageBySlug(index, slug);
      if (!page) {
        return {
          jsonrpc: '2.0',
          id: id ?? null,
          result: {
            content: [{ type: 'text', text: `Page not found: ${slug}` }],
            isError: true,
          },
        };
      }
      return {
        jsonrpc: '2.0',
        id: id ?? null,
        result: { content: [{ type: 'text', text: page.content }] },
      };
    }

    if (toolName === 'search_docs') {
      const query = toolArgs.query;
      if (typeof query !== 'string' || query.trim().length === 0) {
        return {
          jsonrpc: '2.0',
          id: id ?? null,
          result: {
            content: [{ type: 'text', text: 'Missing required string argument: query' }],
            isError: true,
          },
        };
      }
      const results = searchDocs(index.pages, query);
      return {
        jsonrpc: '2.0',
        id: id ?? null,
        result: { content: [{ type: 'text', text: JSON.stringify(results, null, 2) }] },
      };
    }

    if (id === undefined) {
      return null;
    }
    return makeErrorResponse(id, -32601, `Unknown tool: ${toolName}`);
  }

  if (id === undefined) {
    return null;
  }
  return makeErrorResponse(id, -32601, `Unknown method: ${method}`);
}

function writeMessage(payload: object): void {
  const body = JSON.stringify(payload);
  const header = `Content-Length: ${Buffer.byteLength(body, 'utf8')}\r\n\r\n`;
  process.stdout.write(header + body);
}

function findHeadersBoundary(buffer: Buffer): { end: number; separatorLength: 2 | 4 } | null {
  const crlf = buffer.indexOf('\r\n\r\n');
  const lf = buffer.indexOf('\n\n');

  if (crlf === -1 && lf === -1) {
    return null;
  }
  if (crlf !== -1 && (lf === -1 || crlf < lf)) {
    return { end: crlf, separatorLength: 4 };
  }
  return { end: lf, separatorLength: 2 };
}

function parseContentLength(rawHeaders: string): number | null {
  const lines = rawHeaders.split(/\r?\n/);
  for (const line of lines) {
    const match = line.match(/^content-length:\s*(\d+)\s*$/i);
    if (match) {
      const value = Number.parseInt(match[1], 10);
      if (Number.isSafeInteger(value) && value >= 0) {
        return value;
      }
      return null;
    }
  }
  return null;
}

function startStdioServer(index: DocIndex, serverVersion: string): void {
  let buffer = Buffer.alloc(0);

  process.stdin.on('error', () => {
    process.exit(1);
  });

  process.stdin.on('end', () => {
    process.exit(0);
  });

  process.stdin.on('data', (chunk: Buffer) => {
    buffer = Buffer.concat([buffer, chunk]);

    while (true) {
      const boundary = findHeadersBoundary(buffer);
      if (!boundary) {
        break;
      }

      const headerText = buffer.subarray(0, boundary.end).toString('utf8');
      const contentLength = parseContentLength(headerText);
      const bodyStart = boundary.end + boundary.separatorLength;

      if (contentLength === null) {
        writeMessage(makeErrorResponse(null, -32600, 'Missing or invalid Content-Length header'));
        buffer = buffer.subarray(bodyStart);
        continue;
      }

      const bodyEnd = bodyStart + contentLength;
      if (buffer.length < bodyEnd) {
        break;
      }

      const body = buffer.subarray(bodyStart, bodyEnd).toString('utf8');
      buffer = buffer.subarray(bodyEnd);

      let request: JsonRpcRequest;
      try {
        request = JSON.parse(body) as JsonRpcRequest;
      } catch {
        writeMessage(makeErrorResponse(null, -32700, 'Parse error'));
        continue;
      }

      if (typeof request.method !== 'string') {
        writeMessage(
          makeErrorResponse(request.id ?? null, -32600, 'Invalid request: missing "method" field'),
        );
        continue;
      }

      const response = handleRequest(index, serverVersion, request);
      if (response !== null) {
        writeMessage(response);
      }
    }
  });
}

try {
  const packageRoot = findPackageRoot(dirname(fileURLToPath(import.meta.url)));
  const serverVersion = readPackageVersion(packageRoot);
  const docs = loadDocs(packageRoot);
  startStdioServer(docs, serverVersion);
} catch (err) {
  process.stderr.write(
    `[vite-plus mcp] Failed to start: ${err instanceof Error ? err.message : String(err)}\n`,
  );
  process.exit(1);
}

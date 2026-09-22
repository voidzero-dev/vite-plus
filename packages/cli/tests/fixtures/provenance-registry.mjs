import { appendFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';

const args = new Map();
for (let index = 2; index < process.argv.length; index += 2) {
  const name = process.argv[index];
  const value = process.argv[index + 1];
  if (!name?.startsWith('--') || value === undefined) {
    throw new Error(`Invalid argument at position ${index}: ${name ?? '<missing>'}`);
  }
  args.set(name.slice(2), value);
}

const portFile = args.get('port-file');
const logFile = args.get('log-file');
const mode = args.get('mode') ?? 'missing';
const version = args.get('version') ?? '9.9.9-provenance-test.1';
const rawContentType = args.get('raw-content-type') === 'true';

if (!portFile || !logFile) {
  throw new Error('--port-file and --log-file are required');
}
if (
  ![
    'missing',
    'malformed',
    'top-level-only',
    'dotted-top-level-key',
    'unsupported',
    'valid-v1',
    'valid-v0.2',
  ].includes(mode)
) {
  throw new Error(`Unsupported mode: ${mode}`);
}

writeFileSync(logFile, '');

function sendJson(response, status, body) {
  const json = JSON.stringify(body);
  response.writeHead(status, {
    'content-length': Buffer.byteLength(json),
    'content-type': rawContentType ? 'text/plain' : 'application/json',
  });
  response.end(json);
}

function platformMetadata(packageName, registryBase) {
  const metadata = {
    name: packageName,
    version,
    dist: {
      tarball: `${registryBase}/platform.tgz`,
      integrity: 'sha512-test-only',
      signatures: [{ keyid: 'registry-signature-is-not-provenance', sig: 'test-only' }],
    },
  };

  if (mode === 'malformed') {
    metadata.dist.attestations = { provenance: 'not-an-object' };
  } else if (mode === 'top-level-only') {
    metadata.attestations = {
      provenance: { predicateType: 'https://slsa.dev/provenance/v1' },
    };
  } else if (mode === 'dotted-top-level-key') {
    metadata['dist.attestations.provenance.predicateType'] = 'https://slsa.dev/provenance/v1';
  } else if (mode === 'unsupported') {
    metadata.dist.attestations = {
      provenance: { predicateType: 'https://example.test/provenance/v1' },
    };
  } else if (mode === 'valid-v1') {
    metadata.dist.attestations = {
      provenance: { predicateType: 'https://slsa.dev/provenance/v1' },
    };
  } else if (mode === 'valid-v0.2') {
    metadata.dist.attestations = {
      provenance: { predicateType: 'https://slsa.dev/provenance/v0.2' },
    };
  }

  return metadata;
}

const server = createServer((request, response) => {
  const url = new URL(request.url ?? '/', 'http://127.0.0.1');
  let decodedPath;
  try {
    decodedPath = decodeURIComponent(url.pathname);
  } catch {
    sendJson(response, 400, { error: 'invalid URL encoding' });
    return;
  }

  appendFileSync(logFile, `${JSON.stringify({ method: request.method, path: decodedPath })}\n`);
  const requestHost = request.headers.host ?? `127.0.0.1:${server.address().port}`;
  const registryBase = `http://${requestHost}`;

  if (decodedPath === `/vite-plus/${version}`) {
    sendJson(response, 200, {
      name: 'vite-plus',
      version,
      dist: {
        tarball: `${registryBase}/vite-plus.tgz`,
        integrity: 'sha512-test-only',
      },
    });
    return;
  }

  const platformMatch = decodedPath.match(
    new RegExp(`^/(@voidzero-dev/vite-plus-cli-[a-z0-9-]+)/${version.replaceAll('.', '\\.')}$`),
  );
  if (platformMatch) {
    sendJson(response, 200, platformMetadata(platformMatch[1], registryBase));
    return;
  }

  if (decodedPath === '/platform.tgz' || decodedPath === '/vite-plus.tgz') {
    response.writeHead(500, { 'content-type': 'text/plain' });
    response.end('The provenance gate must reject before requesting a tarball.\n');
    return;
  }

  sendJson(response, 404, { error: `No fixture response for ${decodedPath}` });
});

server.listen(0, '127.0.0.1', () => {
  const address = server.address();
  if (!address || typeof address === 'string') {
    throw new Error('Expected a TCP listener');
  }
  writeFileSync(portFile, String(address.port));
});

for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => server.close(() => process.exit(0)));
}

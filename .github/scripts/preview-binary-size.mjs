import { execFileSync } from 'node:child_process';
import { lstat, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { gzipSync } from 'node:zlib';

const marker = '<!-- preview-binary-size -->';
const repository = 'https://github.com/voidzero-dev/vite-plus';
const maxBytes = 128 * 1024 ** 2;

export const targets = [
  ['aarch64-apple-darwin', 'darwin-arm64', 'macOS ARM64'],
  ['x86_64-apple-darwin', 'darwin-x64', 'macOS x64'],
  ['aarch64-unknown-linux-gnu', 'linux-arm64-gnu', 'Linux ARM64 GNU'],
  ['aarch64-unknown-linux-musl', 'linux-arm64-musl', 'Linux ARM64 musl'],
  ['x86_64-unknown-linux-gnu', 'linux-x64-gnu', 'Linux x64 GNU'],
  ['x86_64-unknown-linux-musl', 'linux-x64-musl', 'Linux x64 musl'],
  ['x86_64-pc-windows-msvc', 'win32-x64-msvc', 'Windows x64'],
  ['aarch64-pc-windows-msvc', 'win32-arm64-msvc', 'Windows ARM64'],
];

export function binaryArtifacts() {
  return targets.flatMap(([target, platform, label]) => {
    const windows = platform.startsWith('win32');
    const archive = `vp-${target}.${windows ? 'zip' : 'tar.gz'}`;
    const cli = (name, member) => ({
      name: `${name} (${label})`,
      path: `vp-global-cli-${target}/${member}`,
      asset: archive,
      member,
    });
    return [
      cli('`vp`', windows ? 'vp.exe' : 'vp'),
      {
        name: `NAPI (${label})`,
        path: `vite-plus-native-${target}/vite-plus.${platform}.node`,
        package: `@voidzero-dev/vite-plus-${platform}`,
        member: `package/vite-plus.${platform}.node`,
      },
      ...(windows
        ? [
            cli('Trampoline', 'vp-shim.exe'),
            {
              name: `Installer (${label})`,
              path: `vp-setup-${target}/vp-setup.exe`,
              asset: `vp-setup-${target}.exe`,
            },
          ]
        : []),
    ];
  });
}

function releaseVersion(release) {
  if (release.draft || release.prerelease || !/^v\d+\.\d+\.\d+$/.test(release.tag_name)) {
    throw new Error('Expected a stable Vite+ release tag');
  }
  return release.tag_name.slice(1);
}

export function measureBinary(bytes) {
  if (bytes.length === 0 || bytes.length > maxBytes) {
    throw new Error('Binary is empty or exceeds the size limit');
  }
  return { raw: bytes.length, gzip: gzipSync(bytes, { level: 9 }).length };
}

async function download(url, { allowMissing = false } = {}) {
  const response = await fetch(url, { signal: AbortSignal.timeout(120_000) });
  if (allowMissing && response.status === 404) return null;
  if (!response.ok) throw new Error(`Download failed (${response.status}): ${url}`);
  const chunks = [];
  let size = 0;
  for await (const chunk of response.body) {
    size += chunk.length;
    if (size > maxBytes) throw new Error(`Download exceeds the size limit: ${url}`);
    chunks.push(chunk);
  }
  return Buffer.concat(chunks);
}

// Only release archives pass through tar/unzip. Preview binaries are read as
// bytes, never executed, loaded as addons, or installed as packages.
export function createReleaseLoader(release, directory) {
  const version = releaseVersion(release);
  const downloads = new Map();
  async function cachedDownload(url) {
    if (!downloads.has(url)) downloads.set(url, download(url));
    return downloads.get(url);
  }
  return async (artifact) => {
    let bytes;
    let zip = false;
    if (artifact.package) {
      const metadataBytes = await download(
        `https://registry.npmjs.org/${encodeURIComponent(artifact.package)}/${version}`,
        { allowMissing: true },
      );
      if (!metadataBytes) return null;
      const metadata = JSON.parse(metadataBytes.toString('utf8'));
      if (metadata.name !== artifact.package || metadata.version !== version) {
        throw new Error(`Unexpected npm package: ${artifact.package}@${version}`);
      }
      const url = new URL(metadata.dist.tarball);
      if (url.protocol !== 'https:' || url.host !== 'registry.npmjs.org') {
        throw new Error('Expected an npm registry tarball');
      }
      bytes = await cachedDownload(url.href);
    } else {
      if (!release.assets.some((asset) => asset.name === artifact.asset)) return null;
      bytes = await cachedDownload(
        `${repository}/releases/download/${release.tag_name}/${artifact.asset}`,
      );
      zip = artifact.asset.endsWith('.zip');
    }
    if (!artifact.member) return bytes;
    const options = { maxBuffer: maxBytes, timeout: 30_000 };
    if (zip) {
      const archive = join(directory, artifact.asset);
      await writeFile(archive, bytes);
      return execFileSync('unzip', ['-p', archive, artifact.member], options);
    }
    return execFileSync('tar', ['-xzOf', '-', artifact.member], { ...options, input: bytes });
  };
}

function formatSize(bytes, signed = false) {
  const absolute = Math.abs(bytes);
  const sign = bytes < 0 ? '-' : signed && bytes > 0 ? '+' : '';
  if (absolute >= 1024 ** 2) return `${sign}${(absolute / 1024 ** 2).toFixed(2)} MiB`;
  if (absolute >= 1024) return `${sign}${(absolute / 1024).toFixed(2)} KiB`;
  return `${sign}${absolute} B`;
}

function formatDelta(base, preview) {
  const delta = preview - base;
  return `${formatSize(delta, true)} (${delta > 0 ? '+' : ''}${((delta / base) * 100).toFixed(2)}%)`;
}

export function renderSizeReport({ release, headSha, runUrl, rows }) {
  releaseVersion(release);
  if (!/^[a-f0-9]{40}$/.test(headSha)) throw new Error('Invalid preview commit');
  const table = (format) => [
    '| Binary | Latest release | Preview | Change |',
    '| --- | ---: | ---: | ---: |',
    ...rows.map(({ name, baseline, preview }) =>
      baseline
        ? `| ${name} | ${formatSize(baseline[format])} | ${formatSize(preview[format])} | ${formatDelta(baseline[format], preview[format])} |`
        : `| ${name} | — | ${formatSize(preview[format])} | Not in release |`,
    ),
  ];
  return [
    marker,
    '',
    `### Preview binary sizes (\`${headSha.slice(0, 7)}\`)`,
    '',
    `Compared with the latest stable release, [\`${release.tag_name}\`](${repository}/releases/tag/${release.tag_name}). [Preview build](${runUrl}).`,
    'Each row measures one binary. Negative changes mean a smaller preview binary.',
    '',
    ...table('raw'),
    '',
    '<details>',
    '<summary>gzip -9 sizes</summary>',
    '',
    ...table('gzip'),
    '',
    '</details>',
    '',
    'Raw sizes are uncompressed file sizes. gzip sizes recompress each binary at level 9; they are not package or download archive sizes.',
    'The baseline uses published release files, including NAPI packages from npm at the same version. Build versions and toolchains can affect the difference.',
  ].join('\n');
}

export async function comparePreviewBinaries({
  release,
  artifactDirectory,
  headSha,
  runUrl,
  loadRelease,
}) {
  releaseVersion(release);
  const directory = await mkdtemp(join(tmpdir(), 'vp-release-size-'));
  try {
    loadRelease ??= createReleaseLoader(release, directory);
    const rows = [];
    for (const artifact of binaryArtifacts()) {
      const path = join(artifactDirectory, artifact.path);
      const info = await lstat(path);
      if (!info.isFile() || info.size === 0 || info.size > maxBytes) {
        throw new Error(`Expected a regular preview binary: ${artifact.path}`);
      }
      const preview = measureBinary(await readFile(path));
      const baseline = await loadRelease(artifact);
      rows.push({ name: artifact.name, preview, baseline: baseline && measureBinary(baseline) });
    }
    return renderSizeReport({ release, headSha, runUrl, rows });
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

export async function postSizeComment({ github, context, core, prNumber, body }) {
  const { data: pr } = await github.rest.pulls.get({ ...context.repo, pull_number: prNumber });
  if (
    pr.state !== 'open' ||
    pr.head.sha !== context.payload.workflow_run.head_sha ||
    !pr.labels.some((label) => label.name === 'preview-build')
  ) {
    core.info('The PR changed after this preview build; skipping its size comment.');
    return;
  }
  const comments = await github.paginate(github.rest.issues.listComments, {
    ...context.repo,
    issue_number: prNumber,
  });
  const existing = comments.find(
    (comment) => comment.user?.login === 'github-actions[bot]' && comment.body?.includes(marker),
  );
  if (existing) {
    await github.rest.issues.updateComment({ ...context.repo, comment_id: existing.id, body });
  } else {
    await github.rest.issues.createComment({ ...context.repo, issue_number: prNumber, body });
  }
}

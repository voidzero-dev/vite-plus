/// <reference types="node" />

import { spawnSync } from 'node:child_process';
import {
  appendFileSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { parseArgs, stripVTControlCharacters } from 'node:util';

type ToolName = 'vite' | 'vitest' | 'oxlint' | 'oxfmt' | 'tsdown';

type Tool = {
  commands: string[][];
  name: ToolName;
  packageName: string;
  title: string;
};

type ToolSnapshot = {
  help: string;
  version: string;
};

type Snapshot = {
  tools: Record<ToolName, ToolSnapshot>;
};

type VersionMetadata = Partial<
  Record<
    ToolName,
    {
      new: string;
      tag?: string;
    }
  >
>;

const ROOT = process.cwd();
const WORKSPACE_PATH = join(ROOT, 'pnpm-workspace.yaml');
// Leave room for five reports plus Markdown within GitHub's 65,536-character comment limit.
const MAX_DIFF_LENGTH = 12_000;
const TOOLS: Tool[] = [
  {
    // Vite+ mirrors the root command plus the build and preview option sets.
    commands: [['--help'], ['build', '--help'], ['preview', '--help']],
    name: 'vite',
    packageName: 'vite',
    title: 'Vite',
  },
  {
    commands: [['--help']],
    name: 'vitest',
    packageName: 'vitest',
    title: 'Vitest',
  },
  {
    commands: [['--help']],
    name: 'oxlint',
    packageName: 'oxlint',
    title: 'Oxlint',
  },
  {
    commands: [['--help']],
    name: 'oxfmt',
    packageName: 'oxfmt',
    title: 'Oxfmt',
  },
  {
    commands: [['--help']],
    name: 'tsdown',
    packageName: 'tsdown',
    title: 'tsdown',
  },
];

function readJson(filePath: string): unknown {
  return JSON.parse(readFileSync(filePath, 'utf8'));
}

function readToolVersion(tool: Tool, versions?: VersionMetadata): string {
  if (versions) {
    const change = versions[tool.name];
    const version = tool.name === 'vite' ? change?.tag?.replace(/^v/, '') : change?.new;
    if (!version) {
      throw new Error(`Upgrade metadata has no target version for ${tool.name}`);
    }
    return version;
  }

  if (tool.name === 'vite') {
    const pkg = readJson(join(ROOT, 'vite/packages/vite/package.json')) as { version?: unknown };
    if (typeof pkg.version !== 'string') {
      throw new TypeError('vite/packages/vite/package.json has no version');
    }
    return pkg.version;
  }

  const workspace = readFileSync(WORKSPACE_PATH, 'utf8');
  const match = new RegExp(`^  ${tool.name}: [=~^]?([^\\s#]+)`, 'm').exec(workspace);
  if (!match) {
    throw new Error(`Could not find ${tool.name} in the pnpm workspace catalog`);
  }
  return match[1];
}

function normalizeOutput(output: string, version: string): string {
  // Version banners change on every release but do not represent CLI option drift.
  return stripVTControlCharacters(output)
    .replaceAll('\r\n', '\n')
    .replaceAll(version, '<version>')
    .split('\n')
    .map((line) => line.trimEnd())
    .join('\n')
    .trimEnd();
}

function captureToolHelp(tool: Tool, version: string): string {
  return tool.commands
    .map((command) => {
      const result = spawnSync(
        'pnpm',
        ['--silent', 'dlx', `${tool.packageName}@${version}`, ...command],
        {
          encoding: 'utf8',
          env: {
            ...process.env,
            CI: '1',
            COLUMNS: '120',
            FORCE_COLOR: '0',
            NO_COLOR: '1',
            TERM: 'dumb',
          },
        },
      );
      if (result.status !== 0) {
        throw new Error(
          `Failed to capture ${tool.title} help (${command.join(' ')}):\n${result.stderr}`,
        );
      }
      return [`$ ${tool.packageName} ${command.join(' ')}`, result.stdout.trimEnd()].join('\n');
    })
    .join('\n\n');
}

function captureSnapshot(outputPath: string, versionsPath?: string): void {
  const versions = versionsPath ? (readJson(versionsPath) as VersionMetadata) : undefined;
  const tools = {} as Record<ToolName, ToolSnapshot>;
  for (const tool of TOOLS) {
    const version = readToolVersion(tool, versions);
    console.log(`Capturing ${tool.title} ${version} help...`);
    tools[tool.name] = {
      help: captureToolHelp(tool, version),
      version,
    };
  }

  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, `${JSON.stringify({ tools } satisfies Snapshot, null, 2)}\n`);
}

function createUnifiedDiff(tool: Tool, before: ToolSnapshot, after: ToolSnapshot): string {
  const tempDir = mkdtempSync(join(tmpdir(), 'vite-plus-cli-help-'));
  const beforePath = join(tempDir, 'before.txt');
  const afterPath = join(tempDir, 'after.txt');

  try {
    writeFileSync(beforePath, `${normalizeOutput(before.help, before.version)}\n`);
    writeFileSync(afterPath, `${normalizeOutput(after.help, after.version)}\n`);
    const result = spawnSync(
      'git',
      [
        'diff',
        '--no-index',
        '--no-color',
        '--no-ext-diff',
        '--unified=3',
        '--',
        beforePath,
        afterPath,
      ],
      { encoding: 'utf8' },
    );
    // `git diff --no-index` exits with 1 when it successfully finds differences.
    if (result.status !== 1) {
      throw new Error(`Failed to diff ${tool.title} help:\n${result.stderr}`);
    }
    const firstHunk = result.stdout.indexOf('@@');
    if (firstHunk === -1) {
      throw new Error(`Git produced no diff hunk for ${tool.title}`);
    }
    return [
      `--- ${tool.packageName}@${before.version}`,
      `+++ ${tool.packageName}@${after.version}`,
      result.stdout.slice(firstHunk).trimEnd(),
    ].join('\n');
  } finally {
    rmSync(tempDir, { force: true, recursive: true });
  }
}

function truncateDiff(diff: string): string {
  if (diff.length <= MAX_DIFF_LENGTH) {
    return diff;
  }
  return `${diff.slice(0, MAX_DIFF_LENGTH)}\n... diff truncated to fit in one GitHub comment ...`;
}

function hasHelpChanges(before: Snapshot, after: Snapshot): boolean {
  return TOOLS.some((tool) => {
    const previous = before.tools[tool.name];
    const current = after.tools[tool.name];
    return (
      previous.version !== current.version &&
      normalizeOutput(previous.help, previous.version) !==
        normalizeOutput(current.help, current.version)
    );
  });
}

function renderReport(before: Snapshot, after: Snapshot, hasChanges: boolean): string {
  const lines = [
    hasChanges
      ? '## ⚠️ Upstream CLI help changes detected'
      : '## ✅ No upstream CLI help changes detected',
    '',
    'Compared normalized `--help` output for the upstream CLIs mirrored by Vite+.',
    '',
  ];

  for (const tool of TOOLS) {
    const previous = before.tools[tool.name];
    const current = after.tools[tool.name];
    lines.push('<details>');

    if (previous.version === current.version) {
      lines.push(
        `<summary><strong>➖ ${tool.title}: no version update (${current.version})</strong></summary>`,
        '',
        'No version update was detected, so there is no CLI help diff.',
      );
    } else if (
      normalizeOutput(previous.help, previous.version) ===
      normalizeOutput(current.help, current.version)
    ) {
      lines.push(
        `<summary><strong>✅ ${tool.title}: no CLI help changes (${previous.version} → ${current.version})</strong></summary>`,
        '',
        'The version was updated, but the normalized CLI help output has no differences.',
      );
    } else {
      lines.push(
        `<summary><strong>⚠️ ${tool.title}: CLI help changed (${previous.version} → ${current.version})</strong></summary>`,
        '',
        '```diff',
        truncateDiff(createUnifiedDiff(tool, previous, current)),
        '```',
      );
    }

    lines.push('', '</details>', '');
  }

  return `${lines.join('\n').trimEnd()}\n`;
}

function generateReport(
  beforePath: string,
  afterPath: string,
  outputPath: string,
  githubOutputPath?: string,
): void {
  const before = readJson(beforePath) as Snapshot;
  const after = readJson(afterPath) as Snapshot;
  const hasChanges = hasHelpChanges(before, after);
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, renderReport(before, after, hasChanges));
  if (githubOutputPath) {
    appendFileSync(githubOutputPath, `has-changes=${hasChanges}\n`);
  }
  console.log(`Wrote CLI help report to ${outputPath}`);
}

const { positionals, values } = parseArgs({
  allowPositionals: true,
  options: {
    after: { type: 'string' },
    before: { type: 'string' },
    'github-output': { type: 'string' },
    output: { short: 'o', type: 'string' },
    versions: { type: 'string' },
  },
});
const [command] = positionals;

if (command === 'capture' && values.output) {
  captureSnapshot(values.output, values.versions);
} else if (command === 'report' && values.before && values.after && values.output) {
  generateReport(values.before, values.after, values.output, values['github-output']);
} else {
  throw new Error(
    'Usage: cli-help-diff.ts capture --output <file> [--versions <file>] | report --before <file> --after <file> --output <file> [--github-output <file>]',
  );
}

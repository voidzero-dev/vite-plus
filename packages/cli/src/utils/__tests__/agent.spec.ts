import fs from 'node:fs';
import fsPromises from 'node:fs/promises';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  detectExistingAgentTargetPath,
  replaceMarkedAgentInstructionsSection,
  resolveAgentTargetPaths,
  writeAgentInstructions,
} from '../agent.js';
import { pkgRoot } from '../path.js';

type MockNode =
  | { kind: 'dir' }
  | { kind: 'file'; content: string }
  | { kind: 'symlink'; target: string };

class InMemoryFs {
  private readonly nodes = new Map<string, MockNode>();

  constructor() {
    this.ensureDirectory(path.parse(process.cwd()).root);
  }

  existsSync(filePath: fs.PathLike): boolean {
    return this.nodes.has(this.normalize(filePath));
  }

  lstatSync(filePath: fs.PathLike): fs.Stats {
    const node = this.getNode(filePath);
    return {
      isSymbolicLink: () => node.kind === 'symlink',
    } as fs.Stats;
  }

  async lstat(filePath: fs.PathLike): Promise<fs.Stats> {
    return this.lstatSync(filePath);
  }

  async mkdir(dirPath: fs.PathLike, options: { recursive: true }): Promise<void> {
    if (!options.recursive) {
      throw new Error('Only recursive mkdir is supported in tests');
    }
    this.ensureDirectory(this.normalize(dirPath));
  }

  async readFile(filePath: fs.PathLike): Promise<string> {
    const resolvedPath = this.resolvePath(filePath);
    const node = this.nodes.get(resolvedPath);
    if (!node || node.kind !== 'file') {
      throw new Error(`ENOENT: no such file "${String(filePath)}"`);
    }
    return node.content;
  }

  async writeFile(filePath: fs.PathLike, content: string): Promise<void> {
    const resolvedPath = this.resolvePathForWrite(filePath);
    this.ensureDirectory(path.dirname(resolvedPath));
    this.nodes.set(resolvedPath, { kind: 'file', content });
  }

  async appendFile(filePath: fs.PathLike, content: string): Promise<void> {
    const resolvedPath = this.resolvePathForWrite(filePath);
    this.ensureDirectory(path.dirname(resolvedPath));
    const existing = this.nodes.get(resolvedPath);
    if (!existing) {
      this.nodes.set(resolvedPath, { kind: 'file', content });
      return;
    }
    if (existing.kind !== 'file') {
      throw new Error(`EISDIR: cannot append to non-file "${String(filePath)}"`);
    }
    existing.content += content;
  }

  async realpath(filePath: fs.PathLike): Promise<string> {
    return this.resolvePath(filePath);
  }

  async symlink(target: string, filePath: fs.PathLike): Promise<void> {
    const normalizedPath = this.normalize(filePath);
    this.ensureDirectory(path.dirname(normalizedPath));
    this.nodes.set(normalizedPath, { kind: 'symlink', target });
  }

  async readlink(filePath: fs.PathLike): Promise<string> {
    const node = this.getNode(filePath);
    if (node.kind !== 'symlink') {
      throw new Error(`EINVAL: not a symlink "${String(filePath)}"`);
    }
    return node.target;
  }

  async unlink(filePath: fs.PathLike): Promise<void> {
    const normalizedPath = this.normalize(filePath);
    if (!this.nodes.has(normalizedPath)) {
      throw new Error(`ENOENT: no such file "${String(filePath)}"`);
    }
    this.nodes.delete(normalizedPath);
  }

  isSymlink(filePath: string): boolean {
    return this.lstatSync(filePath).isSymbolicLink();
  }

  readlinkSync(filePath: string): string {
    const node = this.getNode(filePath);
    if (node.kind !== 'symlink') {
      throw new Error(`EINVAL: not a symlink "${filePath}"`);
    }
    return node.target;
  }

  async readText(filePath: string): Promise<string> {
    return this.readFile(filePath);
  }

  private normalize(filePath: fs.PathLike): string {
    return path.resolve(String(filePath));
  }

  private getNode(filePath: fs.PathLike): MockNode {
    const normalizedPath = this.normalize(filePath);
    const node = this.nodes.get(normalizedPath);
    if (!node) {
      throw new Error(`ENOENT: no such file "${String(filePath)}"`);
    }
    return node;
  }

  private ensureDirectory(dirPath: string): void {
    const normalizedPath = path.resolve(dirPath);
    const root = path.parse(normalizedPath).root;
    let current = root;
    this.nodes.set(root, { kind: 'dir' });

    const segments = path.relative(root, normalizedPath).split(path.sep).filter(Boolean);
    for (const segment of segments) {
      current = path.join(current, segment);
      const node = this.nodes.get(current);
      if (!node) {
        this.nodes.set(current, { kind: 'dir' });
        continue;
      }
      if (node.kind !== 'dir') {
        throw new Error(`ENOTDIR: "${current}" is not a directory`);
      }
    }
  }

  private resolvePath(filePath: fs.PathLike): string {
    let current = this.normalize(filePath);
    const visited = new Set<string>();

    while (true) {
      const node = this.nodes.get(current);
      if (!node) {
        throw new Error(`ENOENT: no such file "${String(filePath)}"`);
      }
      if (node.kind !== 'symlink') {
        return current;
      }
      if (visited.has(current)) {
        throw new Error(`ELOOP: too many symlink levels "${String(filePath)}"`);
      }
      visited.add(current);
      current = path.resolve(path.dirname(current), node.target);
    }
  }

  private resolvePathForWrite(filePath: fs.PathLike): string {
    const normalizedPath = this.normalize(filePath);
    const node = this.nodes.get(normalizedPath);
    if (node?.kind === 'symlink') {
      return path.resolve(path.dirname(normalizedPath), node.target);
    }
    return normalizedPath;
  }
}

const AGENT_TEMPLATE = ['<!--VITE PLUS START-->', 'template block', '<!--VITE PLUS END-->'].join(
  '\n',
);

let mockFs: InMemoryFs;
let projectIndex = 0;

beforeEach(async () => {
  vi.spyOn(prompts.log, 'message').mockImplementation(() => {});

  mockFs = new InMemoryFs();
  projectIndex = 0;

  vi.spyOn(fs, 'existsSync').mockImplementation((filePath) => mockFs.existsSync(filePath));
  vi.spyOn(fs, 'lstatSync').mockImplementation((filePath) => mockFs.lstatSync(filePath));

  vi.spyOn(fsPromises, 'appendFile').mockImplementation(async (filePath, data) =>
    mockFs.appendFile(filePath as fs.PathLike, String(data)),
  );
  vi.spyOn(fsPromises, 'lstat').mockImplementation(async (filePath) => mockFs.lstat(filePath));
  vi.spyOn(fsPromises, 'mkdir').mockImplementation(async (filePath, options) => {
    await mockFs.mkdir(filePath, options as { recursive: true });
    return undefined;
  });
  vi.spyOn(fsPromises, 'readFile').mockImplementation(async (filePath) =>
    mockFs.readFile(filePath as fs.PathLike),
  );
  vi.spyOn(fsPromises, 'readlink').mockImplementation(async (filePath) =>
    mockFs.readlink(filePath),
  );
  vi.spyOn(fsPromises, 'realpath').mockImplementation(async (filePath) =>
    mockFs.realpath(filePath),
  );
  vi.spyOn(fsPromises, 'symlink').mockImplementation(async (target, filePath) => {
    await mockFs.symlink(String(target), filePath);
  });
  vi.spyOn(fsPromises, 'unlink').mockImplementation(async (filePath) => {
    await mockFs.unlink(filePath);
  });
  vi.spyOn(fsPromises, 'writeFile').mockImplementation(async (filePath, data) => {
    await mockFs.writeFile(filePath as fs.PathLike, String(data as string));
  });

  await mockFs.writeFile(path.join(pkgRoot, 'AGENTS.md'), AGENT_TEMPLATE);
});

afterEach(() => {
  vi.restoreAllMocks();
});

async function createProjectDir() {
  const dir = path.join(pkgRoot, '__virtual__', `project-${projectIndex++}`);
  await mockFs.mkdir(dir, { recursive: true });
  return dir;
}

describe('resolveAgentTargetPaths', () => {
  it('resolves comma-separated agent names and deduplicates target paths', () => {
    expect(resolveAgentTargetPaths('claude,amp,opencode,chatgpt')).toEqual([
      'CLAUDE.md',
      'AGENTS.md',
    ]);
  });

  it('resolves repeated --agent values and trims whitespace', () => {
    expect(resolveAgentTargetPaths([' claude ', ' amp, opencode ', 'codex'])).toEqual([
      'CLAUDE.md',
      'AGENTS.md',
    ]);
  });

  it('falls back to AGENTS.md when no valid agents are provided', () => {
    expect(resolveAgentTargetPaths()).toEqual(['AGENTS.md']);
    expect(resolveAgentTargetPaths(' , , ')).toEqual(['AGENTS.md']);
  });
});

describe('detectExistingAgentTargetPath', () => {
  it('detects existing regular agent files', async () => {
    const dir = await createProjectDir();
    await mockFs.writeFile(path.join(dir, 'CLAUDE.md'), '# Claude');

    expect(detectExistingAgentTargetPath(dir)).toBe('CLAUDE.md');
  });

  it('ignores symlinked agent files', async () => {
    const dir = await createProjectDir();
    await mockFs.symlink('AGENTS.md', path.join(dir, 'CLAUDE.md'));

    expect(detectExistingAgentTargetPath(dir)).toBeUndefined();
  });
});

describe('replaceMarkedAgentInstructionsSection', () => {
  it('replaces the marker block when markers are present in both files', () => {
    const existing = [
      '# Local instructions',
      '<!--VITE PLUS START-->',
      'old block',
      '<!--VITE PLUS END-->',
      '# Footer',
    ].join('\n');
    const incoming = ['<!--VITE PLUS START-->', 'new block', '<!--VITE PLUS END-->'].join('\n');

    expect(replaceMarkedAgentInstructionsSection(existing, incoming)).toBe(
      [
        '# Local instructions',
        '<!--VITE PLUS START-->',
        'new block',
        '<!--VITE PLUS END-->',
        '# Footer',
      ].join('\n'),
    );
  });

  it('returns undefined when markers are missing in existing content', () => {
    expect(
      replaceMarkedAgentInstructionsSection(
        'no markers here',
        '<!--VITE PLUS START-->\nnew\n<!--VITE PLUS END-->',
      ),
    ).toBeUndefined();
  });
});

describe('writeAgentInstructions symlink behavior', () => {
  it('links non-standard agent files to AGENTS.md when AGENTS.md is selected', async () => {
    const dir = await createProjectDir();

    await writeAgentInstructions({
      projectRoot: dir,
      targetPaths: ['AGENTS.md', 'CLAUDE.md', '.github/copilot-instructions.md'],
      interactive: false,
    });

    expect(mockFs.isSymlink(path.join(dir, 'AGENTS.md'))).toBe(false);
    expect(mockFs.isSymlink(path.join(dir, 'CLAUDE.md'))).toBe(true);
    expect(mockFs.readlinkSync(path.join(dir, 'CLAUDE.md'))).toBe('AGENTS.md');
    expect(mockFs.isSymlink(path.join(dir, '.github/copilot-instructions.md'))).toBe(true);
    expect(mockFs.readlinkSync(path.join(dir, '.github/copilot-instructions.md'))).toBe(
      path.join('..', 'AGENTS.md'),
    );
  });

  it('does not replace existing non-symlink files with symlinks', async () => {
    const dir = await createProjectDir();
    const existingClaude = path.join(dir, 'CLAUDE.md');
    await mockFs.writeFile(existingClaude, 'existing claude instructions');

    await writeAgentInstructions({
      projectRoot: dir,
      targetPaths: ['AGENTS.md', 'CLAUDE.md'],
      interactive: false,
    });

    expect(mockFs.isSymlink(existingClaude)).toBe(false);
    expect(await mockFs.readText(existingClaude)).toBe('existing claude instructions');
    expect(mockFs.existsSync(path.join(dir, 'AGENTS.md'))).toBe(true);
  });
});

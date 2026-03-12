import { join } from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { pkgRoot } from '../../utils/path.js';

const AGENT_TEMPLATE = ['<!--VITE PLUS START-->', 'template block', '<!--VITE PLUS END-->'].join(
  '\n',
);

const { files, fsMock } = vi.hoisted(() => {
  const files = new Map<string, string>();
  const fsMock = {
    existsSync: (p: string) => files.has(p),
    readFileSync: (p: string) => {
      const content = files.get(p);
      if (content === undefined) {
        throw new Error(`ENOENT: no such file "${p}"`);
      }
      return content;
    },
    writeFileSync: (p: string, data: string) => {
      files.set(p, data);
    },
  };
  return { files, fsMock };
});

vi.mock('node:fs', () => ({
  ...fsMock,
  default: fsMock,
}));

import { injectAgentBlock } from '../agent.js';

beforeEach(() => {
  files.clear();
  files.set(join(pkgRoot, 'AGENTS.md'), AGENT_TEMPLATE);
  vi.spyOn(prompts.log, 'info').mockImplementation(() => {});
  vi.spyOn(prompts.log, 'success').mockImplementation(() => {});
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe('injectAgentBlock', () => {
  it('creates file with template when file does not exist', () => {
    injectAgentBlock('/project', 'AGENTS.md');
    expect(files.get(join('/project', 'AGENTS.md'))).toBe(AGENT_TEMPLATE);
  });

  it('updates marked section when file has markers', () => {
    const existing = [
      '# Header',
      '<!--VITE PLUS START-->',
      'old content',
      '<!--VITE PLUS END-->',
      '# Footer',
    ].join('\n');
    files.set(join('/project', 'CLAUDE.md'), existing);

    injectAgentBlock('/project', 'CLAUDE.md');

    expect(files.get(join('/project', 'CLAUDE.md'))).toBe(
      [
        '# Header',
        '<!--VITE PLUS START-->',
        'template block',
        '<!--VITE PLUS END-->',
        '# Footer',
      ].join('\n'),
    );
  });

  it('does not write when content is already up-to-date', () => {
    files.set(join('/project', 'AGENTS.md'), AGENT_TEMPLATE);
    const infoSpy = vi.spyOn(prompts.log, 'info');

    injectAgentBlock('/project', 'AGENTS.md');

    expect(infoSpy).toHaveBeenCalledWith('AGENTS.md already has up-to-date Vite+ instructions');
  });

  it('appends template when file exists without markers', () => {
    files.set(join('/project', 'AGENTS.md'), '# Existing content\n');

    injectAgentBlock('/project', 'AGENTS.md');

    expect(files.get(join('/project', 'AGENTS.md'))).toBe(
      `# Existing content\n\n${AGENT_TEMPLATE}`,
    );
  });
});

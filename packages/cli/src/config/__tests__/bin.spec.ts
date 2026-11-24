import { beforeEach, expect, it, vi } from 'vitest';

import type { parseConfigArgs as parseConfigArgsImplementation } from '../../../binding/index.js';
import type { updateExistingAgentInstructions as updateExistingAgentInstructionsImplementation } from '../../utils/agent.ts';
import type {
  defaultInteractive as defaultInteractiveImplementation,
  promptGitHooks as promptGitHooksImplementation,
} from '../../utils/prompts.ts';
import type {
  log as logImplementation,
  printHeader as printHeaderImplementation,
} from '../../utils/terminal.ts';
import type {
  install as installImplementation,
  isGitHooksEnvDisabled as isGitHooksEnvDisabledImplementation,
  isHooksUserDisabled as isHooksUserDisabledImplementation,
  resolveHooksLocation as resolveHooksLocationImplementation,
} from '../hooks.ts';

const hooks = vi.hoisted(() => ({
  install: vi.fn<typeof installImplementation>(() => ({ message: '', isError: false })),
  isGitHooksEnvDisabled: vi.fn<typeof isGitHooksEnvDisabledImplementation>(() => true),
  isHooksUserDisabled: vi.fn<typeof isHooksUserDisabledImplementation>(() => false),
  resolveHooksLocation: vi.fn<typeof resolveHooksLocationImplementation>(() => {
    throw new Error('git command not found');
  }),
}));
const terminal = vi.hoisted(() => ({
  log: vi.fn<typeof logImplementation>(),
  printHeader: vi.fn<typeof printHeaderImplementation>(),
}));
const binding = vi.hoisted(() => ({
  parseConfigArgs: vi.fn<typeof parseConfigArgsImplementation>(() => ({
    status: 'ok',
    value: { agent: false },
  })),
}));
const prompts = vi.hoisted(() => ({
  defaultInteractive: vi.fn<typeof defaultInteractiveImplementation>(() => false),
  promptGitHooks: vi.fn<typeof promptGitHooksImplementation>(async () => true),
}));

vi.mock('../../../binding/index.js', () => binding);
vi.mock('../../utils/agent.ts', () => ({
  updateExistingAgentInstructions: vi.fn<typeof updateExistingAgentInstructionsImplementation>(),
}));
vi.mock('../../utils/prompts.ts', () => prompts);
vi.mock('../../utils/terminal.ts', () => terminal);
vi.mock('../hooks.ts', () => hooks);

beforeEach(() => {
  vi.clearAllMocks();
  vi.resetModules();
});

it('skips Git-backed hook resolution when hooks are disabled by environment', async () => {
  await import('../bin.ts');

  expect(binding.parseConfigArgs).toHaveBeenCalledWith([]);
  expect(hooks.isGitHooksEnvDisabled).toHaveBeenCalledOnce();
  expect(hooks.resolveHooksLocation).not.toHaveBeenCalled();
  expect(hooks.isHooksUserDisabled).not.toHaveBeenCalled();
  expect(hooks.install).not.toHaveBeenCalled();
  expect(terminal.log).toHaveBeenCalledWith('skip install (git hooks disabled)');
});

it('lets install recheck the hooks state after the first-run prompt', async () => {
  hooks.isGitHooksEnvDisabled.mockReturnValueOnce(false);
  hooks.resolveHooksLocation.mockReturnValueOnce({
    toplevel: '/missing-repo',
    baseDir: '/missing-repo',
    dir: '.vite-hooks',
    prefix: '',
    target: '.vite-hooks/_',
  });
  prompts.defaultInteractive.mockReturnValueOnce(true);

  await import('../bin.ts');

  await vi.waitFor(() => expect(hooks.install).toHaveBeenCalledWith(undefined, {}));
  expect(prompts.promptGitHooks).toHaveBeenCalledOnce();
});

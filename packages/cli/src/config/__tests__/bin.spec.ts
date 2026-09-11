import { beforeEach, expect, it, vi } from 'vitest';

const hooks = vi.hoisted(() => ({
  install: vi.fn(),
  isGitHooksEnvDisabled: vi.fn(() => true),
  isHooksUserDisabled: vi.fn(),
  resolveHooksLocation: vi.fn(() => {
    throw new Error('git command not found');
  }),
}));
const terminal = vi.hoisted(() => ({ log: vi.fn(), printHeader: vi.fn() }));
const binding = vi.hoisted(() => ({
  parseConfigArgs: vi.fn(() => ({ status: 'ok', value: { agent: false } })),
}));

vi.mock('../../../binding/index.js', () => binding);
vi.mock('../../utils/agent.ts', () => ({ updateExistingAgentInstructions: vi.fn() }));
vi.mock('../../utils/prompts.ts', () => ({
  defaultInteractive: () => false,
  promptGitHooks: vi.fn(),
}));
vi.mock('../../utils/terminal.ts', () => terminal);
vi.mock('../hooks.ts', () => hooks);

beforeEach(() => {
  vi.clearAllMocks();
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

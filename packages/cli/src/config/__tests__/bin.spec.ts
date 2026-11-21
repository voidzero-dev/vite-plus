import { beforeEach, expect, it, vi } from 'vitest';

const hooks = vi.hoisted(() => ({
  install: vi.fn(() => ({ message: '', isError: false })),
  isGitHooksEnvDisabled: vi.fn(() => true),
  isHooksUserDisabled: vi.fn(() => false),
  resolveHooksLocation: vi.fn((): unknown => {
    throw new Error('git command not found');
  }),
}));
const terminal = vi.hoisted(() => ({ log: vi.fn(), printHeader: vi.fn() }));
const binding = vi.hoisted(() => ({
  parseConfigArgs: vi.fn(() => ({ status: 'ok', value: { agent: false } })),
}));
const prompts = vi.hoisted(() => ({
  defaultInteractive: vi.fn(() => false),
  promptGitHooks: vi.fn(async () => true),
}));

vi.mock('../../../binding/index.js', () => binding);
vi.mock('../../utils/agent.ts', () => ({ updateExistingAgentInstructions: vi.fn() }));
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

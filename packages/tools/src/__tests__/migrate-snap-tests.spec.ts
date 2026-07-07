import { describe, expect, it } from 'vitest';

import {
  fixtureName,
  translateCommand,
  type NewStep,
  type TranslationContext,
} from '../migrate-snap-tests.ts';

function ctx(): TranslationContext {
  return { todos: [], notes: [], localRegistry: false, needsFreshRuntime: false };
}

function argvs(steps: NewStep[]): string[][] {
  return steps.map((s) => s.argv);
}

function isTodo(steps: NewStep[]): boolean {
  return steps.length === 1 && steps[0].comment?.startsWith('TODO(migrate)') === true;
}

describe('fixtureName', () => {
  it('normalizes every invalid identifier character', () => {
    expect(fixtureName('migration-not-supported-npm8.2')).toBe('migration_not_supported_npm8_2');
    expect(fixtureName('check-pass')).toBe('check_pass');
  });
});

describe('translateCommand', () => {
  it('drops comment-only commands with a report note', () => {
    const context = ctx();
    expect(translateCommand('# tests below assert the cache state', context)).toEqual([]);
    expect(context.notes).toHaveLength(1);
    expect(context.todos).toHaveLength(0);
  });

  it('maps test expressions to stat-file asserts, keeping exit semantics', () => {
    expect(argvs(translateCommand('test ! -f .nvmrc', ctx()))).toEqual([
      ['vpt', 'stat-file', '.nvmrc', '--assert-not', 'file'],
    ]);
    expect(argvs(translateCommand('test -d dist', ctx()))).toEqual([
      ['vpt', 'stat-file', 'dist', '--assert', 'dir'],
    ]);
    expect(argvs(translateCommand('test -e marker', ctx()))).toEqual([
      ['vpt', 'stat-file', 'marker', '--assert-not', 'missing'],
    ]);
  });

  it('keeps guard chains short-circuiting via the failing assert', () => {
    // `test -f marker && vp run build`: the guard step fails on a missing
    // marker and the line-boundary flow skips the guarded command.
    const steps = translateCommand('test -f marker && vp run build', ctx());
    expect(argvs(steps)).toEqual([
      ['vpt', 'stat-file', 'marker', '--assert', 'file'],
      ['vp', 'run', 'build'],
    ]);
    expect(steps[0].continueOnFailure).toBeUndefined();
    expect(steps[1].continueOnFailure).toBe(true);
  });

  it('passes octal and +x chmod through, TODOs other symbolic modes', () => {
    expect(argvs(translateCommand('chmod 755 hook.mjs', ctx()))).toEqual([
      ['vpt', 'chmod', '755', 'hook.mjs'],
    ]);
    expect(argvs(translateCommand('chmod +x hook.mjs', ctx()))).toEqual([
      ['vpt', 'chmod', '+x', 'hook.mjs'],
    ]);
    expect(isTodo(translateCommand('chmod u+rw hook.mjs', ctx()))).toBe(true);
  });

  it('TODOs glob arguments for shell-less vpt file verbs', () => {
    expect(isTodo(translateCommand('rm -rf *.tgz', ctx()))).toBe(true);
    expect(isTodo(translateCommand('cat dist/*.js', ctx()))).toBe(true);
  });

  it('appends the newline echo would have written to redirected files', () => {
    const steps = translateCommand('echo hello > out.txt', ctx());
    expect(argvs(steps)).toEqual([['vpt', 'write-file', 'out.txt', 'hello\n']]);
  });

  it('keeps printf redirects exact, TODOs escape sequences', () => {
    expect(argvs(translateCommand("printf 'plain text' > out.txt", ctx()))).toEqual([
      ['vpt', 'write-file', 'out.txt', 'plain text'],
    ]);
    expect(isTodo(translateCommand("printf 'a\\nb' > out.txt", ctx()))).toBe(true);
  });

  it('accepts dot-path json-edit and TODOs legacy expression syntax', () => {
    expect(
      argvs(translateCommand("json-edit package.json scripts.build 'vp build'", ctx())),
    ).toEqual([['vpt', 'json-edit', 'package.json', 'scripts.build', 'vp build']]);
    expect(isTodo(translateCommand("json-edit package.json '_.dependencies = {}'", ctx()))).toBe(
      true,
    );
  });

  it('TODOs env assignments that need shell expansion', () => {
    expect(
      isTodo(translateCommand('NPM_CONFIG_PREFIX=$(pwd)/prefix npm install -g x', ctx())),
    ).toBe(true);
    expect(isTodo(translateCommand('PATH=$PATH vp check', ctx()))).toBe(true);
  });

  it('marks only the line-final step continue-on-failure', () => {
    // Legacy lines were independent; && within a line short-circuited.
    const steps = translateCommand('vp add x && cat package.json', ctx());
    expect(steps).toHaveLength(2);
    expect(steps[0].continueOnFailure).toBeUndefined();
    expect(steps[1].continueOnFailure).toBe(true);
  });

  it('TODOs ls flags that list-dir does not replicate', () => {
    expect(isTodo(translateCommand('ls -la node_modules', ctx()))).toBe(true);
  });

  it('flags runtime-provisioning commands for seed-runtime = false', () => {
    const context = ctx();
    translateCommand('vp env install 22', context);
    expect(context.needsFreshRuntime).toBe(true);
    const plain = ctx();
    translateCommand('vp env list', plain);
    expect(plain.needsFreshRuntime).toBe(false);
  });

  it('turns leading cd chains into step cwd', () => {
    const steps = translateCommand('cd packages/web && vp run build', ctx());
    expect(steps).toHaveLength(1);
    expect(steps[0].argv).toEqual(['vp', 'run', 'build']);
    expect(steps[0].cwd).toBe('packages/web');
  });

  it('TODOs cd forms it cannot represent', () => {
    expect(isTodo(translateCommand('cd /tmp && vp check', ctx()))).toBe(true);
    expect(isTodo(translateCommand('cd $DIR && vp check', ctx()))).toBe(true);
  });
});

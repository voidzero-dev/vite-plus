import { spawnSync } from 'node:child_process';
import {
  chmod,
  copyFile,
  access,
  readdir,
  mkdtemp,
  mkdir,
  rm,
  symlink,
  writeFile,
} from 'node:fs/promises';
import { createRequire } from 'node:module';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vite-plus/test';

import { resolveCompletionConsumer } from '../../../tools/src/completion-consumer.ts';
import { writeTestCompletionMetadata } from '../../build/completion-metadata.ts';
import { VITEST_VERSION } from '../../src/utils/constants.ts';
const temporary: string[] = [];
afterEach(async () => {
  await Promise.all(temporary.splice(0).map((dir) => rm(dir, { recursive: true, force: true })));
});
describe('generated metadata through native Rust', () => {
  // Use a fresh Cargo artifact even when pnpm puts the local JS CLI first.
  // Missing artifacts are an error, not a reason to use PATH or skip this test.
  it('passes the real generated artifact through the Rust completion consumer', async () => {
    const directory = await mkdtemp(path.join(os.tmpdir(), 'vp-completion-consumer-'));
    temporary.push(directory);
    await writeFile(path.join(directory, 'package.json'), '{"name":"vite-plus","version":"1.3.0"}');
    const require = createRequire(import.meta.url);
    await symlink(
      path.resolve(path.dirname(require.resolve('vitest/package.json')), '..'),
      path.join(directory, 'node_modules'),
      'junction',
    );
    const data = await writeTestCompletionMetadata(directory, VITEST_VERSION);
    const consumerDirectory = path.join(directory, 'consumer-bin');
    await mkdir(consumerDirectory);
    const consumer = path.join(consumerDirectory, process.platform === 'win32' ? 'vp.exe' : 'vp');
    await copyFile(resolveCompletionConsumer(), consumer);
    const project = path.join(directory, 'project');
    await mkdir(project);
    await writeFile(
      path.join(project, 'vite.config.ts'),
      "throw new Error('Completion must not load config');",
    );
    const shadowDirectory = path.join(project, 'node_modules', '.bin');
    await mkdir(shadowDirectory, { recursive: true });
    const shadow = path.join(shadowDirectory, process.platform === 'win32' ? 'vp.cmd' : 'vp');
    await writeFile(
      shadow,
      process.platform === 'win32'
        ? '@echo Unexpected PATH consumer 1>&2\r\n@exit /b 93\r\n'
        : '#!/bin/sh\necho "Unexpected PATH consumer" >&2\nexit 93\n',
    );
    if (process.platform !== 'win32') {
      await chmod(shadow, 0o755);
    }
    const run = data.commands.find((command) => command.name === 'run');
    expect(run, 'The Vite+ implicit run mapping needs a current run definition').toBeDefined();
    const scopes = [
      { name: '', options: [...data.globalOptions, ...run!.options] },
      ...data.commands
        .filter((command) =>
          ['run', 'watch', 'dev', 'related', 'bench', 'list', 'doctor'].includes(command.name),
        )
        .map((command) => ({
          name: command.name,
          options: [...data.globalOptions, ...command.options],
        })),
    ];
    for (const scope of scopes) {
      const flags = [...new Set(scope.options.flatMap((option) => option.flags))];
      const flag = flags.find(
        (candidate) =>
          candidate.startsWith('--') &&
          candidate.length >= 8 &&
          flags.filter((value) => value.startsWith(candidate.slice(0, -1))).length === 1,
      );
      expect(flag, `Need a usable dynamic probe for ${scope.name || 'implicit run'}`).toBeDefined();
      const words = ['vp', 'test', ...(scope.name ? [scope.name] : []), flag!.slice(0, -1)];
      const result = spawnSync(consumer, ['--', ...words], {
        cwd: project,
        encoding: 'utf8',
        timeout: 15000,
        env: {
          ...process.env,
          PATH: shadowDirectory + path.delimiter + (process.env.PATH ?? ''),
          VP_HOME: path.join(directory, 'vp-home'),
          VP_SKIP_DEPS_INSTALL: '1',
          VP_NODE_MANAGER: 'yes',
          VP_GLOBAL_CLI_JS_SCRIPTS_DIR: path.join(directory, 'dist'),
          VP_COMPLETE: 'bash',
          _CLAP_COMPLETE_INDEX: String(words.length - 1),
          _CLAP_IFS: '\n',
        },
      });
      expect(result.error).toBeUndefined();
      expect(result.status, result.stderr).toBe(0);
      expect(result.stderr).toBe('');
      expect(result.stdout.split('\n').map((line) => line.split('\t')[0])).toContain(flag);
    }
  });

  it.each(['registration', 'request'])(
    'handles uninitialized %s without first-start installation',
    async (mode) => {
      const directory = await mkdtemp(path.join(os.tmpdir(), 'vp-uninitialized-completion-'));
      temporary.push(directory);
      const bin = path.join(directory, 'bin');
      const home = path.join(directory, 'home');
      const project = path.join(directory, 'project');
      await Promise.all([bin, home, project].map((entry) => mkdir(entry)));
      const consumer = path.join(bin, process.platform === 'win32' ? 'vp.exe' : 'vp');
      await copyFile(resolveCompletionConsumer(), consumer);
      // A regression must remain offline on Windows too, where setup needs a sibling shim.
      if (process.platform === 'win32') {
        await copyFile(consumer, path.join(bin, 'vp-shim.exe'));
      }
      const vpHome = path.join(directory, 'vp-home');
      const result = spawnSync(
        consumer,
        mode === 'registration' ? [] : ['--', 'vp', 'test', '--'],
        {
          cwd: project,
          encoding: 'utf8',
          timeout: 15000,
          env: {
            ...process.env,
            HOME: home,
            USERPROFILE: home,
            VP_HOME: vpHome,
            XDG_CONFIG_HOME: path.join(directory, 'config'),
            XDG_CACHE_HOME: path.join(directory, 'cache'),
            XDG_DATA_HOME: path.join(directory, 'data'),
            VP_GLOBAL_CLI_JS_SCRIPTS_DIR: path.join(directory, 'missing-dist'),
            VP_COMPLETE: 'bash',
            _CLAP_COMPLETE_INDEX: '2',
            _CLAP_IFS: '\n',
            // If setup accidentally runs, keep the regression probe offline, then fail on its writes.
            VP_SKIP_DEPS_INSTALL: '1',
            VP_NODE_MANAGER: 'yes',
            CI: 'true',
          },
        },
      );
      expect(result.error).toBeUndefined();
      expect(result.status, result.stderr).toBe(0);
      expect(result.stderr).toBe('');
      if (mode === 'registration') {
        expect(result.stdout).toContain('VP_COMPLETE');
      }
      await expect(access(vpHome)).rejects.toThrow();
      await expect(access(path.join(bin, '.vp-setup-complete'))).rejects.toThrow();
      expect(await readdir(home)).toEqual([]);
    },
  );
});

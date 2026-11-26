import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

import type { createCLI } from 'vitest/node';

type CLI = ReturnType<typeof createCLI>;

// Spellings accepted by Vitest but omitted from its option declarations.
const completionAliases = new Map([
  ['--isolate', '--no-isolate'],
  ['--fileParallelism', '--no-file-parallelism'],
  ['--maxWorkers', '--max-workers'],
]);

export interface OptionMetadata {
  flags: Array<string>;
  description: string;
  valueMode: 'none' | 'required' | 'optional';
}

export interface TestCompletionMetadata {
  schemaVersion: 1;
  vitePlusVersion: string;
  vitestVersion: string;
  globalOptions: Array<OptionMetadata>;
  commands: Array<{
    name: string;
    options: Array<OptionMetadata>;
  }>;
}

function primaryFlag(option: OptionMetadata): string {
  const flag = option.flags.find((flag) => flag.startsWith('--'));
  if (!flag) {
    throw new Error('Missing primary Vitest option flag');
  }
  return flag;
}

/** Keep CLI spelling: CAC normalizes `--no-color` to the name `color`. */
export function extractCompletionMetadata(
  cli: Pick<CLI, 'globalCommand' | 'commands'>,
  vitePlusVersion: string,
  vitestVersion: string,
): TestCompletionMetadata {
  function option(value: CLI['globalCommand']['options'][number]): OptionMetadata {
    const modes = [value.isBoolean === true, value.required === true, value.required === false];
    if (modes.filter(Boolean).length !== 1) {
      throw new Error(`Unsupported Vitest option arity: ${value.rawName}`);
    }
    // Match only flag declarations. Removing bracketed placeholders would break
    // nested syntax such as `--inspect [[host:]port]`.
    const flags = Array.from(
      value.rawName.matchAll(/(?:^|,\s*)(--?[^\s,]+)/g),
      (match) => match[1],
    );
    const primary = flags.find((flag) => flag.startsWith('--'));
    const alias = primary && completionAliases.get(primary);
    if (alias && !flags.includes(alias)) {
      flags.push(alias);
    }
    if (
      !primary ||
      flags.length > 16 ||
      flags.some((flag) =>
        flag.startsWith('--')
          ? flag.length - 2 > 256 || !/^--[A-Za-z0-9][A-Za-z0-9._-]*$/.test(flag)
          : !/^-[A-Za-z0-9]$/.test(flag),
      ) ||
      Buffer.byteLength(value.description ?? '') > 16_384
    ) {
      throw new Error(`Unsupported Vitest option declaration: ${value.rawName}`);
    }
    // Vitest also accepts a bare --browser to enable browser mode.
    const requiresValue = value.required && !flags.includes('--browser');
    return {
      flags,
      description: value.description ?? '',
      valueMode: value.isBoolean ? 'none' : requiresValue ? 'required' : 'optional',
    };
  }

  const metadata: TestCompletionMetadata = {
    schemaVersion: 1,
    vitePlusVersion,
    vitestVersion,
    globalOptions: cli.globalCommand.options.map(option),
    commands: cli.commands.map((command) => ({
      name: command.name,
      options: command.options.map(option),
    })),
  };
  if (metadata.commands.length > 64 || metadata.globalOptions.length > 2048) {
    throw new Error('Vitest completion metadata exceeds schema limits');
  }
  const names = new Set<string>();
  for (const command of metadata.commands) {
    if (
      !/^[A-Za-z0-9-]*$/.test(command.name) ||
      command.name.length > 64 ||
      names.has(command.name) ||
      command.options.length > 2048
    ) {
      throw new Error(`Unsupported Vitest command declaration: ${command.name}`);
    }
    names.add(command.name);
  }
  const globals = new Map<string, OptionMetadata>();
  for (const scope of [
    metadata.globalOptions,
    ...metadata.commands.map((command) => command.options),
  ]) {
    const flags = new Set<string>();
    for (const option of scope) {
      for (const flag of option.flags) {
        if (flags.has(flag)) {
          throw new Error(`Duplicate Vitest option flag: ${flag}`);
        }
        flags.add(flag);
      }
      const primary = primaryFlag(option);
      if (scope === metadata.globalOptions) {
        globals.set(primary, option);
      } else {
        const inherited = globals.get(primary);
        if (
          inherited &&
          (inherited.flags.join(',') !== option.flags.join(',') ||
            inherited.valueMode !== option.valueMode)
        ) {
          throw new Error(`Incompatible inherited Vitest option: ${primary}`);
        }
      }
    }
  }
  for (const command of metadata.commands) {
    const merged = new Map(globals);
    for (const option of command.options) {
      merged.set(primaryFlag(option), option);
    }
    const flags = new Set<string>();
    for (const option of merged.values()) {
      for (const flag of option.flags) {
        if (flags.has(flag)) {
          throw new Error(`Conflicting inherited Vitest option flag: ${flag}`);
        }
        flags.add(flag);
      }
    }
  }
  if (Buffer.byteLength(JSON.stringify(metadata, null, 2) + '\n') > 1024 * 1024) {
    throw new Error('Vitest completion metadata exceeds schema limits');
  }
  return metadata;
}

function hasCreateCLI(runner: unknown): runner is { createCLI: typeof createCLI } {
  return (
    !!runner &&
    typeof runner === 'object' &&
    'createCLI' in runner &&
    typeof runner.createCLI === 'function'
  );
}

async function readVersion(file: string): Promise<string> {
  const data: unknown = JSON.parse(await readFile(file, 'utf8'));
  if (
    !data ||
    typeof data !== 'object' ||
    !('version' in data) ||
    typeof data.version !== 'string'
  ) {
    throw new Error(`Missing package version in ${file}`);
  }
  return data.version;
}

/** Generate after tsdown and shim synchronization have finished cleaning dist. */
export async function writeTestCompletionMetadata(
  packageDir: string,
  expectedVitestVersion: string,
): Promise<TestCompletionMetadata> {
  const require = createRequire(join(packageDir, 'package.json'));
  const vitePlusVersion = await readVersion(join(packageDir, 'package.json'));
  const vitestVersion = await readVersion(require.resolve('vitest/package.json'));
  if (vitestVersion !== expectedVitestVersion) {
    throw new Error(`Expected vitest@${expectedVitestVersion}, found ${vitestVersion}`);
  }
  // Resolve from the same CLI package that owns the runner dependency.
  const runner: unknown = await import(pathToFileURL(require.resolve('vitest/node')).href);
  if (!hasCreateCLI(runner)) {
    throw new Error('Vitest does not export createCLI');
  }
  const metadata = extractCompletionMetadata(runner.createCLI(), vitePlusVersion, vitestVersion);
  const directory = join(packageDir, 'dist/completion');
  await mkdir(directory, { recursive: true });
  await writeFile(join(directory, 'test.json'), `${JSON.stringify(metadata, null, 2)}\n`);
  return metadata;
}

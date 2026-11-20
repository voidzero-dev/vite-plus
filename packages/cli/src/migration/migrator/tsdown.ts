import fs from 'node:fs';
import path from 'node:path';

import { parseLiteralCommands } from '../../../binding/index.js';
import type { WorkspacePackage } from '../../types/index.ts';
import { documentationUrl } from '../../utils/documentation.ts';
import { displayRelative } from '../../utils/path.ts';
import { detectDefaultTsdownConfig } from '../detector.ts';
import type { MigrationReport } from '../report.ts';
import {
  getCommandStartIndex,
  readPackageJsonIfExists,
  SHELL_STATE_COMMANDS,
  warnMigration,
} from './shared.ts';

const ENVIRONMENT_ASSIGNMENT = /^[A-Za-z_]\w*=/;
const EXPLICIT_CONFIG = /^(?:-c|--config)(?:=|$)/;
const OTHER_CONFIG_DIRECTORY = /^(?:--cwd|-W|--workspace)(?:=|$)/;
// When the shell parser cannot read an expansion or compound command, keep
// possible explicit config selections rather than risk changing their entry.
const POSSIBLE_CONFIG_COMMAND =
  /(?:^|[\s"';&|()])tsdown(?=\s)[^;&|\r\n]*?\s(?:-c|--config)(?=[=\s"']|$)/;

function scriptInvocation(command: readonly { value: string }[]): {
  args: string[];
  shellCommand?: string;
} {
  const argv = command.map(({ value }) => value);
  let start = 0;
  while (ENVIRONMENT_ASSIGNMENT.test(argv[start] ?? '')) {
    start++;
  }
  const wrapper = argv[start];
  if (wrapper === 'cross-env' || wrapper === 'cross-env-shell' || wrapper === 'env') {
    start++;
    while (ENVIRONMENT_ASSIGNMENT.test(argv[start] ?? '')) {
      start++;
    }
  }
  const invocation = argv.slice(start);
  if (wrapper === 'cross-env-shell' && invocation.length === 1 && invocation[0].includes(' ')) {
    return { args: [], shellCommand: invocation[0] };
  }
  let executable = getCommandStartIndex(invocation);
  if (invocation[0] === 'bunx' && invocation[executable] === '--bun') {
    executable++;
  }
  return { args: invocation.slice(executable) };
}

type TsdownConfigUse = {
  // An undefined path can refer to any workspace package.
  selected: (string | undefined)[];
  defaultDirectories: (string | undefined)[];
};

function resolveScriptPath(cwd: string | undefined, value: string | undefined): string | undefined {
  if (!cwd || !value || /^[-~]|%[^%]+%|![^!]+!|[*?]/.test(value)) {
    return undefined;
  }
  return path.resolve(cwd, value);
}

function scanTsdownScript(script: string, cwd: string | undefined): TsdownConfigUse {
  const usage: TsdownConfigUse = {
    selected: [],
    defaultDirectories: [],
  };
  if (!script.includes('tsdown')) {
    return usage;
  }
  const commands = parseLiteralCommands(script, true);
  if (!commands) {
    if (POSSIBLE_CONFIG_COMMAND.test(script)) {
      usage.selected.push(undefined);
    }
    usage.defaultDirectories.push(undefined);
    return usage;
  }
  // An && chain can only reach the tsdown command after every preceding cd succeeds.
  const andChain = commands.slice(1).every((command, index) => {
    const previous = commands[index].at(-1)!;
    return script.slice(previous.end, command[0].start).trim() === '&&';
  });
  for (const command of commands) {
    const { args, shellCommand } = scriptInvocation(command);
    if (SHELL_STATE_COMMANDS.includes(args[0]) || args[0] === 'chdir') {
      cwd =
        andChain && command[0].value === 'cd' && command.length === 2
          ? resolveScriptPath(cwd, command[1].value)
          : undefined;
    }
    if (shellCommand) {
      const nested = scanTsdownScript(shellCommand, cwd);
      usage.selected.push(...nested.selected);
      usage.defaultDirectories.push(...nested.defaultDirectories);
      continue;
    }
    if (args[0] !== 'tsdown') {
      continue;
    }
    const separator = args.indexOf('--', 1);
    const flags = args.slice(1, separator === -1 ? undefined : separator);
    const configDirectory = flags.some((arg) => OTHER_CONFIG_DIRECTORY.test(arg)) ? undefined : cwd;
    const previousSelections = usage.selected.length;
    for (const [index, arg] of flags.entries()) {
      if (EXPLICIT_CONFIG.test(arg)) {
        const equals = arg.indexOf('=');
        const value = equals < 0 ? flags[index + 1] : arg.slice(equals + 1);
        usage.selected.push(resolveScriptPath(configDirectory, value));
      }
    }
    if (usage.selected.length === previousSelections && !flags.includes('--no-config')) {
      usage.defaultDirectories.push(configDirectory);
    }
  }
  return usage;
}

function readPackageScripts(manifestPath: string): Record<string, unknown> {
  const pkg = readPackageJsonIfExists(manifestPath) as
    | { scripts?: Record<string, unknown> }
    | undefined;
  return pkg?.scripts ?? {};
}

export function collectPackagesKeepingTsdown(
  rootDir: string,
  packages: readonly WorkspacePackage[] = [],
  report?: Pick<MigrationReport, 'warnings'>,
): ReadonlySet<string> {
  const workspaceRoot = path.resolve(rootDir);
  const projectPaths = new Set([
    workspaceRoot,
    ...packages.map((pkg) => path.resolve(rootDir, pkg.path)),
  ]);
  const keptPackages = new Set<string>();
  const defaultDirectories = new Map<string, (string | undefined)[]>();
  for (const dir of projectPaths) {
    const manifestPath = path.join(dir, 'package.json');
    for (const [name, script] of Object.entries(readPackageScripts(manifestPath))) {
      if (typeof script !== 'string') {
        continue;
      }
      const usage = scanTsdownScript(script, dir);
      if (usage.defaultDirectories.length > 0) {
        const directories = defaultDirectories.get(dir) ?? [];
        directories.push(...usage.defaultDirectories);
        defaultDirectories.set(dir, directories);
      }
      if (usage.selected.length === 0) {
        continue;
      }
      keptPackages.add(dir);
      // Explicit config files can belong to a different package. Keep that
      // package's config and imports even though its own scripts do not select it.
      for (const config of usage.selected) {
        for (const owner of findConfigOwners(config, projectPaths)) {
          keptPackages.add(owner);
        }
      }
      if (report) {
        warnMigration(
          `${displayRelative(manifestPath)} script "${name}" selects a tsdown config. Kept tsdown in this workspace. Move the selected config into \`pack\` in \`vite.config.ts\` before switching to \`vp pack\` without the config flag. See ${documentationUrl('/guide/migrate#tsdown')}`,
          report,
        );
      }
    }
  }
  // Keeping a manifest's tsdown scripts also keeps unflagged commands in it.
  // These can run in another package or find a default config in a parent.
  for (const dir of keptPackages) {
    for (const cwd of defaultDirectories.get(dir) ?? []) {
      if (cwd === undefined || !projectPaths.has(cwd)) {
        return projectPaths;
      }
      keptPackages.add(cwd);
      const config = findDefaultTsdownConfig(cwd, workspaceRoot);
      if (config) {
        for (const owner of findConfigOwners(config, projectPaths)) {
          keptPackages.add(owner);
        }
      }
    }
  }
  return keptPackages;
}

function findDefaultTsdownConfig(cwd: string, workspaceRoot: string): string | undefined {
  for (let directory = cwd; ; directory = path.dirname(directory)) {
    const config = detectDefaultTsdownConfig(directory);
    if (config) {
      return path.join(directory, config);
    }
    if (directory === workspaceRoot || directory === path.dirname(directory)) {
      return undefined;
    }
  }
}

// A shell expansion or directory selection can load a config from anywhere
// in the workspace. Keep all packages when the config cannot be located.
function findConfigOwners(
  configPath: string | undefined,
  projectPaths: ReadonlySet<string>,
): Iterable<string> {
  if (!configPath) {
    return projectPaths;
  }
  try {
    const stat = fs.lstatSync(configPath, { throwIfNoEntry: false });
    if (stat?.isDirectory() || stat?.isSymbolicLink()) {
      return projectPaths;
    }
  } catch {
    return projectPaths;
  }
  for (let parent = path.dirname(configPath); ; parent = path.dirname(parent)) {
    if (projectPaths.has(parent)) {
      return [parent];
    }
    if (parent === path.dirname(parent)) {
      return [];
    }
  }
}

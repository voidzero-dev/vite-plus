import fs from 'node:fs';
import path from 'node:path';

import { parseLiteralCommands } from '../../../binding/index.js';
import type { WorkspacePackage } from '../../types/index.ts';
import { documentationUrl } from '../../utils/documentation.ts';
import { displayRelative } from '../../utils/path.ts';
import type { MigrationReport } from '../report.ts';
import {
  getCommandStartIndex,
  readPackageJsonIfExists,
  SHELL_STATE_COMMANDS,
  warnMigration,
} from './shared.ts';

const ENVIRONMENT_ASSIGNMENT = /^[A-Za-z_]\w*=/;
const EXPLICIT_CONFIG = /^(?:-c|--config)(?:=|$)/;
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

// An undefined argument means the script selects a config that we cannot locate.
function selectedTsdownConfigs(script: string): (string | undefined)[] {
  if (!script.includes('tsdown')) {
    return [];
  }
  const commands = parseLiteralCommands(script, true);
  if (!commands) {
    return POSSIBLE_CONFIG_COMMAND.test(script) ? [undefined] : [];
  }
  const configs: (string | undefined)[] = [];
  let knownCwd = true;
  for (const command of commands) {
    const { args, shellCommand } = scriptInvocation(command);
    if (SHELL_STATE_COMMANDS.includes(args[0]) || args[0] === 'chdir') {
      knownCwd = false;
    }
    if (shellCommand) {
      configs.push(
        ...selectedTsdownConfigs(shellCommand).map((arg) => (knownCwd ? arg : undefined)),
      );
      continue;
    }
    if (args[0] !== 'tsdown') {
      continue;
    }
    for (let index = 1; index < args.length; index++) {
      const arg = args[index];
      if (arg === '--') {
        break;
      }
      if (EXPLICIT_CONFIG.test(arg)) {
        const equals = arg.indexOf('=');
        const value = equals < 0 ? args[index + 1] : arg.slice(equals + 1);
        configs.push(knownCwd && !value?.startsWith('-') ? value : undefined);
      }
    }
  }
  return configs;
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
  report?: MigrationReport,
): ReadonlySet<string> {
  const projectPaths = new Set([
    path.resolve(rootDir),
    ...packages.map((pkg) => path.resolve(rootDir, pkg.path)),
  ]);
  const keptPackages = new Set<string>();
  for (const dir of projectPaths) {
    const manifestPath = path.join(dir, 'package.json');
    for (const [name, script] of Object.entries(readPackageScripts(manifestPath))) {
      const configs = typeof script === 'string' ? selectedTsdownConfigs(script) : [];
      if (configs.length === 0) {
        continue;
      }
      keptPackages.add(dir);
      // Explicit config files can belong to a different package. Keep that
      // package's config and imports even though its own scripts do not select it.
      for (const config of configs) {
        for (const owner of findConfigOwners(dir, config, projectPaths)) {
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
  return keptPackages;
}

// A shell expansion or directory selection can load a config from anywhere
// in the workspace. Keep all packages when the config cannot be located.
function findConfigOwners(
  dir: string,
  config: string | undefined,
  projectPaths: ReadonlySet<string>,
): Iterable<string> {
  // Windows command interpreters can expand variables in otherwise literal words.
  if (!config || /%[^%]+%|![^!]+!/.test(config)) {
    return projectPaths;
  }
  const configPath = path.resolve(dir, config);
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

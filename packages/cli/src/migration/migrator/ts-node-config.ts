import fs from 'node:fs';
import path from 'node:path';

import { type ParseError, parse as parseJsonc } from 'jsonc-parser';

import { NODE_OPTIONS_WITH_VALUE } from '../../vpx-script.ts';
import { isPlainRecord } from './shared.ts';

interface TsNodeRunContext {
  cwd?: string;
  environment: readonly string[];
  nodeLoader: boolean;
  executable: string;
  shebang?: boolean;
  publicRegister?: boolean;
  nodeOptionsArgs?: readonly string[];
}

interface TsNodeRunPreparation {
  keepReason?: string;
  /** Indices refer to args, excluding the executable. An absent value removes a token. */
  edits?: { index: number; value?: string }[];
  prependArgs?: string[];
}

interface TsNodeOptions {
  transpileOnly?: boolean;
  typeCheck?: boolean;
  esm?: boolean;
  files?: boolean;
}

const KEEP_TYPE_CHECK =
  'this ts-node command type-checks before execution; `vpx` does not type-check. Keep a separate type-check command before changing this script';
const KEEP_CWD = 'the working directory is unknown, so the ts-node configuration cannot be checked';
const TS_NODE_LOADER =
  /^ts-node\/(?:register(?:\/(?:transpile-only|type-check|files))?|esm(?:\/transpile-only)?(?:\.mjs)?)$/;
const PRELOAD_OPTIONS = new Set([
  '-r',
  '--require',
  '--import',
  '--loader',
  '--experimental-loader',
]);
const BOOLEAN_OPTIONS = new Map<string, keyof TsNodeOptions>([
  ['-T', 'transpileOnly'],
  ['--transpileOnly', 'transpileOnly'],
  ['--transpile-only', 'transpileOnly'],
  ['--typeCheck', 'typeCheck'],
  ['--type-check', 'typeCheck'],
  ['--esm', 'esm'],
  ['--files', 'files'],
]);
const CONFIG_OPTIONS = new Set(['transpileOnly', 'typeCheck', 'esm', 'files']);

function booleanEnvironment(value: string): boolean | undefined {
  if (/^(?:y|yes|true|1)$/i.test(value)) {
    return true;
  }
  if (/^(?:n|no|false|0)$/i.test(value)) {
    return false;
  }
  return undefined;
}

function nearestTsconfig(dir: string): string | undefined {
  for (;;) {
    const file = path.join(dir, 'tsconfig.json');
    if (fs.existsSync(file)) {
      return file;
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      return undefined;
    }
    dir = parent;
  }
}

/** Read only data; migration must not load ts-node or a project's compiler. */
function readTsNodeOptions(
  file: string,
  ancestors = new Set<string>(),
): { options?: TsNodeOptions; keepReason?: string } {
  const keepConfig = (reason: string) => ({
    keepReason: `the ts-node configuration in \`${file}\` ${reason}`,
  });
  if (ancestors.has(file)) {
    return keepConfig('has circular inheritance');
  }
  const chain = new Set(ancestors).add(file);
  let config: unknown;
  try {
    const errors: ParseError[] = [];
    config = parseJsonc(fs.readFileSync(file, 'utf8'), errors, { allowTrailingComma: true });
    if (errors.length || !isPlainRecord(config)) {
      return keepConfig('cannot be parsed');
    }
  } catch {
    return keepConfig('cannot be read');
  }
  let inherited: TsNodeOptions = {};
  if (config.extends !== undefined) {
    if (
      typeof config.extends !== 'string' ||
      (!config.extends.startsWith('.') && !path.isAbsolute(config.extends))
    ) {
      return keepConfig('uses inheritance that vp migrate cannot resolve');
    }
    let base = path.resolve(path.dirname(file), config.extends);
    if (!fs.existsSync(base) && !base.endsWith('.json')) {
      base += '.json';
    }
    const result = readTsNodeOptions(base, chain);
    if (result.keepReason) {
      return result;
    }
    inherited = result.options ?? {};
  }
  const settings = config['ts-node'];
  if (settings === undefined) {
    return { options: inherited };
  }
  if (!isPlainRecord(settings)) {
    return keepConfig('has an invalid `ts-node` object');
  }
  for (const [key, value] of Object.entries(settings)) {
    if (!CONFIG_OPTIONS.has(key)) {
      return keepConfig(`uses \`ts-node.${key}\`, which vp migrate does not translate`);
    }
    if (typeof value !== 'boolean') {
      return keepConfig(`has a non-boolean \`ts-node.${key}\` value`);
    }
  }
  return { options: { ...inherited, ...settings } };
}

/**
 * Translate the supported ts-node options before the shared runner scans Node options.
 * ts-node merges environment < inherited config < CLI/API, then typeCheck overrides
 * transpileOnly. Keep all other compiler behavior for manual migration.
 */
export function prepareTsNodeRun(
  args: readonly string[],
  context: TsNodeRunContext,
): TsNodeRunPreparation {
  const { cwd, nodeLoader, executable } = context;
  if (!cwd) {
    return { keepReason: KEEP_CWD };
  }
  if (context.publicRegister) {
    if (
      context.nodeOptionsArgs === undefined &&
      context.environment.some((assignment) => /^NODE_OPTIONS=.+/i.test(assignment))
    ) {
      return {
        keepReason: 'the literal NODE_OPTIONS arguments cannot be checked for ts-node settings',
      };
    }
    args = [...(context.nodeOptionsArgs ?? []), ...args];
  }
  const environment: TsNodeOptions = {};
  let environmentProject: string | undefined;
  for (const assignment of context.environment) {
    const separator = assignment.indexOf('=');
    if (separator < 0) {
      continue;
    }
    const originalName = assignment.slice(0, separator);
    const name = originalName.toUpperCase();
    const value = assignment.slice(separator + 1);
    if (context.publicRegister && name === 'NODE_OPTIONS' && originalName !== name) {
      return {
        keepReason: `\`${originalName}\` uses platform-dependent environment variable casing`,
      };
    }
    if (name.startsWith('TS_NODE_') && originalName !== name) {
      return {
        keepReason: `\`${originalName}\` uses platform-dependent environment variable casing`,
      };
    }
    if (name === 'TS_NODE_PROJECT') {
      environmentProject = value || undefined;
    } else if (name === 'TS_NODE_TRANSPILE_ONLY' || name === 'TS_NODE_TYPE_CHECK') {
      const parsed = booleanEnvironment(value);
      if (parsed === undefined) {
        return { keepReason: `\`${name}\` does not have a literal boolean value` };
      }
      environment[name === 'TS_NODE_TRANSPILE_ONLY' ? 'transpileOnly' : 'typeCheck'] = parsed;
    } else if (name.startsWith('TS_NODE_')) {
      return {
        keepReason: `\`${name}\` configures ts-node behavior that vp migrate does not translate`,
      };
    } else if (
      ['OXC_TSCONFIG_PATH', 'TSX_TSCONFIG_PATH', 'VP_SCRIPT_TSCONFIG'].includes(name) &&
      value
    ) {
      return {
        keepReason: `\`${name}\` would select a different configuration for the Vite+ loader`,
      };
    }
  }

  const explicit: TsNodeOptions = {};
  if (executable === 'ts-node-transpile-only') {
    explicit.transpileOnly = true;
  } else if (executable !== 'ts-node' && executable !== 'ts-node-esm' && !nodeLoader) {
    return { keepReason: `vp migrate does not translate the \`${executable}\` execution mode` };
  }
  const edits: NonNullable<TsNodeRunPreparation['edits']> = [];
  let project: string | undefined;
  let projectIndex: number | undefined;
  let projectInline = false;
  let entry: string | undefined;
  let loaders = 0;
  for (let index = 0; index < args.length; index++) {
    const value = args[index];
    if (value === '--') {
      entry = args[index + 1];
      break;
    }
    if (!value.startsWith('-')) {
      entry = value;
      break;
    }
    const equals = value.indexOf('=');
    const originalOption = equals < 0 ? value : value.slice(0, equals);
    const name =
      nodeLoader && originalOption.startsWith('--')
        ? originalOption.replaceAll('_', '-')
        : originalOption;
    const inline = equals < 0 ? undefined : value.slice(equals + 1);
    if (context.publicRegister && name === '--run') {
      return {
        keepReason:
          'vp migrate cannot check the ts-node settings of a package script run by Node.js',
      };
    }
    if (
      context.publicRegister &&
      (name === '--eval' ||
        name === '--print' ||
        /^-[ep]/.test(name) ||
        name === '-i' ||
        name === '--interactive')
    ) {
      return {
        keepReason: 'vp migrate does not translate ts-node inline code or REPL execution',
      };
    }
    if (nodeLoader && (name === '--env-file' || name === '--env-file-if-exists')) {
      return {
        keepReason: `\`${name}\` can configure ts-node through environment variables; vp migrate does not translate those settings`,
      };
    }
    if (
      nodeLoader &&
      [
        '--config-file',
        '--experimental-config-file',
        '--experimental-default-config-file',
      ].includes(name)
    ) {
      return {
        keepReason: `\`${name}\` can load preloads that configure ts-node; migrate it manually`,
      };
    }
    const booleanOption = BOOLEAN_OPTIONS.get(name);
    if (!nodeLoader && booleanOption) {
      if (inline !== undefined) {
        return {
          keepReason: `ts-node treats \`${name}=value\` as an enabled flag; use an explicit configuration value before migrating`,
        };
      }
      explicit[booleanOption] = true;
      edits.push({ index });
      continue;
    }
    if (!nodeLoader && (name === '-P' || name === '--project')) {
      if (projectIndex !== undefined) {
        return { keepReason: 'the command selects a ts-node project more than once' };
      }
      project = inline ?? args[index + 1];
      projectIndex = index;
      projectInline = inline !== undefined;
      if (!project || project.startsWith('-')) {
        return { keepReason: `\`${name}\` does not name a tsconfig` };
      }
      if (!projectInline) {
        index++;
      }
      continue;
    }
    if (PRELOAD_OPTIONS.has(name)) {
      if (!nodeLoader && name !== '-r' && name !== '--require') {
        return {
          keepReason: `\`${name}\` uses ts-node behavior that vp migrate does not translate`,
        };
      }
      const preload = inline ?? args[index + 1];
      if (!preload || preload.startsWith('-')) {
        return { keepReason: `\`${name}\` does not name a preload` };
      }
      if (TS_NODE_LOADER.test(preload)) {
        if (!nodeLoader || ++loaders > 1) {
          return { keepReason: 'the command registers ts-node more than once' };
        }
        if (/\/transpile-only(?:\.mjs)?$/.test(preload)) {
          explicit.transpileOnly = true;
        } else if (preload.endsWith('/type-check')) {
          explicit.typeCheck = true;
        }
      } else {
        return {
          keepReason: `the additional preload \`${preload}\` can configure ts-node before execution; migrate it manually`,
        };
      }
      if (inline === undefined) {
        index++;
      }
      continue;
    }
    if (!nodeLoader) {
      return { keepReason: `\`${name}\` uses ts-node behavior that vp migrate does not translate` };
    }
    if (inline === undefined && NODE_OPTIONS_WITH_VALUE.has(name)) {
      index++;
    }
  }
  if (!entry || entry === '-') {
    return { keepReason: 'vp migrate only replaces ts-node commands that name a script file' };
  }

  const selectedProject = project ?? environmentProject;
  if (context.shebang && selectedProject !== undefined) {
    return {
      keepReason:
        'the ts-node shebang selects a tsconfig relative to the caller; migrate it manually',
    };
  }
  let configFile: string | undefined;
  let configArgument = project;
  if (selectedProject) {
    configFile = path.resolve(cwd, selectedProject);
    if (fs.statSync(configFile, { throwIfNoEntry: false })?.isDirectory()) {
      if (project === undefined) {
        return {
          keepReason: '`TS_NODE_PROJECT` names a directory; `vpx` requires the tsconfig file path',
        };
      }
      configFile = path.join(configFile, 'tsconfig.json');
      configArgument = path.join(project, 'tsconfig.json');
    }
  } else {
    let searchDir = cwd;
    if (!nodeLoader) {
      try {
        searchDir = path.dirname(fs.realpathSync(path.resolve(cwd, entry)));
      } catch {
        return {
          keepReason: `\`${entry}\` cannot be resolved, so its ts-node configuration cannot be checked`,
        };
      }
    }
    configFile = nearestTsconfig(searchDir);
    if (context.publicRegister && !configFile) {
      let entryConfig: string | undefined;
      try {
        entryConfig = nearestTsconfig(path.dirname(fs.realpathSync(path.resolve(cwd, entry))));
      } catch {
        return {
          keepReason: `\`${entry}\` cannot be resolved, so its ts-node configuration cannot be checked`,
        };
      }
      if (entryConfig) {
        return {
          keepReason:
            'the Node.js preload would discover an entrypoint tsconfig that ts-node does not use; select the intended config with TS_NODE_PROJECT before migrating',
        };
      }
    }
  }
  const config = configFile ? readTsNodeOptions(configFile) : { options: {} };
  if (config.keepReason) {
    return { keepReason: config.keepReason };
  }
  if (context.shebang && configFile) {
    return {
      keepReason:
        'the ts-node shebang needs a pinned tsconfig, whose path depends on the caller; migrate it manually',
    };
  }
  const options = { ...environment, ...config.options, ...explicit };
  if (options.transpileOnly !== true || options.typeCheck === true) {
    return { keepReason: KEEP_TYPE_CHECK };
  }
  if (projectIndex !== undefined) {
    edits.push({
      index: projectIndex,
      value: `--tsconfig=${configArgument!.split(path.sep).join('/')}`,
    });
    if (!projectInline) {
      edits.push({ index: projectIndex + 1 });
    }
  }
  if (configFile && selectedProject === undefined) {
    if (context.publicRegister) {
      return {
        keepReason:
          'the Node.js preload needs the ts-node config pinned explicitly; set TS_NODE_PROJECT to its file path before migrating',
      };
    }
    // ts-node applies its selected config to the whole run; vpx otherwise discovers
    // configs per file. Pin the same file with a portable path from the command's cwd.
    const relative = path
      .relative(fs.realpathSync(cwd), fs.realpathSync(configFile))
      .split(path.sep)
      .join('/');
    return { edits, prependArgs: [`--tsconfig=${relative}`] };
  }
  return { edits };
}

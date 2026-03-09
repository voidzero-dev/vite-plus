import fs from 'node:fs';
import path from 'node:path';

import { mergeJsonConfig } from '../binding/index.js';
import { fmt as resolveFmt } from './resolve-fmt.js';
import { runCommandSilently } from './utils/command.js';
import { BASEURL_TSCONFIG_WARNING, VITE_PLUS_NAME } from './utils/constants.js';
import { warnMsg } from './utils/terminal.js';
import { hasBaseUrlInTsconfig } from './utils/tsconfig.js';

interface InitCommandSpec {
  configKey: 'lint' | 'fmt';
  triggerFlags: string[];
  defaultConfigFiles: string[];
}

const INIT_COMMAND_SPECS: Record<string, InitCommandSpec> = {
  lint: {
    configKey: 'lint',
    triggerFlags: ['--init'],
    defaultConfigFiles: ['.oxlintrc.json'],
  },
  fmt: {
    configKey: 'fmt',
    triggerFlags: ['--init', '--migrate'],
    defaultConfigFiles: ['.oxfmtrc.json', '.oxfmtrc.jsonc'],
  },
};

const VITE_CONFIG_FILES = [
  'vite.config.ts',
  'vite.config.mts',
  'vite.config.cts',
  'vite.config.js',
  'vite.config.mjs',
  'vite.config.cjs',
] as const;

export interface InitCommandInspection {
  handled: boolean;
  configKey?: 'lint' | 'fmt';
  existingViteConfigPath?: string;
  hasExistingConfigKey?: boolean;
}

export interface ApplyToolInitResult {
  handled: boolean;
  action?: 'added' | 'skipped-existing' | 'no-generated-config';
  configKey?: 'lint' | 'fmt';
  viteConfigPath?: string;
}

function optionTerminatorIndex(args: string[]): number {
  const index = args.indexOf('--');
  return index === -1 ? args.length : index;
}

function hasTriggerFlag(args: string[], triggerFlags: string[]): boolean {
  const limit = optionTerminatorIndex(args);
  for (let i = 0; i < limit; i++) {
    const arg = args[i];
    if (triggerFlags.some((flag) => arg === flag || arg.startsWith(`${flag}=`))) {
      return true;
    }
  }
  return false;
}

function extractConfigPathArg(args: string[]): string | null {
  const limit = optionTerminatorIndex(args);
  for (let i = 0; i < limit; i++) {
    const arg = args[i];
    if (arg === '-c' || arg === '--config') {
      const value = args[i + 1];
      return value ? value : null;
    }
    if (arg.startsWith('--config=')) {
      return arg.slice('--config='.length);
    }
    if (arg.startsWith('-c=')) {
      return arg.slice('-c='.length);
    }
  }
  return null;
}

function resolveGeneratedConfigPath(
  projectPath: string,
  args: string[],
  defaultConfigFiles: readonly string[],
): string | null {
  const configArg = extractConfigPathArg(args);
  if (configArg) {
    const resolved = path.isAbsolute(configArg) ? configArg : path.join(projectPath, configArg);
    if (fs.existsSync(resolved)) {
      return resolved;
    }
  }

  for (const filename of defaultConfigFiles) {
    const fullPath = path.join(projectPath, filename);
    if (fs.existsSync(fullPath)) {
      return fullPath;
    }
  }

  return null;
}

function findViteConfigPath(projectPath: string): string | null {
  for (const filename of VITE_CONFIG_FILES) {
    const fullPath = path.join(projectPath, filename);
    if (fs.existsSync(fullPath)) {
      return fullPath;
    }
  }
  return null;
}

function ensureViteConfigPath(projectPath: string): string {
  const existing = findViteConfigPath(projectPath);
  if (existing) {
    return existing;
  }
  const viteConfigPath = path.join(projectPath, 'vite.config.ts');
  fs.writeFileSync(
    viteConfigPath,
    `import { defineConfig } from '${VITE_PLUS_NAME}';

export default defineConfig({});
`,
  );
  return viteConfigPath;
}

function hasConfigKey(viteConfigPath: string, configKey: string): boolean {
  const viteConfig = fs.readFileSync(viteConfigPath, 'utf8');
  return new RegExp(`\\b${configKey}\\s*:`).test(viteConfig);
}

async function vpFmt(cwd: string, filePath: string): Promise<void> {
  const { binPath, envs } = await resolveFmt();
  const result = await runCommandSilently({
    command: binPath,
    args: ['--write', filePath],
    cwd,
    envs: {
      ...process.env,
      ...envs,
    },
  });
  if (result.exitCode !== 0) {
    throw new Error(
      `Failed to format ${filePath} with vp fmt:\n${result.stdout.toString()}${result.stderr.toString()}`,
    );
  }
}

function resolveInitSpec(command: string | undefined, args: string[]): InitCommandSpec | null {
  if (!command) {
    return null;
  }
  const spec = INIT_COMMAND_SPECS[command];
  if (!spec || !hasTriggerFlag(args, spec.triggerFlags)) {
    return null;
  }
  return spec;
}

export function inspectInitCommand(
  command: string | undefined,
  args: string[],
  projectPath = process.cwd(),
): InitCommandInspection {
  const spec = resolveInitSpec(command, args);
  if (!spec) {
    return { handled: false };
  }

  const viteConfigPath = findViteConfigPath(projectPath);
  if (!viteConfigPath) {
    return {
      handled: true,
      configKey: spec.configKey,
      hasExistingConfigKey: false,
    };
  }

  return {
    handled: true,
    configKey: spec.configKey,
    existingViteConfigPath: viteConfigPath,
    hasExistingConfigKey: hasConfigKey(viteConfigPath, spec.configKey),
  };
}

/**
 * Merge generated tool config from `vp lint/fmt --init` (and fmt --migrate)
 * into the project's vite config, then remove the generated standalone file.
 *
 * Returns true when the command was an init/migrate command (handled), false otherwise.
 */
export async function applyToolInitConfigToViteConfig(
  command: string | undefined,
  args: string[],
  projectPath = process.cwd(),
): Promise<ApplyToolInitResult> {
  const inspection = inspectInitCommand(command, args, projectPath);
  if (!inspection.handled || !inspection.configKey) {
    return { handled: false };
  }
  const spec = INIT_COMMAND_SPECS[command as keyof typeof INIT_COMMAND_SPECS];
  const viteConfigPath = ensureViteConfigPath(projectPath);
  const generatedConfigPath = resolveGeneratedConfigPath(
    projectPath,
    args,
    spec.defaultConfigFiles,
  );

  if (hasConfigKey(viteConfigPath, spec.configKey)) {
    if (generatedConfigPath) {
      fs.rmSync(generatedConfigPath, { force: true });
    }
    return {
      handled: true,
      action: 'skipped-existing',
      configKey: spec.configKey,
      viteConfigPath,
    };
  }

  if (spec.configKey === 'lint' && hasTriggerFlag(args, ['--init'])) {
    const lintInitConfigPath = path.join(projectPath, '.vite-plus-lint-init.oxlintrc.json');
    // Skip typeAware/typeCheck when tsconfig.json has baseUrl (unsupported by tsgolint)
    const hasBaseUrl = hasBaseUrlInTsconfig(projectPath);
    const initOptions = hasBaseUrl ? {} : { typeAware: true, typeCheck: true };
    if (hasBaseUrl) {
      warnMsg(BASEURL_TSCONFIG_WARNING);
    }
    fs.writeFileSync(lintInitConfigPath, JSON.stringify({ options: initOptions }));
    const mergeResult = mergeJsonConfig(viteConfigPath, lintInitConfigPath, spec.configKey);

    if (!mergeResult.updated) {
      throw new Error(`Failed to initialize lint config in ${path.basename(viteConfigPath)}`);
    }

    fs.writeFileSync(viteConfigPath, mergeResult.content);
    fs.rmSync(lintInitConfigPath, { force: true });
    if (generatedConfigPath) {
      fs.rmSync(generatedConfigPath, { force: true });
    }
    await vpFmt(projectPath, path.relative(projectPath, viteConfigPath));
    return {
      handled: true,
      action: 'added',
      configKey: spec.configKey,
      viteConfigPath,
    };
  }

  if (!generatedConfigPath) {
    return {
      handled: true,
      action: 'no-generated-config',
      configKey: inspection.configKey,
      viteConfigPath,
    };
  }

  const mergeResult = mergeJsonConfig(viteConfigPath, generatedConfigPath, spec.configKey);
  if (!mergeResult.updated) {
    throw new Error(
      `Failed to merge ${path.basename(generatedConfigPath)} into ${path.basename(viteConfigPath)}`,
    );
  }

  fs.writeFileSync(viteConfigPath, mergeResult.content);
  fs.rmSync(generatedConfigPath, { force: true });
  await vpFmt(projectPath, path.relative(projectPath, viteConfigPath));
  return {
    handled: true,
    action: 'added',
    configKey: spec.configKey,
    viteConfigPath,
  };
}

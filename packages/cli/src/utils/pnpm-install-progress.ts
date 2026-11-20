import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';

import { resolveNpmRegistry } from '../../binding/index.js';
import { getVpCommand, runCommandSilently } from './command.ts';
import { readJsonFile } from './json.ts';
import { displayRelative } from './path.ts';
import { formatDuration } from './terminal.ts';

function isPnpmLoggerName(name: unknown): name is string {
  return typeof name === 'string' && (name === 'pnpm' || name.startsWith('pnpm:'));
}

type PnpmLog = Record<string, unknown> & { name: string };

const PNPM_MISC_LOGGERS = new Set([
  'pnpm',
  'pnpm:global',
  'pnpm:lockfile',
  'pnpm:store',
  'pnpm:registry',
]);

function parsePnpmLog(line: string): PnpmLog | undefined {
  if (!line.startsWith('{')) {
    return undefined;
  }
  try {
    const log: PnpmLog = JSON.parse(line);
    if (isPnpmLoggerName(log?.name)) {
      return log;
    }
  } catch {
    // The launcher and scripts with inherited stdio can also write plain text.
  }
  return undefined;
}

export function pnpmProbeEnv(envs: NodeJS.ProcessEnv): NodeJS.ProcessEnv {
  // The project hooks and preloads belong to the install.
  return { ...envs, NODE_OPTIONS: undefined, npm_config_ignore_pnpmfile: 'true' };
}

type PnpmInstallOutputMode = 'debug' | 'info' | 'warn' | 'error' | 'silent' | 'json' | 'ndjson';

export async function getPnpmInstallReporting(
  cwd: string,
  envs: NodeJS.ProcessEnv,
): Promise<{ mode: PnpmInstallOutputMode | undefined; useNativeReporter: boolean }> {
  const readConfig = async (args: string[], env: NodeJS.ProcessEnv) => {
    try {
      const { exitCode, stdout } = await runCommandSilently({
        ...getVpCommand(args),
        cwd,
        envs: env,
        idleTimeoutMs: 5000,
      });
      if (exitCode === 0) {
        return stdout.toString().trim().split(/\r?\n/).at(-1);
      }
    } catch {
      // Replay the normal diagnostics if the setting cannot be read.
    }
    return undefined;
  };
  const probeEnv = pnpmProbeEnv(envs);
  const [reporter, loglevel, json, parseable, pnpmfile, globalPnpmfile, lockfileDir] =
    await Promise.all(
      [
        'reporter',
        'loglevel',
        'json',
        'parseable',
        'pnpmfile',
        'global-pnpmfile',
        'lockfile-dir',
      ].map((key) => readConfig(['pm', 'config', 'get', key, '--location', 'project'], probeEnv)),
    );
  if (json === 'true' || parseable === 'true') {
    return { mode: 'json', useNativeReporter: false };
  }
  if (reporter === 'silent' || loglevel === 'silent') {
    return { mode: 'silent', useNativeReporter: false };
  }
  if ([reporter, loglevel, json, parseable].includes(undefined)) {
    return { mode: undefined, useNativeReporter: false };
  }
  if (reporter === 'ndjson') {
    return { mode: 'ndjson', useNativeReporter: false };
  }

  // pnpm runs filterLog inside its human reporters. Let pnpm own the output
  // when a configured file can supply a filter, with no extra hook execution.
  const directory = resolve(cwd, lockfileDir && lockfileDir !== 'undefined' ? lockfileDir : '.');
  const localFile = pnpmfile && pnpmfile !== 'undefined' ? pnpmfile : '.pnpmfile.cjs';
  const globalFile =
    globalPnpmfile && globalPnpmfile !== 'undefined'
      ? globalPnpmfile.replace(/^~[/\\]/, `${homedir()}/`)
      : undefined;
  const hasPnpmfile =
    existsSync(resolve(directory, localFile)) ||
    (globalFile !== undefined && existsSync(resolve(directory, globalFile)));
  // The config override skips the hook without replacing the raw setting.
  // `vp exec` forwards the flag to pnpm's config command.
  const ignorePnpmfile = hasPnpmfile
    ? await readConfig(
        [
          'exec',
          '--',
          'pnpm',
          'config',
          'get',
          'ignore-pnpmfile',
          '--location',
          'project',
          '--config._ignore-pnpmfile=true',
        ],
        { ...envs, NODE_OPTIONS: undefined },
      )
    : undefined;
  const mode =
    loglevel === 'debug' || loglevel === 'warn' || loglevel === 'error' ? loglevel : 'info';
  return { mode, useNativeReporter: hasPnpmfile && ignorePnpmfile !== 'true' };
}

function getPnpmLifecycleKey(log: Record<string, unknown> | undefined): string | undefined {
  if (
    log?.name === 'pnpm:lifecycle' &&
    typeof log.wd === 'string' &&
    typeof log.stage === 'string'
  ) {
    return JSON.stringify([log.wd, log.stage]);
  }
  return undefined;
}

export function watchPnpmResolution(): (line: string) => boolean {
  const resolving = new Set<string>();
  const runningScripts = new Set<string>();
  return (line) => {
    const log = parsePnpmLog(line);
    if (log?.name === 'pnpm:stage' && typeof log.prefix === 'string') {
      if (log.stage === 'resolution_started') {
        resolving.add(log.prefix);
      } else if (log.stage === 'resolution_done') {
        resolving.delete(log.prefix);
      }
    }
    // Resolving a Git dependency can invoke an install or prepare script.
    const script = getPnpmLifecycleKey(log);
    if (log && script) {
      if (typeof log.script === 'string') {
        runningScripts.add(script);
      } else if (typeof log.exitCode === 'number') {
        runningScripts.delete(script);
      }
    }
    return resolving.size > 0 && runningScripts.size === 0;
  };
}

function asRecord(value: unknown): Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

function getPnpmTerminalError(log: Record<string, unknown> | undefined) {
  // The CLI error handler names the terminal error. Recovered errors, such as
  // a repaired lockfile, can also be logged at error level.
  if (log?.name === 'pnpm' && log.level === 'error') {
    const error = asRecord(log.err);
    if (isPnpmLoggerName(error.name) && typeof error.message === 'string') {
      return {
        code: log.code ?? error.code ?? error.name,
        message: error.message,
      };
    }
  }
  return undefined;
}

function formatRequestRetry(log: Record<string, unknown>): string | undefined {
  const { method, url, timeout, attempt, maxRetries } = log;
  if (
    typeof method !== 'string' ||
    typeof url !== 'string' ||
    typeof timeout !== 'number' ||
    typeof attempt !== 'number' ||
    typeof maxRetries !== 'number'
  ) {
    return undefined;
  }
  const error = asRecord(log.error);
  const code = error.httpStatusCode ?? error.status ?? error.errno ?? error.code ?? error.message;
  const reason =
    typeof code === 'string' || typeof code === 'number'
      ? ` (${redactRegistryUrls(String(code), url)})`
      : '';
  const safeUrl = displayRegistryUrl(url) ?? '[registry URL]';
  const retriesLeft = maxRetries - attempt + 1;
  const retries = retriesLeft === 1 ? 'retry' : 'retries';
  return `${method} ${safeUrl} error${reason}. Will retry in ${formatDuration(timeout)}. ${retriesLeft} ${retries} left.`;
}

function appendPublishedVersions(messages: string[], packageMeta: unknown): void {
  const { name, 'dist-tags': distTags, versions } = asRecord(packageMeta);
  const tags = asRecord(distTags);
  if (typeof name !== 'string') {
    return;
  }
  if (typeof tags.latest === 'string') {
    messages.push(`The latest release of ${name} is "${tags.latest}".`);
  }
  const otherTags = Object.entries(tags).flatMap(([tag, version]) =>
    tag !== 'latest' && typeof version === 'string' ? [`  ${tag}: ${version}`] : [],
  );
  if (otherTags.length > 0) {
    messages.push('Other release tags', ...otherTags);
  }
  const versionCount = Object.keys(asRecord(versions)).length;
  if (versionCount > 0) {
    messages.push(
      `Run "pnpm view ${name} versions" to see all ${versionCount} published versions.`,
    );
  }
}

function displayRegistryUrl(value: string): string | undefined {
  try {
    const url = new URL(value);
    if (url.protocol === 'http:' || url.protocol === 'https:') {
      return `${url.origin}${url.pathname}`;
    }
  } catch {
    // pnpm may fail before a usable registry URL is available.
  }
  return undefined;
}

function redactRegistryUrls(text: string, requestUrl?: unknown): string {
  if (typeof requestUrl === 'string' && requestUrl.length > 0) {
    text = text.replaceAll(requestUrl, displayRegistryUrl(requestUrl) ?? '[registry URL]');
  }
  // Some pnpm errors only include the URL in their message. Keep the colon that ends a URL label.
  return text.replace(
    /https?:\/\/[^\s"'<>]+?(?=:\s|[\s"'<>]|$)/gi,
    (url) => displayRegistryUrl(url) ?? '[registry URL]',
  );
}

function appendRegistryAuthContext(
  messages: string[],
  log: Record<string, unknown>,
  cwd: string,
): void {
  if (typeof log.pkgName === 'string') {
    const scope = /^@([^/]+)\//.exec(log.pkgName)?.[1];
    try {
      const registry = resolveNpmRegistry(cwd, scope);
      const displayUrl = displayRegistryUrl(registry);
      if (displayUrl) {
        const key =
          scope && registry !== resolveNpmRegistry(cwd) ? `@${scope}:registry` : 'registry';
        messages.push(`${key}=${displayUrl}`);
      }
    } catch {
      // Keep the original failure and the guidance if the config cannot be read.
    }
  }
  messages.push(
    'Check the registry and authentication settings in .npmrc. Run "pnpm login" if you need to sign in.',
  );
}

function appendDependencyContext(messages: string[], log: Record<string, unknown>): void {
  if (!Array.isArray(log.pkgsStack)) {
    return;
  }
  const stack = log.pkgsStack.flatMap((entry: unknown) => {
    const { name, version } = asRecord(entry);
    if (typeof name !== 'string') {
      return [];
    }
    return [typeof version === 'string' ? `${name}@${version}` : name];
  });
  const [requester, ...ancestors] = stack;
  if (requester) {
    messages.push(`While installing the dependencies of ${requester}`);
    messages.push(...ancestors.map((ancestor) => `  at ${ancestor}`));
  } else if (typeof log.prefix === 'string') {
    messages.push(`While installing a direct dependency of ${log.prefix}`);
  }
}

function readPnpmPeerRules(cwd: string): unknown {
  try {
    const manifest = readJsonFile(join(cwd, 'package.json'));
    return asRecord(manifest.pnpm).peerDependencyRules;
  } catch {
    return undefined;
  }
}

async function formatPeerDependencyIssues(issuesByProjects: unknown, cwd: string): Promise<string> {
  const { renderPeerIssues } = await import('@pnpm/render-peer-issues');
  type Options = NonNullable<Parameters<typeof renderPeerIssues>[1]>;
  try {
    return renderPeerIssues(asRecord(issuesByProjects) as Parameters<typeof renderPeerIssues>[0], {
      rules: readPnpmPeerRules(cwd) as Options['rules'],
      width: process.stdout.columns ?? 80,
    }).trimEnd();
  } catch {
    // Keep the error headline if a record cannot be rendered.
    return '';
  }
}

export async function formatPnpmInstallOutput(
  output: string,
  cwd: string,
  mode: PnpmInstallOutputMode = 'info',
): Promise<string> {
  if (mode === 'ndjson') {
    return output;
  }
  const messages: string[] = [];
  const showInfo = mode === 'info' || mode === 'debug';
  const showWarnings = showInfo || mode === 'warn';
  const deprecatedSubdependencies: string[] = [];
  let terminalErrorIndex: number | undefined;
  const progress = { resolved: 0, found_in_store: 0, fetched: 0, imported: 0 };
  for (const line of output.split(/\r?\n/)) {
    const log = parsePnpmLog(line);
    if (!log) {
      messages.push(line);
    } else if (mode === 'json') {
      const error = getPnpmTerminalError(log);
      if (error) {
        if (terminalErrorIndex !== undefined) {
          messages.splice(terminalErrorIndex, 1);
        }
        terminalErrorIndex = messages.length;
        const message = redactRegistryUrls(error.message, asRecord(log.request).url);
        messages.push(JSON.stringify({ error: { code: error.code, message } }, null, 2));
      }
    } else if (log.name === 'pnpm:request-retry') {
      // pnpm's human reporter displays retry records as warnings.
      if (showWarnings) {
        const retry = formatRequestRetry(log);
        if (retry) {
          messages.push(retry);
        }
      }
    } else if (log.name === 'pnpm:deprecation') {
      // Deprecation events are logged at debug level and reported as warnings.
      const { depth, deprecated, pkgName, pkgVersion } = log;
      if (showWarnings && typeof pkgName === 'string' && typeof pkgVersion === 'string') {
        const dependency = `${pkgName}@${pkgVersion}`;
        if (depth === 0 && typeof deprecated === 'string') {
          messages.push(`WARN deprecated ${dependency}: ${deprecated}`);
        } else if (typeof depth === 'number' && depth > 0) {
          deprecatedSubdependencies.push(dependency);
        }
      }
    } else if (log.name === 'pnpm:stage' && log.stage === 'resolution_done') {
      if (deprecatedSubdependencies.length > 0) {
        const dependencies = deprecatedSubdependencies.toSorted().join(', ');
        messages.push(
          `WARN ${deprecatedSubdependencies.length} deprecated subdependencies found: ${dependencies}`,
        );
        deprecatedSubdependencies.length = 0;
      }
    } else if (log.name === 'pnpm:hook' && log.level === 'debug') {
      // readPackage's context.log() uses debug records in pnpm 9's info reporter.
      if (showInfo && typeof log.hook === 'string' && typeof log.message === 'string') {
        const label = typeof log.prefix === 'string' ? displayRelative(log.prefix, cwd) : '';
        const message = redactRegistryUrls(log.message);
        messages.push(label ? `${label} ${log.hook}: ${message}` : `${log.hook}: ${message}`);
      }
    } else if (mode === 'silent' && log.level === 'error') {
      // The silent reporter suppresses errors returned to the CLI and pnpm's
      // own diagnostics, but can still print an internal error from a hook.
      const error = asRecord(log.err);
      if (
        getPnpmTerminalError(log) ||
        (typeof error.code === 'string' && error.code.startsWith('ERR_PNPM_'))
      ) {
        continue;
      }
      const message = error.message ?? log.message;
      if (typeof message === 'string') {
        messages.push(redactRegistryUrls(message));
      }
      if (typeof error.stack === 'string') {
        messages.push(redactRegistryUrls(error.stack));
      }
    } else if (!showInfo && log.level !== 'error' && !(mode === 'warn' && log.level === 'warn')) {
      // The silent reporter leaves directly inherited script output visible.
      // Its lifecycle warnings and messages remain in the structured stream.
      continue;
    } else if (log.name === 'pnpm:progress') {
      if (showInfo && typeof log.status === 'string' && Object.hasOwn(progress, log.status)) {
        progress[log.status as keyof typeof progress]++;
      }
    } else if (log.name === 'pnpm:lifecycle' && typeof log.stage === 'string') {
      const directory =
        typeof log.wd === 'string' ? displayRelative(log.wd, cwd) || '.' : log.depPath;
      const label = typeof directory === 'string' ? `${directory} ${log.stage}` : log.stage;
      if (typeof log.script === 'string') {
        messages.push(`${label}$ ${log.script}`);
      } else if (typeof log.line === 'string') {
        messages.push(`${label}: ${log.line}`);
      } else if (typeof log.exitCode === 'number' && log.exitCode !== 0) {
        const optional = log.optional === true ? ' (optional dependency)' : '';
        messages.push(`${label}: failed with exit code ${log.exitCode}${optional}`);
      }
    } else if (
      log.name !== 'pnpm:install-check' &&
      (log.level !== 'debug' || (mode === 'debug' && PNPM_MISC_LOGGERS.has(log.name)))
    ) {
      const error = asRecord(log.err);
      const message = log.message ?? error.message;
      if (typeof message === 'string') {
        const errorCode = log.code ?? error.code;
        const code = typeof errorCode === 'string' ? errorCode : '';
        const authFailure = code === 'ERR_PNPM_FETCH_401' || code === 'ERR_PNPM_FETCH_403';
        const details =
          log.level === 'error' &&
          typeof error.stack === 'string' &&
          !code.startsWith('ERR_PNPM_') &&
          code !== 'ELIFECYCLE'
            ? error.stack
            : message;
        const requestUrl = asRecord(log.request).url;
        const displayed =
          log.level === 'error' || log.level === 'debug' || typeof requestUrl === 'string'
            ? redactRegistryUrls(details, requestUrl)
            : details;
        messages.push(code ? `${code} ${displayed}` : displayed);
        if (log.level === 'error') {
          appendDependencyContext(messages, log);
        }
        if (log.code === 'ERR_PNPM_PEER_DEP_ISSUES') {
          const details = await formatPeerDependencyIssues(log.issuesByProjects, cwd);
          if (details) {
            messages.push(details);
          }
        }
        if (code === 'ERR_PNPM_NO_MATCHING_VERSION') {
          appendPublishedVersions(messages, log.packageMeta);
        }
        if (typeof log.hint === 'string') {
          messages.push(
            redactRegistryUrls(
              log.hint.replace(
                /^An authorization header was used:.*$/gm,
                'An authorization header was used: [hidden]',
              ),
              requestUrl,
            ),
          );
        }
        if (authFailure) {
          appendRegistryAuthContext(messages, log, cwd);
        }
      }
    }
  }
  if (Object.values(progress).some((count) => count > 0)) {
    messages.push(
      `Progress: resolved ${progress.resolved}, reused ${progress.found_in_store}, downloaded ${progress.fetched}, added ${progress.imported}`,
    );
  }
  return messages.join('\n').trim();
}

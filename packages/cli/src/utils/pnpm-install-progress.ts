import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';

import { resolveNpmRegistry } from '../../binding/index.js';
import { MAX_TIMER_DELAY_MS, runCommandSilently } from './command.ts';
import { readJsonFile } from './json.ts';
import { displayRelative } from './path.ts';
import { formatDuration } from './terminal.ts';

export const PNPM_PROBE_TIMEOUT_MS = 5000;

function isPnpmLoggerName(name: unknown): name is string {
  return typeof name === 'string' && (name === 'pnpm' || name.startsWith('pnpm:'));
}

type PnpmLog = Record<string, unknown> & { name: string };

// pnpm's human reporter uses these streams for general diagnostics.
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

function parsePnpmDiagnostic(line: string): PnpmLog | undefined {
  const log = parsePnpmLog(line);
  if (log) {
    return log;
  }
  // pnpm can exit before its pipe drains a large error record. Match the
  // logger header and only trust a complete error code before the payload.
  const header =
    /^\{"time":\d+,"hostname":"(?:[^"\\]|\\.)*","pid":\d+,"level":"error","name":"pnpm"(?=,|$)/.exec(
      line,
    );
  if (!header) {
    return undefined;
  }
  const code = /^,"code":"([A-Z0-9_]+)"(?=,|$)/.exec(line.slice(header[0].length))?.[1];
  const details = 'pnpm did not finish writing the error details.';
  return {
    name: 'pnpm',
    level: 'error',
    code,
    err: {
      name: 'pnpm',
      message:
        code === 'ERR_PNPM_NO_MATCHING_VERSION' ? `No matching version found. ${details}` : details,
    },
  };
}

function splitPnpmLine(
  line: string,
  parse = parsePnpmLog,
): { text: string; log: PnpmLog | undefined } {
  const log = parse(line);
  if (log) {
    return { text: '', log };
  }
  // An inherited script can leave its final stdout line open. pnpm writes the
  // next record after that text, starting with the logger's timestamp and host.
  for (const { index } of line.matchAll(/\{"time":\d+,"hostname":/g)) {
    if (index) {
      const suffix = parse(line.slice(index));
      if (suffix) {
        return { text: line.slice(0, index), log: suffix };
      }
    }
  }
  return { text: line, log: undefined };
}

export function pnpmProbeEnv(envs: NodeJS.ProcessEnv): NodeJS.ProcessEnv {
  const probeEnv = { ...envs };
  const replaced = new Set([
    'node_options',
    'npm_config_ignore_pnpmfile',
    'pnpm_config_ignore_pnpmfile',
  ]);
  for (const key of Object.keys(probeEnv)) {
    // Windows spawn sorts keys and keeps only the first spelling of each name.
    if (replaced.has(key.toLowerCase())) {
      delete probeEnv[key];
    }
  }
  // pnpm 11 and later read pnpm_config_ instead of npm_config_.
  probeEnv.npm_config_ignore_pnpmfile = 'true';
  probeEnv.pnpm_config_ignore_pnpmfile = 'true';
  return probeEnv;
}

type PnpmInstallOutputMode = 'debug' | 'info' | 'warn' | 'error' | 'silent' | 'json' | 'ndjson';

interface PnpmLifecycleOutputOptions {
  aggregateOutput?: boolean;
  hideLifecyclePrefix?: boolean;
}

interface PnpmInstallReporting extends PnpmLifecycleOutputOptions {
  mode: PnpmInstallOutputMode;
  useNativeReporter: boolean;
  fetchTimeoutMs?: number;
}

export async function getPnpmInstallReporting(
  cwd: string,
  pnpmExecutable: string,
  envs: NodeJS.ProcessEnv,
): Promise<PnpmInstallReporting | undefined> {
  let config: Record<string, unknown>;
  try {
    const { exitCode, stdout } = await runCommandSilently({
      // Use the selected pnpm directly to avoid another launcher version check.
      // The overrides change the query without replacing the configured values.
      command: pnpmExecutable,
      args: [
        'config',
        'list',
        '--location',
        'project',
        '--config._json=true',
        '--config._ignore-pnpmfile=true',
      ],
      cwd,
      envs: { ...envs, NODE_OPTIONS: undefined },
      timeoutMs: PNPM_PROBE_TIMEOUT_MS,
    });
    if (exitCode !== 0) {
      return undefined;
    }
    const parsed: unknown = JSON.parse(stdout.toString());
    if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
      return undefined;
    }
    config = parsed as Record<string, unknown>;
  } catch {
    return undefined;
  }

  const { reporter, loglevel, json, parseable, pnpmfile, 'fetch-timeout': fetchTimeout } = config;
  const fetchTimeoutMs =
    typeof fetchTimeout === 'number' && fetchTimeout >= 1 && fetchTimeout <= MAX_TIMER_DELAY_MS
      ? fetchTimeout
      : undefined;
  // pnpm tests the resolved values for truthiness. Its config parser converts
  // true and false to booleans and keeps environment values like "1" as strings.
  if (json || parseable) {
    return { mode: 'json', useNativeReporter: false, fetchTimeoutMs };
  }
  if (reporter === 'silent' || loglevel === 'silent') {
    return { mode: 'silent', useNativeReporter: false, fetchTimeoutMs };
  }
  if (reporter === 'ndjson') {
    return { mode: 'ndjson', useNativeReporter: false, fetchTimeoutMs };
  }

  // pnpm runs filterLog inside its human reporters. Let pnpm own the output
  // when a configured file can supply a filter, with no extra hook execution.
  const lockfileDir = config['lockfile-dir'];
  const globalPnpmfile = config['global-pnpmfile'];
  const directory = resolve(cwd, typeof lockfileDir === 'string' ? lockfileDir : '.');
  const localFile = typeof pnpmfile === 'string' && pnpmfile ? pnpmfile : '.pnpmfile.cjs';
  const useNativeReporter =
    !config['ignore-pnpmfile'] &&
    (existsSync(resolve(directory, localFile)) ||
      (typeof globalPnpmfile === 'string' &&
        globalPnpmfile.length > 0 &&
        existsSync(resolve(directory, globalPnpmfile.replace(/^~[/\\]/, `${homedir()}/`)))));
  const mode =
    loglevel === 'debug' || loglevel === 'warn' || loglevel === 'error' ? loglevel : 'info';
  // pnpm defaults to append-only when stdout is captured.
  const appendOnly = !reporter || reporter === 'append-only';
  return {
    mode,
    useNativeReporter,
    fetchTimeoutMs,
    aggregateOutput: appendOnly && Boolean(config['aggregate-output']),
    hideLifecyclePrefix:
      (appendOnly || Boolean(config.stream)) && Boolean(config['reporter-hide-prefix']),
  };
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

export function watchPnpmResolution() {
  const resolving = new Set<string>();
  const runningScripts = new Set<string>();
  return (line: string, deferIdleTimeout?: (delayMs: number) => void) => {
    const { log } = splitPnpmLine(line);
    if (log?.name === 'pnpm:stage' && typeof log.prefix === 'string') {
      if (log.stage === 'resolution_started') {
        resolving.add(log.prefix);
      } else if (log.stage === 'resolution_done') {
        resolving.delete(log.prefix);
      }
    }
    if (
      log?.name === 'pnpm:request-retry' &&
      resolving.size > 0 &&
      typeof log.timeout === 'number'
    ) {
      // pnpm logs this delay after scheduling the next registry request.
      deferIdleTimeout?.(log.timeout);
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
      ? ` (${redactPnpmDiagnostic(String(code), url)})`
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

function redactPnpmDiagnostic(text: string, requestUrl?: unknown): string {
  return redactRegistryUrls(
    text.replace(/^([ \t]*An authorization header was used:)[^\r\n]*/gm, '$1 [hidden]'),
    requestUrl,
  );
}

function appendRegistryAuthContext(
  messages: string[],
  pkgName: unknown,
  requestUrl: unknown,
  cwd: string,
): void {
  if (typeof pkgName === 'string' && typeof requestUrl === 'string') {
    const scope = /^@([^/]+)\//.exec(pkgName)?.[1];
    try {
      const registry = resolveNpmRegistry(cwd, scope);
      const displayUrl = displayRegistryUrl(registry);
      const packagePath = encodeURIComponent(pkgName).replace(/^%40/, '@');
      // The launcher and pnpm can load different npmrc files. Only show the
      // configured registry when it matches the actual package metadata URL.
      if (
        displayUrl &&
        displayRegistryUrl(requestUrl) ===
          new URL(packagePath, displayUrl.replace(/\/?$/, '/')).href
      ) {
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

function appendDependencyContext(
  messages: string[],
  log: Record<string, unknown>,
  cwd: string,
): void {
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
    const member = typeof log.prefix === 'string' ? displayRelative(log.prefix, cwd) : '';
    if (member) {
      messages.push(`In ${member}`);
    }
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

async function formatPeerDependencyIssues(
  issuesByProjects: unknown,
  cwd: string,
  kind: 'warning' | 'error',
): Promise<string> {
  const { renderPeerIssues } = await import('@pnpm/render-peer-issues');
  type Options = NonNullable<Parameters<typeof renderPeerIssues>[1]>;
  try {
    const issues = asRecord(issuesByProjects) as Parameters<typeof renderPeerIssues>[0];
    const details = renderPeerIssues(issues, {
      rules: readPnpmPeerRules(cwd) as Options['rules'],
      width: process.stdout.columns ?? 80,
    }).trimEnd();
    if (!details || kind === 'warning') {
      return details;
    }
    const hints = [];
    const hasMissingPeers = Object.values(issues).some(({ missing }) =>
      Object.values(missing).some((entries) => entries.some(({ optional }) => !optional)),
    );
    if (hasMissingPeers) {
      hints.push(
        'hint: To install missing peer dependencies automatically, set "auto-install-peers=true" in the project .npmrc.',
      );
    }
    hints.push(
      'hint: To allow installation with peer dependency issues, set "strict-peer-dependencies=false" in the project .npmrc.',
    );
    return [details, '', ...hints].join('\n');
  } catch {
    // Keep the error headline if a record cannot be rendered.
    return '';
  }
}

export async function formatPnpmInstallOutput(
  output: string,
  cwd: string,
  mode: PnpmInstallOutputMode = 'info',
  lifecycle: PnpmLifecycleOutputOptions = {},
): Promise<string> {
  if (mode === 'ndjson') {
    return output;
  }
  const messages: string[] = [];
  const showInfo = mode === 'info' || mode === 'debug';
  const showWarnings = showInfo || mode === 'warn';
  const deprecatedSubdependencies: string[] = [];
  const pendingScripts = new Map<string, string[]>();
  const infoErrorIndices = new Map<string, number[]>();
  let seenScope = false;
  let seenPeerIssues = false;
  let terminalErrorIndex: number | undefined;
  const emptyProgress = { resolved: 0, found_in_store: 0, fetched: 0, imported: 0 };
  const progressByRequester = new Map<string, typeof emptyProgress>();
  for (const line of output.split(/\r?\n/)) {
    const { text, log } = splitPnpmLine(line, parsePnpmDiagnostic);
    if (text || !log) {
      messages.push(text);
    }
    if (!log) {
      continue;
    }
    if (mode === 'json') {
      const error = getPnpmTerminalError(log);
      if (error) {
        if (terminalErrorIndex !== undefined) {
          messages.splice(terminalErrorIndex, 1);
        }
        terminalErrorIndex = messages.length;
        const message = redactRegistryUrls(error.message, asRecord(log.request).url);
        messages.push(JSON.stringify({ error: { code: error.code, message } }, null, 2));
      }
    } else if (log.name === 'pnpm:scope') {
      // The human reporter uses the first scope record for this install.
      if (!showInfo || seenScope) {
        continue;
      }
      seenScope = true;
      const { selected, total, workspacePrefix } = log;
      if (typeof selected !== 'number' || selected === 1) {
        continue;
      }
      let count = String(selected);
      if (selected === total) {
        count = `all ${selected}`;
      } else if (typeof total === 'number' && total > 0) {
        count += ` of ${total}`;
      }
      const projects =
        typeof workspacePrefix === 'string' && workspacePrefix ? 'workspace projects' : 'projects';
      messages.push(`Scope: ${count} ${projects}`);
    } else if (log.name === 'pnpm:request-retry') {
      // pnpm's human reporter displays retry records as warnings.
      if (showWarnings) {
        const retry = formatRequestRetry(log);
        if (retry) {
          messages.push(retry);
        }
      }
    } else if (log.name === 'pnpm:peer-dependency-issues') {
      // pnpm's human reporter consumes only the first peer issues record.
      if (showWarnings && !seenPeerIssues) {
        seenPeerIssues = true;
        const details = await formatPeerDependencyIssues(log.issuesByProjects, cwd, 'warning');
        if (details) {
          messages.push(
            `WARN Issues with peer dependencies found\n${redactPnpmDiagnostic(details)}`,
          );
        }
      }
    } else if (log.name === 'pnpm:deprecation') {
      // Deprecation events are logged at debug level and reported as warnings.
      const { depth, deprecated, pkgName, pkgVersion } = log;
      if (showWarnings && typeof pkgName === 'string' && typeof pkgVersion === 'string') {
        const dependency = redactRegistryUrls(`${pkgName}@${pkgVersion}`);
        if (depth === 0 && typeof deprecated === 'string') {
          const prefix = typeof log.prefix === 'string' ? displayRelative(log.prefix, cwd) : '';
          const warning = `WARN deprecated ${dependency}: ${redactPnpmDiagnostic(deprecated)}`;
          messages.push(prefix ? `${prefix}: ${warning}` : warning);
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
    } else if (log.name === 'pnpm:skipped-optional-dependency') {
      // pnpm reports direct skips from the current importer at info level.
      if (
        showInfo &&
        log.prefix === cwd &&
        Array.isArray(log.parents) &&
        log.parents.length === 0
      ) {
        const { id, name, version, pref } = asRecord(log.package);
        const nameAndVersion =
          typeof name === 'string' && name && typeof version === 'string'
            ? `${name}@${version}`
            : '';
        const dependency = (typeof id === 'string' && id) || nameAndVersion || pref;
        if (typeof dependency === 'string' && dependency) {
          messages.push(
            `info: ${redactPnpmDiagnostic(dependency)} is an optional dependency and failed compatibility check. Excluding it from installation.`,
          );
        }
      }
    } else if (log.name === 'pnpm:hook' && log.level === 'debug') {
      // readPackage's context.log() uses debug records in pnpm 9's info reporter.
      if (showInfo && typeof log.hook === 'string' && typeof log.message === 'string') {
        const label = typeof log.prefix === 'string' ? displayRelative(log.prefix, cwd) : '';
        const message = redactPnpmDiagnostic(log.message);
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
        messages.push(redactPnpmDiagnostic(message));
      }
      if (typeof error.stack === 'string') {
        messages.push(redactPnpmDiagnostic(error.stack));
      }
    } else if (!showInfo && log.level !== 'error' && !(mode === 'warn' && log.level === 'warn')) {
      // The silent reporter leaves directly inherited script output visible.
      // Its lifecycle warnings and messages remain in the structured stream.
      continue;
    } else if (log.name === 'pnpm:progress') {
      if (showInfo && typeof log.status === 'string' && Object.hasOwn(emptyProgress, log.status)) {
        const requester = typeof log.requester === 'string' && log.requester ? log.requester : cwd;
        let progress = progressByRequester.get(requester);
        if (progress === undefined) {
          progress = { ...emptyProgress };
          progressByRequester.set(requester, progress);
        }
        progress[log.status as keyof typeof emptyProgress]++;
      }
    } else if (log.name === 'pnpm:lifecycle' && typeof log.stage === 'string') {
      const directory =
        typeof log.wd === 'string' ? displayRelative(log.wd, cwd) || '.' : log.depPath;
      const label =
        typeof directory === 'string' ? `${redactRegistryUrls(directory)} ${log.stage}` : log.stage;
      const key = lifecycle.aggregateOutput ? getPnpmLifecycleKey(log) : undefined;
      const scriptOutput = key === undefined ? messages : (pendingScripts.get(key) ?? []);
      if (typeof log.script === 'string') {
        scriptOutput.push(`${label}$ ${log.script}`);
      } else if (typeof log.line === 'string') {
        scriptOutput.push(lifecycle.hideLifecyclePrefix ? log.line : `${label}: ${log.line}`);
      } else if (typeof log.exitCode === 'number') {
        if (log.exitCode === 0) {
          scriptOutput.push(`${label}: Done`);
        } else {
          const optional = log.optional === true ? ' (optional dependency)' : '';
          scriptOutput.push(`${label}: failed with exit code ${log.exitCode}${optional}`);
        }
      }
      if (key !== undefined) {
        if (typeof log.exitCode === 'number') {
          pendingScripts.delete(key);
          if (scriptOutput.length > 0) {
            messages.push(scriptOutput.join('\n'));
          }
        } else {
          pendingScripts.set(key, scriptOutput);
        }
      }
    } else if (PNPM_MISC_LOGGERS.has(log.name) && (log.level !== 'debug' || mode === 'debug')) {
      const error = asRecord(log.err);
      // pnpm scopes ordinary info messages to the current importer.
      if (
        log.level === 'info' &&
        typeof log.prefix === 'string' &&
        log.prefix &&
        log.prefix !== cwd &&
        typeof error.message !== 'string'
      ) {
        continue;
      }
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
        const requestUrl = asRecord(log.request).url ?? log.url;
        const displayed = redactPnpmDiagnostic(details, requestUrl);
        const headline = code ? `${code} ${displayed}` : displayed;
        const diagnosticPath =
          typeof log.prefix === 'string' &&
          (log.level === 'warn' || getPnpmTerminalError(log) !== undefined)
            ? displayRelative(log.prefix, cwd)
            : '';
        // pnpm logs each recursive failure at info level before it reports member paths.
        if (log.name === 'pnpm' && log.level === 'info' && typeof error.message === 'string') {
          const key = JSON.stringify([code, message]);
          const indices = infoErrorIndices.get(key) ?? [];
          indices.push(messages.length);
          infoErrorIndices.set(key, indices);
        }
        messages.push(diagnosticPath ? `${diagnosticPath}: ${headline}` : headline);
        if (code === 'ERR_PNPM_RECURSIVE_FAIL' && Array.isArray(log.failures)) {
          for (const failure of log.failures) {
            const { prefix, message: reason, error: memberError } = asRecord(failure);
            if (typeof prefix === 'string' && typeof reason === 'string') {
              const member = redactRegistryUrls(displayRelative(prefix, cwd) || '.');
              const details = asRecord(memberError);
              const url = asRecord(details.request).url ?? details.url;
              const memberCode = typeof details.code === 'string' ? details.code : '';
              const index = infoErrorIndices.get(JSON.stringify([memberCode, reason]))?.shift();
              if (index === undefined) {
                messages.push(`${member}: ${redactPnpmDiagnostic(reason, url)}`);
              } else {
                messages[index] = `${member}: ${redactPnpmDiagnostic(messages[index], url)}`;
              }
            }
          }
        }
        const context: string[] = [];
        if (log.level === 'error') {
          appendDependencyContext(context, log, cwd);
        }
        if (log.code === 'ERR_PNPM_PEER_DEP_ISSUES') {
          const details = await formatPeerDependencyIssues(log.issuesByProjects, cwd, 'error');
          if (details) {
            context.push(details);
          }
        }
        if (code === 'ERR_PNPM_NO_MATCHING_VERSION') {
          appendPublishedVersions(context, log.packageMeta);
        }
        if (typeof log.hint === 'string') {
          context.push(log.hint);
        } else if (log.level === 'error' && code === 'ERR_PNPM_LOCKFILE_BREAKING_CHANGE') {
          context.push('', 'Run with the --force parameter to recreate the lockfile.');
        }
        if (authFailure) {
          appendRegistryAuthContext(context, log.pkgName, requestUrl, cwd);
        }
        messages.push(...context.map((message) => redactPnpmDiagnostic(message, requestUrl)));
      }
    }
  }
  for (const [requester, progress] of progressByRequester) {
    const label = redactRegistryUrls(displayRelative(requester, cwd));
    const summary = `Progress: resolved ${progress.resolved}, reused ${progress.found_in_store}, downloaded ${progress.fetched}, added ${progress.imported}`;
    messages.push(label ? `${label} | ${summary}` : summary);
  }
  return messages.join('\n').trim();
}

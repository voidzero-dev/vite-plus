import { join } from 'node:path';

import { readJsonFile } from './json.ts';

function parsePnpmLog(line: string): Record<string, unknown> | undefined {
  if (!line.startsWith('{')) {
    return undefined;
  }
  try {
    const log: Record<string, unknown> = JSON.parse(line);
    if (typeof log?.name === 'string' && (log.name === 'pnpm' || log.name.startsWith('pnpm:'))) {
      return log;
    }
  } catch {
    // The launcher and scripts with inherited stdio can also write plain text.
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
    if (
      log?.name === 'pnpm:lifecycle' &&
      typeof log.wd === 'string' &&
      typeof log.stage === 'string'
    ) {
      const script = JSON.stringify([log.wd, log.stage]);
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

export async function formatPnpmInstallOutput(output: string, cwd: string): Promise<string> {
  const messages: string[] = [];
  const progress = { resolved: 0, found_in_store: 0, fetched: 0, imported: 0 };
  for (const line of output.split(/\r?\n/)) {
    const log = parsePnpmLog(line);
    if (!log) {
      messages.push(line);
    } else if (log.name === 'pnpm:progress') {
      if (typeof log.status === 'string' && Object.hasOwn(progress, log.status)) {
        progress[log.status as keyof typeof progress]++;
      }
    } else if (log.name === 'pnpm:lifecycle' && typeof log.stage === 'string') {
      if (typeof log.script === 'string') {
        messages.push(`${log.stage}$ ${log.script}`);
      } else if (typeof log.line === 'string') {
        messages.push(`${log.stage}: ${log.line}`);
      }
    } else if (log.level !== 'debug' && log.name !== 'pnpm:install-check') {
      const error = log.err as { message?: unknown } | undefined;
      const message = log.message ?? error?.message;
      if (typeof message === 'string') {
        const code = typeof log.code === 'string' ? `${log.code} ` : '';
        messages.push(`${code}${message}`);
        if (log.level === 'error') {
          appendDependencyContext(messages, log);
        }
        if (log.code === 'ERR_PNPM_PEER_DEP_ISSUES') {
          const details = await formatPeerDependencyIssues(log.issuesByProjects, cwd);
          if (details) {
            messages.push(details);
          }
        }
        if (typeof log.hint === 'string') {
          messages.push(log.hint);
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

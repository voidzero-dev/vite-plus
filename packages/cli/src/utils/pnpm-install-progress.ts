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

function appendPeerDependencyIssues(messages: string[], issuesByProjects: unknown): void {
  for (const [project, value] of Object.entries(asRecord(issuesByProjects))) {
    const issues = asRecord(value);
    const conflicts = Array.isArray(issues.conflicts) ? issues.conflicts : [];
    const intersections = asRecord(issues.intersections);
    const missing = Object.entries(asRecord(issues.missing)).filter(
      ([name]) => conflicts.includes(name) || Object.hasOwn(intersections, name),
    );
    const peers = [...Object.entries(asRecord(issues.bad)), ...missing];
    const location = project === '.' ? '' : ` in ${project}`;

    for (const [name, entries] of peers) {
      if (!Array.isArray(entries)) {
        continue;
      }
      for (const entry of entries) {
        const issue = asRecord(entry);
        if (typeof issue.wantedRange !== 'string') {
          continue;
        }
        const parent = asRecord(Array.isArray(issue.parents) ? issue.parents.at(-1) : undefined);
        const requester =
          typeof parent.name === 'string' && typeof parent.version === 'string'
            ? `${parent.name}@${parent.version} requires `
            : '';
        const found =
          typeof issue.foundVersion === 'string' ? `found ${issue.foundVersion}` : 'missing';
        messages.push(`  ${requester}${name}@${issue.wantedRange} (${found})${location}`);
      }
    }
  }
}

export function formatPnpmInstallOutput(output: string): string {
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
        if (log.code === 'ERR_PNPM_PEER_DEP_ISSUES') {
          appendPeerDependencyIssues(messages, log.issuesByProjects);
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

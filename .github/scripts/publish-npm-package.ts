import { spawn } from 'node:child_process';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import {
  type FetchLike,
  type NpmPackageVersion,
  DEFAULT_NPM_REGISTRY,
  isNpmPackagePublished,
  parseNpmPackageSpec,
} from './wait-for-npm-packages.ts';

export interface PublishCommandResult {
  exitCode: number | null;
  output: string;
  error?: Error;
}

export type PublishCommandRunner = (
  command: string,
  args: readonly string[],
  cwd: string,
) => Promise<PublishCommandResult>;

export interface PublishNpmPackageOptions {
  pkg: NpmPackageVersion;
  command: string;
  args: readonly string[];
  cwd: string;
  registry: string;
  fetchImpl: FetchLike;
  runCommand: PublishCommandRunner;
  log: (message: string) => void;
  warn: (message: string) => void;
}

export type PublishNpmPackageResult = 'published' | 'already-published';

/** Matches npm's immutable-version errors without swallowing unrelated 403s. */
export function isAlreadyPublishedError(output: string): boolean {
  return (
    /you cannot publish over the previously published versions?/i.test(output) ||
    /EPUBLISHCONFLICT/i.test(output)
  );
}

/**
 * Publishes one package idempotently. A registry lookup handles normal reruns;
 * the error check handles the scan window where npm has accepted the version
 * but still hides it from registry reads.
 */
export async function publishNpmPackage(
  options: PublishNpmPackageOptions,
): Promise<PublishNpmPackageResult> {
  const spec = `${options.pkg.name}@${options.pkg.version}`;

  try {
    if (
      await isNpmPackagePublished(options.pkg, {
        registry: options.registry,
        fetchImpl: options.fetchImpl,
      })
    ) {
      options.log(`${spec} is already published; skipping upload.`);
      return 'already-published';
    }
  } catch (error) {
    // A transient read failure must not prevent the publish attempt. If this is
    // a rerun, npm's immutable-version response is handled below.
    options.warn(`Could not check whether ${spec} is published; trying upload (${String(error)})`);
  }

  const result = await options.runCommand(options.command, options.args, options.cwd);
  if (result.exitCode === 0) {
    return 'published';
  }
  if (isAlreadyPublishedError(result.output)) {
    options.log(`${spec} was accepted by an earlier attempt; skipping upload.`);
    return 'already-published';
  }

  const detail = result.error?.message ?? `exit code ${String(result.exitCode)}`;
  throw new Error(`Failed to publish ${spec}: ${detail}`);
}

function runPublishCommand(
  command: string,
  args: readonly string[],
  cwd: string,
): Promise<PublishCommandResult> {
  return new Promise((resolveResult) => {
    const child = spawn(command, args, { cwd, env: process.env });
    let output = '';

    child.stdout?.on('data', (chunk: Buffer) => {
      output += chunk.toString();
      process.stdout.write(chunk);
    });
    child.stderr?.on('data', (chunk: Buffer) => {
      output += chunk.toString();
      process.stderr.write(chunk);
    });
    child.on('error', (error) => {
      resolveResult({ exitCode: null, output, error });
    });
    child.on('close', (exitCode) => {
      resolveResult({ exitCode, output });
    });
  });
}

export async function publishNpmPackageFromEnv(
  pkg: NpmPackageVersion,
  command: string,
  args: readonly string[],
  cwd = process.cwd(),
): Promise<PublishNpmPackageResult> {
  return publishNpmPackage({
    pkg,
    command,
    args,
    cwd,
    registry: process.env.PUBLISH_REGISTRY ?? DEFAULT_NPM_REGISTRY,
    fetchImpl: fetch,
    runCommand: runPublishCommand,
    log: console.log,
    warn: console.warn,
  });
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const separator = args.indexOf('--');
  if (separator !== 1 || args.length <= separator + 1) {
    throw new Error(
      'Usage: node .github/scripts/publish-npm-package.ts <name@version> -- <command> [args...]',
    );
  }

  const spec = args[0];
  const command = args[separator + 1];
  if (!spec || !command) {
    throw new Error('A package version and publish command are required.');
  }
  await publishNpmPackageFromEnv(parseNpmPackageSpec(spec), command, args.slice(separator + 2));
}

const invokedPath = process.argv[1];
if (invokedPath && pathToFileURL(resolve(invokedPath)).href === import.meta.url) {
  main().catch((error: unknown) => {
    console.error(`::error::${error instanceof Error ? error.message : String(error)}`);
    process.exitCode = 1;
  });
}

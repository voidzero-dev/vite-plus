import type { ClientRequest } from 'node:http';

// A stalled upstream must release local-package metadata lookups and let package
// managers retry proxied requests before a snapshot step reaches its 120s limit.
const UPSTREAM_REQUEST_TIMEOUT_MS = 15_000;

export function setRegistryRequestTimeout(
  request: ClientRequest,
  timeout = UPSTREAM_REQUEST_TIMEOUT_MS,
): void {
  const abort = () => {
    request.destroy(new Error(`Registry upstream request timed out after ${timeout}ms`));
  };
  // Bound DNS, connection setup, and the wait for response headers too; a socket
  // timeout alone does not cover all of those stages.
  const timer = setTimeout(abort, timeout);
  timer.unref();
  // Once headers arrive, allow downloads to continue as long as data arrives.
  request.setTimeout(timeout, abort);
  request.once('response', () => clearTimeout(timer));
  request.once('close', () => clearTimeout(timer));
}

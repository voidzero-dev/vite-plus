import type { ClientRequest } from 'node:http';

// A stalled upstream must release local-package metadata lookups and let package
// managers retry proxied requests before a snapshot step reaches its 120s limit.
const UPSTREAM_REQUEST_TIMEOUT_MS = 15_000;

export function setRegistryRequestDeadline(
  request: ClientRequest,
  timeout = UPSTREAM_REQUEST_TIMEOUT_MS,
): void {
  // Socket timeouts do not cover DNS, connection setup, or waiting for an agent
  // socket. Keep this deadline active until the response body finishes too.
  const timer = setTimeout(() => {
    request.destroy(new Error(`Registry upstream request timed out after ${timeout}ms`));
  }, timeout);
  timer.unref();
  request.once('close', () => clearTimeout(timer));
}

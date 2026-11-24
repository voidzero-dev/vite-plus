import { readFileSync } from 'node:fs';

import type { JsonObject } from '../packages/cli/src/utils/json-value.ts';
import { isJsonObject, parseJsonObject } from '../packages/cli/src/utils/json-value.ts';

/** Tooling scripts can run before the workspace dependencies are installed. */
export function requireJsonObject(value: unknown, source: string): JsonObject {
  if (!isJsonObject(value)) {
    throw new TypeError(`Expected a JSON object in ${source}`);
  }
  return value;
}

export function readJsonObject(path: string): JsonObject {
  return parseJsonObject(readFileSync(path, 'utf8'));
}

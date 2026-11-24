import type { JsonObject } from '../../utils/json.ts';
import { isJsonObject } from '../../utils/json.ts';

/** Require an object at a particular path in a migrated JSON or YAML fixture. */
export function fixtureObject(value: unknown, ...path: Array<string>): JsonObject {
  let object = value;
  for (const key of path) {
    if (!isJsonObject(object)) {
      throw new TypeError(`Expected a fixture object before key ${key}`);
    }
    object = object[key];
  }
  if (!isJsonObject(object)) {
    throw new TypeError(`Expected a fixture object at ${path.join('.') || 'the root'}`);
  }
  return object;
}

export function fixtureString(value: unknown, parent: string, key: string): string {
  const item = fixtureObject(value, parent)[key];
  if (typeof item !== 'string') {
    throw new TypeError(`Expected a string at ${parent}.${key}`);
  }
  return item;
}

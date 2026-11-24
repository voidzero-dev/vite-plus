export type JsonValue = string | number | boolean | null | Array<JsonValue> | JsonObject;

export interface JsonObject {
  [key: string]: JsonValue | undefined;
}

export function isPlainObject(value: unknown): value is object {
  if (value === null || typeof value !== 'object') {
    return false;
  }
  const prototype: unknown = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}

function isJsonValue(value: unknown, ancestors: Set<object>): boolean {
  if (
    value === null ||
    typeof value === 'string' ||
    typeof value === 'boolean' ||
    typeof value === 'number'
  ) {
    return true;
  }
  if (typeof value !== 'object' || ancestors.has(value)) {
    return false;
  }
  const array = Array.isArray(value);
  if (!array && !isPlainObject(value)) {
    return false;
  }
  ancestors.add(value);
  const valid = Object.values(value).every(
    (entry: unknown) => (!array && entry === undefined) || isJsonValue(entry, ancestors),
  );
  ancestors.delete(value);
  return valid;
}

export function isJsonObject(value: unknown): value is JsonObject {
  return (
    value !== null &&
    typeof value === 'object' &&
    !Array.isArray(value) &&
    isJsonValue(value, new Set())
  );
}

/** Narrow the root of a value that already came from a JSON parser. */
export function isObjectValue(value: JsonValue | undefined): value is JsonObject {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

export function parseJsonObject(text: string, source?: string): JsonObject {
  const value = parseJsonValue(text);
  if (!isObjectValue(value)) {
    throw new TypeError(
      source === undefined ? 'Expected a JSON object' : `Expected a JSON object in ${source}`,
    );
  }
  return value;
}

export function isStringRecord(value: unknown): value is Record<string, string> {
  return (
    isPlainObject(value) &&
    Object.values(value).every((entry: unknown) => typeof entry === 'string')
  );
}

export function parseJsonValue(text: string): JsonValue {
  // oxlint-disable-next-line typescript/no-unsafe-return -- JSON.parse without a reviver only produces JSON values.
  return JSON.parse(text);
}

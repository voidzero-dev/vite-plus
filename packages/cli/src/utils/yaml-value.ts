import { isPlainObject } from './json-value.ts';

export interface YamlMapping {
  // oxlint-disable-next-line quality/no-unsafe-dictionary-type -- YAML mappings can contain tagged values and recursive aliases. Callers validate the keys they read.
  [key: string]: unknown;
}

export function isYamlMapping(value: unknown): value is YamlMapping {
  return isPlainObject(value);
}

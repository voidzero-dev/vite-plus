import { readFileSync } from 'node:fs';

import type { JsonObject } from '../../cli/src/utils/json-value.ts';
import { isObjectValue, parseJsonObject, isStringRecord } from '../../cli/src/utils/json-value.ts';

export const dependencyFields = [
  'dependencies',
  'devDependencies',
  'optionalDependencies',
  'peerDependencies',
] as const;

export interface PackageManifest extends JsonObject {
  name?: string;
  version?: string;
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
  optionalDependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
  peerDependenciesMeta?: JsonObject;
  bundledVersions?: JsonObject;
}

function isPackageManifest(value: JsonObject): value is PackageManifest {
  return (
    [value.name, value.version].every(
      (field) => field === undefined || typeof field === 'string',
    ) &&
    dependencyFields.every((field) => value[field] === undefined || isStringRecord(value[field])) &&
    [value.peerDependenciesMeta, value.bundledVersions].every(
      (field) => field === undefined || isObjectValue(field),
    )
  );
}

export function readPackageManifest(file: string): PackageManifest {
  const value = parseJsonObject(readFileSync(file, 'utf8'));
  if (!isPackageManifest(value)) {
    throw new Error(`Invalid package manifest in ${file}`);
  }
  return value;
}

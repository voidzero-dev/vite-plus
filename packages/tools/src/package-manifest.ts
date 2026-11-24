import { readFileSync } from 'node:fs';

import type { JsonObject } from '../../cli/src/utils/json-value.ts';
import { isObjectValue, parseJsonObject, isStringRecord } from '../../cli/src/utils/json-value.ts';

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
    (value.name === undefined || typeof value.name === 'string') &&
    (value.version === undefined || typeof value.version === 'string') &&
    [
      value.dependencies,
      value.devDependencies,
      value.optionalDependencies,
      value.peerDependencies,
    ].every((deps) => deps === undefined || isStringRecord(deps)) &&
    (value.peerDependenciesMeta === undefined || isObjectValue(value.peerDependenciesMeta)) &&
    (value.bundledVersions === undefined || isObjectValue(value.bundledVersions))
  );
}

export function readPackageManifest(file: string): PackageManifest {
  const value = parseJsonObject(readFileSync(file, 'utf8'));
  if (!isPackageManifest(value)) {
    throw new Error(`Invalid package manifest in ${file}`);
  }
  return value;
}

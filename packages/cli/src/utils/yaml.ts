import fs from 'node:fs';

import type { Document } from 'yaml';
import { parseDocument, parse as parseYaml, Scalar } from 'yaml';

import type { YamlMapping } from './yaml-value.ts';
import { isYamlMapping } from './yaml-value.ts';

export { isYamlMapping };
export type { YamlMapping };

export function readYamlFile(file: string): YamlMapping | null {
  const content = fs.readFileSync(file, 'utf8');
  const value: unknown = parseYaml(content);
  if (value !== null && !isYamlMapping(value)) {
    throw new TypeError(`Expected a YAML mapping in ${file}`);
  }
  return value;
}

export type YamlDocument = Document.Parsed;

export function editYamlFile(file: string, callback: (doc: YamlDocument) => void) {
  const content = fs.readFileSync(file, 'utf8');
  const doc = parseDocument(content);
  callback(doc);
  // prefer single quotes
  fs.writeFileSync(file, doc.toString({ singleQuote: true }), 'utf8');
}

export function scalarString(value: string): Scalar<string> {
  return new Scalar(value);
}

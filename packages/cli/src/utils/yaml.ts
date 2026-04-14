import fs from 'node:fs';

import { type Document, parseDocument, parse as parseYaml, Scalar } from 'yaml';

export function readYamlFile(file: string): Record<string, unknown> {
  const content = fs.readFileSync(file, 'utf-8');
  return parseYaml(content);
}

export type YamlDocument = Document.Parsed;

export function editYamlFile(file: string, callback: (doc: YamlDocument) => void) {
  const content = fs.readFileSync(file, 'utf-8');
  const doc = parseDocument(content);
  callback(doc);
  // prefer single quotes
  fs.writeFileSync(file, doc.toString({ singleQuote: true }), 'utf-8');
}

export function scalarString(value: string): Scalar<string> {
  return new Scalar(value);
}

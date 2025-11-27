import fs from 'node:fs';

import { type Document, type ParsedNode, parseDocument, parse as parseYaml, Scalar } from 'yaml';

export function readYamlFile<T = Record<string, any>>(file: string): T {
  const content = fs.readFileSync(file, 'utf-8');
  return parseYaml(content) as T;
}

export type YamlDocument = Document.Parsed<ParsedNode, true>;

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

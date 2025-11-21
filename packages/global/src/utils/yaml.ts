import fs from 'node:fs';

import { parse as parseYaml } from '@std/yaml';
import { type Document, type ParsedNode, parseDocument, Scalar } from 'yaml';

export function readYamlFile<T = Record<string, any>>(file: string): T {
  const content = fs.readFileSync(file, 'utf-8');
  return parseYaml(content) as T;
}

export function editYamlFile(
  file: string,
  callback: (doc: Document.Parsed<ParsedNode, true>) => void,
) {
  const content = fs.readFileSync(file, 'utf-8');
  const doc = parseDocument(content);
  callback(doc);
  // prefer single quotes
  fs.writeFileSync(file, doc.toString({ singleQuote: true }), 'utf-8');
}

export function scalarString(value: string): Scalar<string> {
  return new Scalar(value);
}

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { parseDocument } from 'yaml';

import { readJsonFile, writeJsonFile as writeJson } from '../utils/json.ts';
import { type YamlDocument } from '../utils/yaml.ts';
import { isMigrationIgnored, migrationScope, sharedDependencyFields } from './scope.ts';

export { readJsonFile, isJsonFile } from '../utils/json.ts';
export { readYamlFile, scalarString, type YamlDocument } from '../utils/yaml.ts';

function ignored(file: fs.PathLike | number): boolean {
  return (
    typeof file !== 'number' &&
    isMigrationIgnored(file instanceof URL ? fileURLToPath(file) : file.toString())
  );
}

// Keep migration writes behind one boundary. Reads still see excluded files:
// shared dependency ownership must be determined from the complete project.
const migrationFs: typeof fs = Object.assign(Object.create(fs) as typeof fs, {
  writeFileSync(file, ...args) {
    if (!ignored(file)) {
      fs.writeFileSync(file, ...args);
    }
  },
  appendFileSync(file, ...args) {
    if (!ignored(file)) {
      fs.appendFileSync(file, ...args);
    }
  },
  unlinkSync(file) {
    if (!ignored(file)) {
      fs.unlinkSync(file);
    }
  },
  rmSync(file, ...args) {
    if (!ignored(file)) {
      fs.rmSync(file, ...args);
    }
  },
  renameSync(from, to) {
    if (!ignored(from) && !ignored(to)) {
      fs.renameSync(from, to);
    }
  },
} satisfies Pick<
  typeof fs,
  'writeFileSync' | 'appendFileSync' | 'unlinkSync' | 'rmSync' | 'renameSync'
>);
export default migrationFs;

function preserveShared(file: string): boolean {
  const scope = migrationScope();
  return (
    !!scope?.preserveSharedDependencies &&
    path.dirname(file) === scope.root &&
    ['package.json', 'pnpm-workspace.yaml', '.yarnrc.yml'].includes(path.basename(file))
  );
}

export function writeJsonFile(file: string, data: Record<string, unknown>): void {
  if (isMigrationIgnored(file)) {
    return;
  }
  if (preserveShared(file) && fs.existsSync(file)) {
    const before = parseDocument(fs.readFileSync(file, 'utf8'));
    const after = parseDocument(JSON.stringify(data));
    for (const keys of sharedDependencyFields) {
      if (before.hasIn(keys)) {
        after.setIn(keys, before.getIn(keys));
      } else if (after.hasIn(keys)) {
        after.deleteIn(keys);
      }
    }
    data = after.toJS() as Record<string, unknown>;
  }
  writeJson(file, data);
}

export function editJsonFile<T extends Record<string, unknown> = Record<string, unknown>>(
  file: string,
  callback: (content: T) => T | undefined,
): void {
  if (isMigrationIgnored(file)) {
    return;
  }
  const updated = callback(readJsonFile(file) as T);
  if (updated) {
    writeJsonFile(file, updated);
  }
}

export function editYamlFile(file: string, callback: (doc: YamlDocument) => void): void {
  if (isMigrationIgnored(file)) {
    return;
  }
  const before = parseDocument(fs.readFileSync(file, 'utf8'));
  const after = parseDocument(fs.readFileSync(file, 'utf8'));
  callback(after);
  if (preserveShared(file)) {
    for (const keys of sharedDependencyFields) {
      if (before.hasIn(keys)) {
        after.setIn(keys, before.getIn(keys));
      } else if (after.hasIn(keys)) {
        after.deleteIn(keys);
      }
    }
  }
  fs.writeFileSync(file, after.toString({ singleQuote: true }), 'utf8');
}

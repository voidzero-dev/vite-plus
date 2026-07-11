import fs from 'node:fs';
import path from 'node:path';

import configEntryFiles from '../vite-config-entry-basenames.json' with { type: 'json' };

export const VITE_CONFIG_ENTRY_FILES = configEntryFiles;
export const VITE_CONFIG_ENTRY_BASENAMES = new Set(VITE_CONFIG_ENTRY_FILES.map((file) => path.basename(file)));

function isWithinStopDir(dir: string, stopDir: string): boolean {
  const relative = path.relative(stopDir, dir);
  return relative === '' || (!relative.startsWith('..') && !path.isAbsolute(relative));
}

export function findSupportedConfigFile(dir: string): string | undefined {
  for (const filename of VITE_CONFIG_ENTRY_FILES) {
    const fullPath = path.join(dir, filename);
    if (fs.existsSync(fullPath)) {
      return fullPath;
    }
  }
  return undefined;
}

export function findSupportedConfigFileUp(startDir: string, stopDir: string): string | undefined {
  let dir = path.resolve(startDir);
  const stop = path.resolve(stopDir);

  while (true) {
    const configFile = findSupportedConfigFile(dir);
    if (configFile) {
      return configFile;
    }

    if (dir === stop) {
      break;
    }

    const parent = path.dirname(dir);
    if (parent === dir || !isWithinStopDir(parent, stop)) {
      break;
    }
    dir = parent;
  }

  return undefined;
}

export function hasSupportedConfigFile(dir: string): boolean {
  return findSupportedConfigFile(dir) !== undefined;
}

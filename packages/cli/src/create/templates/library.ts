import path from 'node:path';

import { VITE_PLUS_NAME, VITE_PLUS_VERSION } from '../../utils/constants.ts';
import { editJsonFile } from '../../utils/json.ts';

export function prepareLibraryPackage(projectPath: string, packageName: string): void {
  // The remote template can lag behind the CLI. Use the creating CLI's version
  // before project integration writes its core override or catalog references.
  editJsonFile<{ name?: string; devDependencies?: Record<string, string> }>(
    path.join(projectPath, 'package.json'),
    (pkg) => {
      pkg.name = packageName;
      pkg.devDependencies ??= {};
      pkg.devDependencies[VITE_PLUS_NAME] = VITE_PLUS_VERSION;
      return pkg;
    },
  );
}

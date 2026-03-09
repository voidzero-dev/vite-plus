import fs from 'node:fs';
import path from 'node:path';

/**
 * Check if tsconfig.json has compilerOptions.baseUrl set.
 * oxlint's TypeScript checker (tsgolint) does not support baseUrl,
 * so typeAware/typeCheck must be disabled when it is present.
 */
export function hasBaseUrlInTsconfig(projectPath: string): boolean {
  try {
    const tsconfig = JSON.parse(
      fs.readFileSync(path.join(projectPath, 'tsconfig.json'), 'utf-8'),
    ) as { compilerOptions?: { baseUrl?: string } };
    return tsconfig?.compilerOptions?.baseUrl !== undefined;
  } catch {
    return false;
  }
}

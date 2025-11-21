import path from 'node:path';

// Get the package root directory (packages/global)
// Built files are in dist/, templates are in templates/
// So from dist/ we need to go up to the package root
export const pkgRoot = import.meta.dirname.endsWith('dist')
  ? path.dirname(import.meta.dirname)
  : path.join(import.meta.dirname, '../..');

export const templatesDir = path.join(pkgRoot, 'templates');
export const rulesDir = path.join(pkgRoot, 'rules');

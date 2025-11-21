export function getScopeFromPackageName(packageName: string): string {
  if (packageName.startsWith('@')) {
    return packageName.split('/')[0];
  }
  return '';
}

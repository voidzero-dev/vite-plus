// Extract the package name an override/resolution key targets. This mirrors the
// package-manager grammars for bare names, versioned descriptors, pnpm parent
// selectors, and Yarn from/target selectors.
export function extractOverrideTargetName(key: string): string {
  let target = key.trim();

  // pnpm uses `>` between parent and child selectors, except when it is part of
  // a comparator such as `pkg@>4` or `pkg@>=4`.
  for (
    let delimiter = target.search(/[^ |@]>/);
    delimiter !== -1;
    delimiter = target.search(/[^ |@]>/)
  ) {
    target = target.slice(delimiter + 2).trim();
  }
  if (!target) {
    return target;
  }

  // Yarn uses `from/target`; retain a trailing scoped target as one name.
  if (target.includes('/')) {
    const segments = target.split('/');
    const last = segments[segments.length - 1];
    const scope = segments[segments.length - 2];
    target = scope?.startsWith('@') ? `${scope}/${last}` : last;
  }

  // The leading `@` of a scope is not a version delimiter.
  const nameStart = target.startsWith('@') ? target.indexOf('/') + 1 : 0;
  const versionAt = target.indexOf('@', nameStart);
  return versionAt > 0 ? target.slice(0, versionAt) : target;
}

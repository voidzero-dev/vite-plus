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
    const last = segments.at(-1)!;
    const scope = segments[segments.length - 2];
    target = scope?.startsWith('@') ? `${scope}/${last}` : last;
  }

  // The leading `@` of a scope is not a version delimiter.
  const nameStart = target.startsWith('@') ? target.indexOf('/') + 1 : 0;
  const versionAt = target.indexOf('@', nameStart);
  return versionAt > 0 ? target.slice(0, versionAt) : target;
}

// Strip a trailing `@version`/range from a selector segment and keep its scope.
// Mirrors the version-suffix peeling in `extractOverrideTargetName`: the version
// `@` follows the name (after the `/` of a scoped name); the leading scope `@`
// is never a version separator.
function stripSegmentVersion(segment: string): string {
  const nameStart = segment.startsWith('@') ? segment.indexOf('/') + 1 : 0;
  const versionAt = segment.indexOf('@', nameStart);
  return versionAt > 0 ? segment.slice(0, versionAt) : segment;
}

// Extract the ordered PARENT chain of an override/resolution key — the ancestor
// segments above the forced TARGET — or `null` when the key has no parent
// selector (a bare/versioned global pin). Each segment's own `@version`/range is
// stripped and scoped names (`@scope/name`) are kept whole; glob segments (`**`,
// `vite-*`) are preserved verbatim for `parentChainReachesVitePlus`.
//
// Mirrors `extractOverrideTargetName`'s grammar so target and parent stay
// consistent (see that function for the full delimiter rationale):
//   - pnpm `a>b>child`: every `>`-separated prefix is a parent level (`a`, `b`);
//     pnpm has no globs, so a chain of length > 1 always carries a specific
//     ancestor.
//   - yarn `from/descriptor`: the descriptor is the trailing 1 (unscoped) or 2
//     (scoped) segments; the remaining leading `/`-segments are the `from` chain,
//     with scoped ancestors (`@scope/name`) rejoined.
//   - bare/versioned names (`pkg`, `@scope/pkg`, `pkg@4`) have NO parent → `null`.
export function extractOverrideParentSegments(key: string): string[] | null {
  let rest = key.trim();
  // Peel every pnpm `>` parent level. pnpm splits at a `>` whose preceding char
  // is NOT space, `|`, or `@` (its DELIMITER_REGEX), so semver comparators like
  // `pkg@>=4` are not mistaken for a parent selector.
  const pnpmParents: string[] = [];
  for (let delim = rest.search(/[^ |@]>/); delim !== -1; delim = rest.search(/[^ |@]>/)) {
    pnpmParents.push(stripSegmentVersion(rest.slice(0, delim + 1).trim()));
    rest = rest.slice(delim + 2).trim();
  }
  if (pnpmParents.length > 0) {
    return pnpmParents;
  }
  // No pnpm parent — check for a yarn `from/descriptor` selector. `rest` is the
  // child (target) descriptor; only a `/` beyond a single scoped name leaves a
  // leading `from` (parent) chain.
  if (!rest.includes('/')) {
    return null;
  }
  const segments = rest.split('/');
  // The trailing descriptor occupies the last 2 segments when it is a scoped
  // name (second-to-last segment starts with `@`), else the last 1.
  const descriptorIsScoped = segments[segments.length - 2]?.startsWith('@') ?? false;
  const descriptorSegmentCount = descriptorIsScoped ? 2 : 1;
  const rawParents = segments.slice(0, segments.length - descriptorSegmentCount);
  if (rawParents.length === 0) {
    // The whole key was a bare scoped name (`@scope/pkg`) — no parent selector.
    return null;
  }
  // Rejoin scoped ancestors (`@scope` + `name`) and strip each segment's version.
  const parents: string[] = [];
  for (let i = 0; i < rawParents.length; i += 1) {
    const segment = rawParents[i];
    if (segment.startsWith('@') && i + 1 < rawParents.length) {
      parents.push(stripSegmentVersion(`${segment}/${rawParents[i + 1]}`));
      i += 1;
    } else {
      parents.push(stripSegmentVersion(segment));
    }
  }
  return parents;
}

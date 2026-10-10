// Cargo.toml version helpers shared by `sync-remote` and `patch-oxc-node`. Keep this
// module free of package imports: CI runs `patch-oxc-node` before installing dependencies.

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

// Parse every `oxc*` crate version from a Cargo.toml in a single pass, keyed by
// crate name. Handles both the bare `oxc_x = "X"` form and the inline-table
// `oxc = { version = "X", features = [...] }` form (which may span lines, but
// always lists `version` before its `[...]` feature array, so `[^}]*?` reaches
// it without crossing the closing `}`).
export function parseCargoOxcVersions(src: string): Map<string, string> {
  const versions = new Map<string, string>();
  const re = /^\s*(oxc[\w-]*)\s*=\s*(?:"([^"]+)"|\{[^}]*?version\s*=\s*"([^"]+)")/gm;
  for (let m = re.exec(src); m; m = re.exec(src)) {
    versions.set(m[1], m[2] ?? m[3]);
  }
  return versions;
}

// Replace the version of a single crate entry in-place, preserving features,
// formatting, and surrounding text. Only the first matching entry is rewritten.
export function replaceCargoCrateVersion(src: string, key: string, newVersion: string): string {
  const escaped = escapeRegExp(key);
  const bareRe = new RegExp(`^(\\s*${escaped}\\s*=\\s*")[^"]+(")`, 'm');
  if (bareRe.test(src)) {
    return src.replace(bareRe, `$1${newVersion}$2`);
  }
  const tableStart = new RegExp(`^\\s*${escaped}\\s*=\\s*\\{`, 'm').exec(src);
  if (!tableStart) {
    return src;
  }
  const start = tableStart.index;
  const close = src.indexOf('}', start);
  const end = close >= 0 ? close : src.length;
  const table = src.slice(start, end).replace(/(version\s*=\s*")[^"]+(")/, `$1${newVersion}$2`);
  return src.slice(0, start) + table + src.slice(end);
}

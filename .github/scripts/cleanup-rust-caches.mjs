const ref = 'refs/heads/main';
const unusedMilliseconds = 48 * 60 * 60 * 1000;

function family(cache) {
  // Keep the job, OS, architecture, Rust/environment hash, and archive version
  // separate. Only the final manifest hash can differ within one family.
  const match =
    /^v1-rust-.+-(?:Linux|Darwin|Windows_NT)-(?:x64|arm64)-[a-f0-9]{8}-[a-f0-9]{8}$/.exec(
      cache.key,
    );
  if (cache.ref !== ref || !match || !cache.version) {
    return undefined;
  }
  return `${cache.key.slice(0, -9)}:${cache.version}`;
}

export function supersededRustCaches(caches, now = Date.now()) {
  const newest = new Map();
  for (const cache of caches) {
    const group = family(cache);
    const created = Date.parse(cache.created_at);
    if (group && created > (newest.get(group) ?? -Infinity)) {
      newest.set(group, created);
    }
  }
  const cutoff = now - unusedMilliseconds;
  return caches.filter(
    (cache) =>
      Date.parse(cache.created_at) < newest.get(family(cache)) &&
      Date.parse(cache.created_at) < cutoff &&
      Date.parse(cache.last_accessed_at) < cutoff,
  );
}

export async function cleanupRustCaches({ github, context, core }, { dryRun = true } = {}) {
  // This workflow runs trusted main code with actions:write. PR caches are
  // handled separately by the existing closed-PR job.
  if (context.ref !== ref) {
    throw new Error('Rust cache cleanup must run on main');
  }
  const list = async (key) =>
    github.paginate(github.rest.actions.getActionsCacheList, {
      ...context.repo,
      ref,
      key,
      per_page: 100,
      sort: 'created_at',
      direction: 'desc',
    });
  const now = Date.now();
  const candidates = supersededRustCaches(await list('v1-rust-'), now);
  let count = 0;
  let bytes = 0;
  for (const candidate of candidates) {
    if (!dryRun) {
      // Recheck recent use and the replacement before deleting. The initial
      // snapshot may be stale after pagination or another cleanup.
      const current = supersededRustCaches(await list(candidate.key.slice(0, -8)), now);
      if (!current.some((cache) => cache.id === candidate.id)) {
        continue;
      }
      try {
        await github.rest.actions.deleteActionsCacheById({
          ...context.repo,
          cache_id: candidate.id,
        });
      } catch (error) {
        if (error.status === 404) {
          continue;
        }
        throw error;
      }
    }
    core.info(
      `${dryRun ? 'Would delete' : 'Deleted'} ${candidate.key} (${candidate.size_in_bytes} B)`,
    );
    count++;
    bytes += candidate.size_in_bytes;
  }
  core.info(
    `${dryRun ? 'Eligible' : 'Deleted'}: ${count} caches, ${(bytes / 1024 ** 2).toFixed(1)} MiB`,
  );
  return { count, bytes };
}

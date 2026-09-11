export function patchDecodersNodeRuntime(workflow: string): string {
  // Temporary ecosystem-only patch: decoders still tests Node 20 at the pinned
  // revision, which blocks the Vitest v5 migration even when our CI uses Node 24.
  // Upgrade that matrix entry without changing the library's public engines.
  // Remove this helper and its call when repo.json selects an upstream revision
  // whose CI matrix no longer includes Node 20.
  const node20 = /^([ \t]*)'20\.x',[^\r\n]*$/gm;
  if (workflow.match(node20)?.length !== 1) {
    throw new Error('decoders patch: review the temporary Node 20 CI matrix override');
  }
  return workflow.replace(node20, "$1'26.x',");
}

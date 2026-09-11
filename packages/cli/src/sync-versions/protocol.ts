import { z } from 'zod';

import { parseSyncVersionsRequest, planSyncVersions, type SyncVersionsToolchain } from './plan.ts';

const exactVersionSchema = z
  .string()
  .regex(/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/);

const toolchainManifestSchema = z
  .object({
    schemaVersion: z.literal(1),
    nodes: z.array(
      z
        .object({
          id: z.string().min(1),
          version: exactVersionSchema.optional(),
        })
        .passthrough(),
    ),
  })
  .passthrough();

function requiredToolVersion(
  nodes: z.infer<typeof toolchainManifestSchema>['nodes'],
  id: string,
): string {
  const matches = nodes.filter((node) => node.id === id);
  if (matches.length !== 1 || matches[0].version === undefined) {
    throw new Error(`Toolchain manifest must contain one exact ${id} version`);
  }
  return matches[0].version;
}

export function toolchainFromManifest(input: unknown): SyncVersionsToolchain {
  const manifest = toolchainManifestSchema.parse(input);
  return {
    vitePlus: requiredToolVersion(manifest.nodes, 'vite-plus'),
    vitest: requiredToolVersion(manifest.nodes, 'vitest'),
  };
}

export function runSyncVersionsProtocol(requestJson: string, manifest: unknown): string {
  let input: unknown;
  try {
    input = JSON.parse(requestJson);
  } catch {
    throw new Error('Invalid sync request JSON');
  }
  const request = parseSyncVersionsRequest(input);
  const toolchain = toolchainFromManifest(manifest);
  const plan = planSyncVersions(request, toolchain);
  return `${JSON.stringify(plan)}\n`;
}

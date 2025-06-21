import { createPkgGraph, type Package } from "@pnpm/workspace.pkgs-graph";
import { sortPackages } from "@pnpm/sort-packages";
import { glob } from "node:fs/promises";
import { dirname, join } from "node:path";
import type { BaseManifest } from "@pnpm/types";

const cwd = process.cwd();

type Task = { dir: string; script: string; manifest: BaseManifest };

export async function getTaskList(taskNames: string[]): Promise<Task[][]> {
  const packages: Package[] = [];

  for await (const filePath of glob("{apps,packages}/*/package.json", { cwd })) {
    const absPath = join(cwd, filePath);
    const { default: manifest } = await import(absPath, { with: { type: "json" } });
    packages.push({ rootDir: dirname(absPath), manifest });
  }

  const withScript = packages.filter(
    pkg => pkg.manifest.scripts && taskNames.some(task => pkg.manifest.scripts && task in pkg.manifest.scripts)
  );

  const { graph } = createPkgGraph(withScript);

  const ordered = sortPackages(graph);

  const split = ordered.map(dirs =>
    dirs.flatMap(dir =>
      taskNames.flatMap(task => {
        const manifest = graph[dir].package.manifest;
        if (manifest?.scripts && task in manifest.scripts) return { dir, script: manifest.scripts[task], manifest };
        return [];
      })
    )
  );

  return split;
}

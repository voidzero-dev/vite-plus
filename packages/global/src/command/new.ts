import { cp } from "node:fs/promises";
import { join } from "node:path";

export default async function copyTemplateFiles(targetDir: string) {
  await copyFiles(targetDir);
}

async function copyFiles(targetDir: string) {
  const templateDir = join(import.meta.dirname, "../template");
  await cp(templateDir, targetDir, { force: true, recursive: true });
}

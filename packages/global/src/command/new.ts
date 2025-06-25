import { cp, readdir } from "node:fs/promises";
import { join } from "node:path";

const templatesDir = join(import.meta.dirname, "../../templates");

export async function getAvailableTemplates(): Promise<string[]> {
  const dirs = await readdir(templatesDir);
  return dirs;
}

export async function copyTemplateFiles(templateDir: string, targetDir: string): Promise<void> {
  await copyFiles(templateDir, targetDir);
}

async function copyFiles(templateDir: string, targetDir: string): Promise<void> {
  await cp(templateDir, targetDir, { force: true, recursive: true });
}

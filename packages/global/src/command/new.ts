import { cp, readdir } from "node:fs/promises";
import { join } from "node:path";
import { spawn } from "node:child_process";
import { intro, select, outro, text, confirm, tasks, cancel } from "@clack/prompts";

const cwd = process.cwd();
const templatesDir = join(import.meta.dirname, "../templates");

async function getAvailableTemplates(): Promise<string[]> {
  const dirs = await readdir(templatesDir);
  return dirs;
}

async function copyTemplateFiles(templateDir: string, targetDir: string): Promise<void> {
  await copyFiles(templateDir, targetDir);
}

async function copyFiles(templateDir: string, targetDir: string): Promise<void> {
  await cp(templateDir, targetDir, { force: true, recursive: true });
}

export const questionnaire = async (): Promise<void> => {
  intro("Let's create a new Vite+ project");

  const availableTemplates = await getAvailableTemplates();

  const targetDir = await text({
    message: "Where should we create your project?",
    placeholder: "./",
    initialValue: "./",
    validate(value) {
      if (!value || value.startsWith("..") || !value.startsWith(".")) return "Please enter a relative path";
    }
  });

  const isUseTypeScript = await select({
    message: "Do you plan to use TypeScript?",
    options: [
      { value: "ts", label: "TypeScript" },
      { value: "js", label: "JavaScript with JSDoc" }
    ]
  });

  const templateDir = await select({
    message: "Please choose a project template",
    options: availableTemplates.map(template => ({
      value: template,
      label: template
    }))
  });

  const isInstallDependencies = await confirm({
    message: "Do you want to install dependencies?",
    initialValue: true
  });

  const taskList = [
    {
      title: "Copying template files",
      task: async () => {
        const sourceTemplateDir = join(templatesDir, templateDir);
        const targetDirPath = join(cwd, targetDir);
        await copyTemplateFiles(sourceTemplateDir, targetDirPath);
        return "Copied template files";
      }
    }
  ];

  if (isInstallDependencies) {
    taskList.push({
      title: "Installing dependencies",
      task: async () => {
        const exitCode = await new Promise((resolve, reject) => {
          const targetDirPath = join(cwd, targetDir);
          const p = spawn("pnpm", ["install"], { cwd: targetDirPath });
          p.on("exit", resolve);
          p.on("error", reject);
        });
        if (exitCode !== 0) {
          cancel("Failed to install dependencies");
          process.exit(exitCode);
        }
        return "Installed dependencies using pnpm";
      }
    });
  }

  await tasks(taskList);

  outro("Enjoy Vite+");
};

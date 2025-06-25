import { join } from "node:path";
import { spawn } from "node:child_process";
import { intro, select, outro, text, confirm, tasks } from "@clack/prompts";
import { getAvailableTemplates, copyTemplateFiles } from "./new.ts";

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

  const t = [
    {
      title: "Copying template files",
      task: async () => {
        const sourceTemplateDir = join(import.meta.dirname, "../../templates", templateDir);
        const targetDirPath = join(process.cwd(), targetDir);
        await copyTemplateFiles(sourceTemplateDir, targetDirPath);
        return "Copied template files";
      }
    }
  ];

  if (isInstallDependencies) {
    t.push({
      title: "Installing dependencies",
      task: async () => {
        await new Promise(resolve => {
          const targetDirPath = join(process.cwd(), targetDir);
          const p = spawn("pnpm", ["install"], { cwd: targetDirPath });
          p.on("exit", resolve);
        });
        return "Installed dependencies using pnpm";
      }
    });
  }

  await tasks(t);

  outro("Enjoy Vite+");
};

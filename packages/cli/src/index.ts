import { spawnSync } from "node:child_process";
import path from "node:path";
import { parseArgs } from "node:util";

function execPackageBin(binName: string, args: string[], cwd: string, dir?: string) {
  const program = path.join(cwd, "node_modules/.bin", binName);
  exec(program, args, dir ? path.join(cwd, dir) : cwd);
}

function exec(program: string, args: string[], cwd?: string) {
  const { status, error } = spawnSync(program, args, { stdio: "inherit", cwd });
  if (error !== undefined) {
    throw error;
  }
  process.exit(status ?? 255);
}

export default function main(): void {
  const { positionals } = parseArgs({ allowPositionals: true });

  const [command, ...rest] = positionals;

  const tasks = rest.filter((task) => task.includes("#"));

  const [subcommand, packageDir] = command === "task" && tasks.length > 0 ? tasks[0].split("#") : [command, "."];

  const cwd = process.cwd();

  const index = process.argv.indexOf("--");
  const commandArgs = index === -1 ? [] : process.argv.slice(index + 1);

  switch (subcommand) {
    case "run":
      exec(process.execPath, ["--import", import.meta.resolve("@oxc-node/core/register"), ...commandArgs], cwd);
    case "build":
      execPackageBin("vite", ["build", ...commandArgs], cwd, packageDir);
    case "optimize":
      execPackageBin("vite", ["optimize", ...commandArgs], cwd, packageDir);
    case "preview":
      execPackageBin("vite", ["preview", ...commandArgs], cwd, packageDir);
    case "dev":
      execPackageBin("vite", ["dev", ...commandArgs], cwd, packageDir);
    case "lint":
      execPackageBin("oxlint", commandArgs, cwd, packageDir);
    case "test":
      execPackageBin("vitest", commandArgs, cwd, packageDir);
    case "bench":
      execPackageBin("vitest", ["bench", ...commandArgs], cwd, packageDir);
    case "docs":
      execPackageBin("vitepress", commandArgs, cwd, packageDir);
  }
}

import { spawnSync } from "node:child_process";
import path, { join } from "node:path";
import { parseArgs } from "node:util";
import { multiplex } from "multiplexer";
import type { Command } from "multiplexer";

function execPackageBin(binName: string, args: string[], cwd: string, dir?: string) {
  const program = path.join(import.meta.dirname, "../node_modules/.bin", binName);
  exec(program, args, dir ? path.join(cwd, dir) : cwd);
}

function exec(program: string, args: string[], cwd?: string) {
  const { status, error } = spawnSync(program, args, { stdio: "inherit", cwd });
  if (error !== undefined) {
    throw error;
  }
  process.exit(status ?? 255);
}

function getArgs(subcommand: string, commandArgs: string[]) {
  switch (subcommand) {
    case "build":
      return { command: "vite", args: ["build", ...commandArgs] };
    case "optimize":
      return { command: "vite", args: ["optimize", ...commandArgs] };
    case "preview":
      return { command: "vite", args: ["preview", ...commandArgs] };
    case "dev":
      return { command: "vite", args: ["dev", ...commandArgs] };
    case "lint":
      return { command: "oxlint", args: commandArgs };
    case "test":
      return { command: "vitest", args: commandArgs };
    case "bench":
      return { command: "vitest", args: ["bench", ...commandArgs] };
    case "docs":
      return { command: "vitepress", args: commandArgs };
    default:
      return { command: subcommand, args: commandArgs };
  }
}

export default function main(): void {
  const { positionals } = parseArgs({ allowPositionals: true });

  const [command, ...rest] = positionals;

  const tasks = rest.filter(task => task.includes("#"));

  const [subcommand, packageDir] = command === "task" && tasks.length > 0 ? tasks[0].split("#") : [command, "."];

  const cwd = process.cwd();

  const index = process.argv.indexOf("--");
  const commandArgs = index === -1 ? [] : process.argv.slice(index + 1);

  if (tasks.length > 1) {
    const env = { ...process.env, FORCE_COLOR: "true" };
    const commands: Command[] = tasks.flatMap(task => {
      const [subcommand, packageDir] = task.split("#");
      if (!subcommand) return [];

      if (subcommand === "exec") {
        // for demo purposes only
        const [cmd, ...args] = packageDir.split(" ");
        return [{ name: task, cmd, args, cwd, env }];
      }
      const { command, args = [] } = getArgs(subcommand, [...commandArgs]);

      if (command === "vitest") args.push("--watch"); // for demo purposes only

      const program = join(import.meta.dirname, "../node_modules/.bin", command);
      return [{ name: task, cmd: program, args, cwd: join(cwd, packageDir), env }];
    });

    multiplex(commands);
  } else {
    switch (subcommand) {
      case "run":
        exec(process.execPath, ["--import", import.meta.resolve("@oxc-node/core/register"), ...commandArgs], cwd);
      default: {
        const { command, args = [] } = getArgs(subcommand, commandArgs);
        if (command) execPackageBin(command, args, cwd, packageDir);
      }
    }
  }
}

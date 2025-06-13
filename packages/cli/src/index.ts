import { spawnSync } from "node:child_process";
import { join } from "node:path";
import { parseArgs } from "node:util";
import { multiplex, type Command } from "multiplexer";

function execPackageBin(binName: string, args: string[], cwd: string, dir?: string) {
  const program = join(import.meta.dirname, "../node_modules/.bin", binName);
  exec(program, args, dir ? join(cwd, dir) : cwd);
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
      return { command: "vite", mode: "stream", args: ["build", ...commandArgs] };
    case "optimize":
      return { command: "vite", mode: "stream", args: ["optimize", ...commandArgs] };
    case "preview":
      return { command: "vite", mode: "stream", args: ["preview", ...commandArgs] };
    case "dev":
      return { command: "vite", mode: "watch", args: ["dev", ...commandArgs] };
    case "lint":
      return { command: "oxlint", mode: "stream", args: commandArgs };
    case "test":
      return { command: "vitest", mode: "watch", args: commandArgs };
    case "bench":
      return { command: "vitest", mode: "watch", args: ["bench", ...commandArgs] };
    case "docs":
      return { command: "vitepress", mode: "watch", args: commandArgs };
    default:
      return { command: subcommand, mode: "stream", args: commandArgs };
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
        return [{ name: task, cmd, args, cwd, env, mode: "watch" }];
      }
      const { command, args = [], mode } = getArgs(subcommand, [...commandArgs]);

      if (command === "vitest" && mode === "watch") args.push("--watch"); // for demo purposes only

      const program = join(import.meta.dirname, "../node_modules/.bin", command);
      return [{ name: task, cmd: program, args, cwd: join(cwd, packageDir), env, mode }];
    });

    multiplex(commands);
  } else {
    switch (subcommand) {
      case "run":
        exec(process.execPath, ["--import", import.meta.resolve("@oxc-node/core/register"), ...commandArgs], cwd);
        break;
      default: {
        const { command, args = [] } = getArgs(subcommand, commandArgs);
        if (command) execPackageBin(command, args, cwd, packageDir);
      }
    }
  }
}

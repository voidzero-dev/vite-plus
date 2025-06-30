import { join } from "node:path";
import { parseArgs } from "node:util";
import { multiplex, type Command } from "multiplexer";
import { getTaskList } from "./getTasks.ts";
import { spawn, spawnSync } from "node:child_process";

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
      return { command: "vite", args: ["build", ...commandArgs] };
    case "optimize":
      return { command: "vite", args: ["optimize", ...commandArgs] };
    case "preview":
      return { command: "vite", args: ["preview", ...commandArgs] };
    case "dev":
      return { command: "vite", args: ["dev", ...commandArgs] };
    case "lint":
      return { command: "oxlint", args: commandArgs };
    case "lib":
      return { command: "tsdown", args: commandArgs };
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

export async function run(): Promise<void> {
  const { positionals } = parseArgs({ allowPositionals: true });

  const [subcommand, ...rest] = positionals;

  if (subcommand === "task") {
    const taskList = await getTaskList(rest);
    const commands: Command[][] = [];
    const env = { ...process.env, FORCE_COLOR: "true" };

    for (const tasks of taskList) {
      commands.push(
        tasks.map(task => {
          const [binName, ...args] = task.script.split(" ");
          return {
            name: binName,
            cmd: join(task.dir, "node_modules/.bin", binName),
            args: args,
            cwd: task.dir,
            env
          };
        })
      );
    }

    const all = commands.flat();
    if (all.length > 1) multiplex(commands);
    else if (all.length === 1) spawn(all[0].cmd, all[0].args, { stdio: "inherit" });
    else console.error("404 Task Not Found");
  } else {
    const cwd = process.cwd();
    const index = process.argv.indexOf("--");
    const commandArgs = index === -1 ? [] : process.argv.slice(index + 1);
    const [dir = "."] = rest;

    switch (subcommand) {
      case "run":
        exec(process.execPath, ["--import", import.meta.resolve("@oxc-node/core/register"), ...commandArgs], cwd);
        break;
      default: {
        const { command, args = [] } = getArgs(subcommand, commandArgs);
        if (command) execPackageBin(command, args, cwd, dir);
      }
    }
  }
}

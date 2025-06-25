import { join } from "node:path";
import { parseArgs } from "node:util";
import { multiplex, type Command } from "multiplexer";
import { getTaskList } from "./getTasks.ts";
import { spawn } from "node:child_process";

export async function runTasks(): Promise<void> {
  const { positionals } = parseArgs({ allowPositionals: true });

  const [command, ...taskNames] = positionals;

  if (command === "task") {
    const taskList = await getTaskList(taskNames);
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
  }
}

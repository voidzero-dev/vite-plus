import { join } from "node:path";
import { multiplex, type Command } from "multiplexer";

const cwd = process.cwd();
const env = { ...process.env, FORCE_COLOR: "true" };

const commands: Command[] = [
  {
    name: "vite",
    cmd: join(cwd, "packages/app", "node_modules/.bin", "vite"),
    args: ["dev"],
    cwd: join(cwd, "packages/app"),
    env,
    mode: "watch"
  },
  {
    name: "vitest",
    cmd: join(cwd, "packages/lib", "node_modules/.bin", "vitest"),
    args: ["--watch"],
    cwd: join(cwd, "packages/lib"),
    env,
    mode: "watch"
  },
  {
    name: "oxlint",
    cmd: "pnpm",
    args: ["run", "-F", "@my-vite-plus-monorepo/lib", "lint"],
    cwd,
    env,
    mode: "watch"
  },
  ...Array.from(
    { length: 6 },
    (_, i): Command => ({
      name: `stream ${i + 1}`,
      cmd: "bash",
      args: [join(cwd, "demo-stream.sh")],
      cwd,
      env,
      mode: "stream"
    })
  )
];

multiplex(commands.sort(() => Math.random() - 0.5));

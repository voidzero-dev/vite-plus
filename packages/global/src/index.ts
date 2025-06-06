import { execFileSync } from "node:child_process";
import { parseArgs } from "node:util";
import copyTemplateFiles from "./command/new.ts";

try {
  const { positionals } = parseArgs({ allowPositionals: true });

  const [command, dir] = positionals;

  if (command === "new") {
    const targetDir = dir ?? process.cwd();
    await copyTemplateFiles(targetDir);
    execFileSync("pnpm", ["install"], { stdio: "inherit" });
  } else {
    const { default: main } = await import("vite-plus");
    main();
  }
} catch (e) {
  if (e && e.status) process.exit(e.status);
  throw e;
}

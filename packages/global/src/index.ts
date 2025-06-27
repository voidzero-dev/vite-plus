import { parseArgs } from "node:util";
import { join } from "node:path";
import { questionnaire } from "./command/tasks.ts";

try {
  const { positionals } = parseArgs({ allowPositionals: true });

  const [command] = positionals;

  if (command === "new") {
    await questionnaire();
  } else {
    await import(join(process.cwd(), "node_modules/vite-plus/dist/cli.js"));
  }
} catch (e) {
  if (e && e.status) process.exit(e.status);
  throw e;
}

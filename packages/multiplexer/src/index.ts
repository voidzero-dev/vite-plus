import * as readline from "node:readline";
import type { Command } from "./types.ts";
import { LayoutEngine } from "./layout.ts";

// import { createWriteStream } from "node:fs";
// const debugStream = createWriteStream(`debug.log`, { flags: "a" });
// export const debug = (value: unknown) => {
//   if (typeof value === "string") debugStream.write(value + "\n");
//   else debugStream.write(JSON.stringify(value) + "\n");
// };

export function multiplex(commands: Command[]): void {
  process.stdin.setRawMode(true);
  process.stdin.resume();
  process.stdin.setEncoding("utf8");
  process.stdout.write("\u001B[?25l");

  const engine = new LayoutEngine(commands);
  engine.spawnAll();
  engine.listenAll();
  engine.render();

  process.stdin.on("data", data => {
    const chunk = data.toString();

    if (chunk === "\t" || chunk === "\x1B[B") {
      engine.movePanel(1);
    } else if (chunk === "\x1B[A") {
      engine.movePanel(-1);
    } else if (chunk === "\r" || chunk === "\n") {
      engine.killOrStartPanel(engine.selectedPanelIndex);
    } else if (chunk === "g") {
      engine.toggleGrid();
    } else if (chunk === "c") {
      engine.toggleControlPanelPosition();
    } else if (chunk === "q" || chunk === "\u0003") {
      engine.killAll();
      readline.cursorTo(process.stdout, 0, process.stdout.rows);
      process.stdout.write("\x1b[?25h");
      process.exit();
    }
  });
}

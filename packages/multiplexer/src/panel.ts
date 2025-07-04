import * as readline from "node:readline";
import { spawn, type ChildProcess } from "node:child_process";
import type { Command, Dimensions } from "./types.ts";
import stringWidth from "fast-string-truncated-width";
import { ANSI } from "./util.ts";

export class Panel {
  command?: Command;
  setCommand(command: Command) {
    this.command = command;
  }

  process?: ChildProcess;
  spawn() {
    const command = this.command;
    if (command) {
      this.process = spawn(command.cmd, command.args, {
        stdio: ["pipe", "pipe", "pipe"],
        cwd: command.cwd,
        env: command.env
      }).on("error", error => {
        delete command.env;
        const msg = error?.stack ?? error?.toString();
        this.buffer = [JSON.stringify(command, null, 2), "\n\n", msg];
        this.batchRender();
      });
    }
  }

  isClearScreen = false;

  listen(callback?: (code: number | null, signal: NodeJS.Signals | null) => void) {
    this.process?.stdout?.on("data", data => {
      const str = data.toString();
      const isClearScreen = str.startsWith("\u001bc");
      if (isClearScreen) this.isClearScreen = true;
      const chunk = str.replace(/\x1b(?:c|\[3J)/g, "");
      this.buffer.push(chunk);
      this.batchRender();
    });

    this.process?.stderr?.on("data", data => {
      const chunk = data.toString().replace(/\x1b(?:c|\[3J)/g, "");
      this.buffer.push(chunk);
      this.batchRender();
    });

    if (callback) this.process?.on("exit", callback);
    if (callback) this.process?.on("close", callback);
  }

  kill() {
    this.lines = [];
    this.buffer = [];
    this.timeout = undefined;
    this.process?.kill();
  }

  dimensions?: Dimensions;
  setDimensions(dimensions: Dimensions) {
    this.dimensions = dimensions;
  }

  lines: string[] = [];
  buffer: string[] = [];
  timeout?: NodeJS.Timeout;
  clearLines = 0;

  constructor(options: { command: Command; dimensions: Dimensions }) {
    this.command = options.command;
    this.dimensions = options.dimensions;
  }

  batchRender() {
    if (this.timeout) clearTimeout(this.timeout);

    this.timeout = setTimeout(() => {
      const trimmed = this.buffer.join("").trim().split("\n");
      if (this.command?.mode === "watch" || this.isClearScreen) this.lines = trimmed;
      else this.lines.push(...trimmed);
      this.buffer = [];
      this.render();
      this.lines = this.lines.slice(-process.stdout.rows);
    }, 50);
  }

  clear() {
    const dimensions = this.dimensions;
    if (dimensions) {
      const line = " ".repeat(dimensions.width);
      for (let i = 0; i < dimensions.height; i++) {
        readline.cursorTo(process.stdout, dimensions.left, dimensions.top + i);
        process.stdout.write(line);
      }
    }
  }

  render() {
    this.clear();
    const dimensions = this.dimensions;
    if (dimensions) {
      const lines = this.lines.slice(-dimensions.height);
      lines.forEach((line, lineIndex) => {
        readline.cursorTo(process.stdout, dimensions.left, dimensions.top + lineIndex);
        const width = stringWidth(line, { limit: dimensions.width });
        process.stdout.write(line.slice(0, width.index).trimEnd() + ANSI.reset);
        // const stripped = stripVTControlCharacters(line);
        // process.stdout.write(stripped.slice(0, dimensions.width).trimEnd());
      });
    }
  }
}

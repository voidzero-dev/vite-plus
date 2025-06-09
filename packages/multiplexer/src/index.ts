import { spawn } from "node:child_process";
import * as readline from "node:readline";
import { ChildProcess } from "node:child_process";
import stringWidth from "fast-string-truncated-width";
import type { Command } from "./types.ts";

type Panel = {
  name: string;
  command: Command;
  process: ChildProcess;
  lines: string[];
  buffer: string[];
  renderTimeout?: NodeJS.Timeout;
};

type Mode = "vertical" | "horizontal" | "quad";

type PanelDimensions = {
  width: number;
  height: number;
  top: number;
  left: number;
};

function calculatePanelDimensions(
  size: number,
  mode: Mode,
  gutterSize: number,
  statusBarHeight: number
): PanelDimensions[] {
  const width = process.stdout.columns;
  const height = process.stdout.rows;
  const columns = mode === "vertical" ? size : mode === "quad" ? Math.round(size / 2) : 1;
  const rows = mode === "vertical" ? 1 : mode === "quad" ? Math.round(size / 2) : size;
  return Array.from({ length: size }, (_, index) => {
    switch (mode) {
      case "vertical":
        return {
          width: Math.floor(width / columns) - gutterSize * columns,
          height: height - statusBarHeight,
          top: 0,
          left: index * Math.floor(width / columns)
        };
      case "horizontal":
        return {
          width: width,
          height: Math.floor(height / rows) - gutterSize * rows - statusBarHeight,
          top: index * Math.floor(height / rows),
          left: 0
        };
      case "quad":
        return {
          width: Math.floor(width / columns) - gutterSize * columns,
          height: Math.floor(height / rows) - gutterSize * rows - statusBarHeight,
          top: Math.floor(index / 2) * Math.floor(height / rows) + Math.floor(index / 2) * gutterSize,
          left: (index % 2) * Math.floor(width / columns)
        };
    }
  });
}

function renderScreen(
  panels: Panel[],
  isFullScreen: boolean,
  selectedIndex: number,
  mode: Mode,
  gutterSize: number,
  statusBarHeight: number
) {
  console.clear();

  const visiblePanels = isFullScreen ? [panels[selectedIndex]] : panels;

  const dimensions = calculatePanelDimensions(visiblePanels.length, mode, gutterSize, statusBarHeight);
  const horizontals = new Set(dimensions.map(d => d.top));
  const verticals = new Set(dimensions.map(d => d.left));

  for (let i = 0; i < visiblePanels.length; i++) {
    const panel = visiblePanels[i];
    if (panel) {
      const visibleLines = panel.lines.slice(-dimensions[i].height);
      visibleLines.forEach((line, lineIndex) => {
        readline.cursorTo(process.stdout, dimensions[i].left, dimensions[i].top + lineIndex);
        const width = stringWidth(line, { limit: dimensions[i].width });
        process.stdout.write(line.slice(0, width.index) + "\x1b[0m");
      });
    }

    if (dimensions[i].top !== 0) {
      readline.cursorTo(process.stdout, dimensions[i].left, dimensions[i].top - gutterSize);
      process.stdout.write("─".repeat(dimensions[i].width + verticals.size - 1));
    }
  }

  for (const left of verticals) {
    if (left === 0) continue;
    for (let j = 0; j < process.stdout.rows - 1; j++) {
      readline.cursorTo(process.stdout, left - 1, j);
      if (horizontals.has(j + gutterSize)) process.stdout.write("┼");
      else process.stdout.write("│");
    }
  }

  const y = process.stdout.rows - 1;
  readline.cursorTo(process.stdout, 0, y);
  const maxLength = Math.floor(process.stdout.columns / panels.length) - 2;
  panels.forEach((panel, index) => {
    const isSelected = index === selectedIndex;
    const cmdText = panel.name;
    const t = cmdText.length > maxLength ? cmdText.slice(0, maxLength - 3) + "..." : cmdText.padEnd(maxLength);
    const text = `[${t}]`;
    process.stdout.write(isSelected ? `\x1b[7m${text}\x1b[0m` : text);
  });
}

export function multiplex(commands: Command[]): void {
  process.stdin.setRawMode(true);
  process.stdin.resume();
  process.stdin.setEncoding("utf8");

  const gutterSize = 1;
  const statusBarHeight = 1;

  let viewMode: Mode =
    commands.length === 4 ? "quad" : process.stdout.columns / 2 > process.stdout.rows ? "vertical" : "horizontal";
  let isFullScreen = false;
  let selectedIndex = 0;

  const panels: Panel[] = commands.map(command => ({
    command,
    process: spawn(command.cmd, command.args, {
      stdio: ["pipe", "pipe", "pipe"],
      cwd: command.cwd,
      env: command.env
    }),
    name: command.name,
    lines: [],
    buffer: []
  }));

  function render() {
    renderScreen(panels, isFullScreen, selectedIndex, viewMode, gutterSize, statusBarHeight);
  }

  function batchRender(index: number) {
    const panel = panels[index];
    if (panel.renderTimeout) clearTimeout(panel.renderTimeout);

    panel.renderTimeout = setTimeout(() => {
      const lines = panel.buffer.join("").trim().split("\n");
      if (lines.length > 0) {
        panel.lines = lines;
        panel.buffer = [];
      }
      render();
    }, 16);
  }

  panels.forEach((panel, index) => {
    panel.process.stdout?.on("data", data => {
      const chunk = data.toString().replace(/\x1b(?:c|\[3J)/g, "");
      panel.buffer.push(chunk);
      batchRender(index);
    });

    panel.process.stderr?.on("data", data => {
      const chunk = data.toString();
      panel.buffer.push(chunk);
      batchRender(index);
    });
  });

  render();

  process.stdin.on("data", data => {
    const chunk = data.toString();
    if (chunk === "q" || chunk === "\u0003") {
      panels.forEach(panel => panel.process.kill());
      console.clear();
      readline.cursorTo(process.stdout, 0, 0);
      process.stdout.write("\x1b[?25h");
      readline.cursorTo(process.stdout, 0, process.stdout.rows - 1);
      process.exit();
    }

    if (chunk === "\t") {
      selectedIndex = (selectedIndex + 1) % commands.length;
      render();
      return;
    }

    if (chunk === "\r") {
      isFullScreen = !isFullScreen;
      render();
      return;
    }

    for (const panel of panels) panel.process.stdin?.write(data + "\n");
  });
}

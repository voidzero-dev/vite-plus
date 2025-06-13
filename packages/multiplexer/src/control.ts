import * as readline from "node:readline";
import type { Dimensions, Position } from "./types.ts";
import { DIVIDER_WIDTH } from "./layout.ts";
import stringWidth from "fast-string-truncated-width";

export class ControlPanel {
  position: Position;
  names: string[];

  constructor(options: { position: Position; names: string[] }) {
    this.position = options.position;
    this.names = options.names.map(name => {
      const options = { limit: 3, ellipsis: "…" };
      const width = stringWidth(name, { limit: 16 });
      return `${name.slice(0, width.index)}${width.ellipsed ? options.ellipsis : ""}`;
    });
  }

  setPosition(position: Position) {
    this.position = position;
  }

  getDimensions(screen: Dimensions): Dimensions {
    const width = Math.max(...this.names.map(name => name.length)) + DIVIDER_WIDTH;
    const height = 1;
    switch (this.position) {
      case "top":
        return { top: 0, left: 0, width: screen.width, height };
      case "bottom":
        return { top: screen.height - height, left: 0, width: screen.width, height };
      case "left":
        return { width, height: screen.height, top: 0, left: 0 };
      case "right":
        return { width, height: screen.height, top: 0, left: screen.width - width };
    }
  }

  render(screen: Dimensions, selectedIndex: number) {
    const dimensions = this.getDimensions(screen);

    const names = this.names.map((name, index) => (index === selectedIndex ? `\x1b[7m${name}\x1b[0m` : name));
    const lines = this.position === "top" || this.position === "bottom" ? [names.join(" • ")] : names;

    lines.forEach((line, lineIndex) => {
      readline.cursorTo(process.stdout, dimensions.left, dimensions.top + lineIndex);
      process.stdout.write(line);
    });
  }
}

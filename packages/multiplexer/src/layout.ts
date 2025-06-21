import * as readline from "node:readline";
import type { Command, Dimensions, Position, PanelState } from "./types.ts";
import { Panel } from "./panel.ts";
import { ControlPanel } from "./control.ts";
import { getScreenDimensions } from "./util.ts";

const POSITIONS: Position[] = ["left", "top", "right", "bottom"];

export const DIVIDER_WIDTH = 1;

export function getGrid(size: number, width: number, height: number) {
  const physicalHeight = height * 2.5;
  const rows = Math.ceil((Math.sqrt(size) * physicalHeight) / width);
  return { rows, columns: Math.ceil(size / rows) };
}

export function getPanelDimensions(size: number, screen: Dimensions): Dimensions[] {
  const { rows, columns } = getGrid(size, screen.width, screen.height);
  const width = Math.floor((screen.width - DIVIDER_WIDTH * columns) / columns);
  const height = Math.floor((screen.height - DIVIDER_WIDTH * rows) / rows);
  const fillX = screen.width - columns * width - (columns - 1) * DIVIDER_WIDTH;
  const fillY = screen.height - rows * height - (rows - 1) * DIVIDER_WIDTH;

  const dimensions: Dimensions[] = [];

  for (let index = 0; index < size; index++) {
    const row = Math.floor(index / columns);
    const col = index % columns;
    const above = dimensions[index - columns];
    const previous = dimensions[index - 1];
    dimensions.push({
      width: width + (col < fillX ? 1 : 0),
      height: height + (row < fillY ? 1 : 0),
      left: col === 0 ? screen.left : previous.left + previous.width + DIVIDER_WIDTH,
      top: row === 0 ? screen.top : above.top + above.height + DIVIDER_WIDTH
    });
  }

  return dimensions;
}

export class LayoutEngine {
  commands: Command[] = [];
  isGrid = false;
  controlPanel: ControlPanel;
  panels: Panel[] = [];
  selectedPanelIndex = 0;

  constructor() {
    this.controlPanel = new ControlPanel({ position: POSITIONS[0], names: [] });

    process.stdout.on("resize", () => {
      this.render();
    });
  }

  addCommands(commands: Command[]) {
    this.commands.push(...commands);
    const dimensions = this.getPanelDimensions();
    for (const command of commands) {
      this.controlPanel.addItem(command.name);
      this.panels.push(new Panel({ command, dimensions: dimensions[this.panels.length] }));
    }
  }

  setSelectedIndex(index: number) {
    this.selectedPanelIndex = index;
    this.controlPanel.setSelectedIndex(index);
  }

  getAvailableScreen() {
    const screen = getScreenDimensions();
    const dim = this.controlPanel.getDimensions();
    const pos = this.controlPanel.position;
    return {
      top: pos === "bottom" || pos === "right" || pos === "left" ? 0 : dim.height + DIVIDER_WIDTH,
      left: pos === "top" || pos === "right" || pos === "bottom" ? 0 : dim.width + DIVIDER_WIDTH,
      width: pos === "top" || pos === "bottom" ? screen.width : Math.max(0, screen.width - dim.width - DIVIDER_WIDTH),
      height:
        pos === "left" || pos === "right" ? screen.height : Math.max(0, screen.height - dim.height - DIVIDER_WIDTH)
    };
  }

  getPanelDimensions(): Dimensions[] {
    const screen = this.getAvailableScreen();
    const size = this.commands.length;

    if (!this.isGrid) {
      const dimensions = new Array(size);
      const items = this.controlPanel.items.map(item => item.state === "running");
      const runningSize = items.filter(Boolean).length;
      if (runningSize === 0) {
        dimensions[this.selectedPanelIndex] = screen;
      } else {
        const runningPanelDimensions = getPanelDimensions(runningSize, screen);
        for (let index = 0; index < items.length; index++) {
          if (items[index]) dimensions[index] = runningPanelDimensions.shift();
        }
      }
      return dimensions;
    }

    return getPanelDimensions(size, screen);
  }

  movePanel(direction: number) {
    const count = this.controlPanel.items.length;
    this.setSelectedIndex((this.selectedPanelIndex + direction + count) % count);
    if (this.isGrid) this.controlPanel.render();
    else this.render();
  }

  handlePanelFinished(index: number, state: PanelState) {
    this.controlPanel.setItemState(index, state);
    const nextPanelIndex = this.controlPanel.items.findIndex((item, i) => item.state === "running");
    if (nextPanelIndex !== -1) this.setSelectedIndex(nextPanelIndex);
    this.render();
  }

  toggleGrid() {
    this.isGrid = !this.isGrid;
    this.render();
  }

  toggleControlPanelPosition() {
    this.controlPanel.setPosition(POSITIONS[(POSITIONS.indexOf(this.controlPanel.position) + 1) % POSITIONS.length]);
    this.render();
  }

  run(command: Command, callback: (code: number | null, signal: NodeJS.Signals | null) => void) {
    const panelIndex = this.panels.findIndex(panel => panel.command === command);
    this.panels[panelIndex]?.spawn();
    this.panels[panelIndex]?.listen((code, signal) => {
      this.handlePanelFinished(panelIndex, code === 0 ? "done" : "error");
      callback(code, signal);
    });
    this.controlPanel.setItemState(panelIndex, "running");
  }

  spawnAll() {
    this.panels.forEach(panel => panel.spawn());
  }

  listenAll() {
    this.panels.forEach(panel => panel.listen());
  }

  killAll() {
    this.panels.forEach(panel => panel.kill());
  }

  killOrStartPanel(index: number) {
    const panel = this.panels[index];
    if (!panel) return;
    if (panel.process && !panel.process.killed) {
      this.controlPanel.setItemState(index, "error");
      panel.clear();
      panel.kill();
    } else {
      this.controlPanel.setItemState(index, "running");
      panel.spawn();
      panel.listen();
      panel.render();
    }
  }

  renderDividers() {
    const screen = getScreenDimensions();
    const isLeft = this.controlPanel.position === "left";
    const isRight = this.controlPanel.position === "right";
    const isBottom = this.controlPanel.position === "bottom";
    const dim = this.controlPanel.getDimensions();

    const dimensions = this.getPanelDimensions();
    const horizontals: Set<number> = new Set(dimensions.map(d => d.top).filter(Boolean));
    const verticals: Set<number> = new Set(dimensions.map(d => d.left).filter(Boolean));

    const top = this.controlPanel.position === "top" ? dim.height : 0;
    const bottom = this.controlPanel.position === "bottom" ? process.stdout.rows - dim.height : process.stdout.rows;
    if (isRight) verticals.add(dim.left);
    for (const left of verticals) {
      for (let j = top; j < bottom; j++) {
        readline.cursorTo(process.stdout, left - 1, j);
        process.stdout.write("│");
      }
    }

    if (isBottom) horizontals.add(dim.top);
    for (const top of horizontals) {
      let divider = "─".repeat(isLeft || isRight ? screen.width - dim.width : screen.width);
      for (const left of verticals) {
        const pos = isLeft ? left - dim.width : left;
        const isStart = pos === 1 || top - dim.height === 1;
        const isEnd = pos === divider.length || top === dim.top;
        const char = isLeft || isRight ? (isStart ? "├" : isEnd ? "┤" : "┼") : isStart ? "┬" : isEnd ? "┴" : "┼";
        divider = divider.substring(0, pos - 1) + char + divider.substring(pos);
      }
      readline.cursorTo(process.stdout, isLeft ? dim.width : 0, top - 1);
      process.stdout.write(divider);
    }
  }

  render() {
    console.clear();
    this.controlPanel.render();
    const dimensions = this.getPanelDimensions();
    this.panels.forEach((panel, index) => panel.setDimensions(dimensions[index]));
    for (const panel of this.panels) panel.render();
    this.renderDividers();
  }
}

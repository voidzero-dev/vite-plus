import stringWidth from 'fast-string-truncated-width';
import * as readline from 'node:readline';
import { DIVIDER_WIDTH } from './layout';
import type { Dimensions, PanelState, Position } from './types';
import { ANSI, getScreenDimensions } from './util';

const stateStyles = {
  running: { color: ANSI.yellow, symbol: '→' },
  done: { color: ANSI.green, symbol: '✔︎' },
  error: { color: ANSI.red, symbol: '✖︎' },
  idle: { color: '', symbol: ' ' },
};

export class ControlPanel {
  position: Position;
  items: { name: string; state: PanelState }[] = [];
  selectedIndex = 0;

  constructor(options: { position: Position; names: string[] }) {
    this.position = options.position;
    for (const name of options.names) this.addItem(name);
  }

  addItem(name: string) {
    const options = { limit: 3, ellipsis: '…' };
    const width = stringWidth(name, { limit: 16 });
    this.items.push({
      state: 'idle',
      name: `${name.slice(0, width.index)}${width.ellipsed ? options.ellipsis : ''}`,
    });
  }

  setSelectedIndex(index: number) {
    this.selectedIndex = index;
  }

  setItemState(index: number, state: PanelState) {
    if (this.items[index]) this.items[index].state = state;
  }

  setPosition(position: Position) {
    this.position = position;
  }

  getDimensions(): Dimensions {
    const screen = getScreenDimensions();
    const width =
      Math.max(...this.items.map((item) => item.name.length)) +
      DIVIDER_WIDTH +
      2;
    const height = 1;
    switch (this.position) {
      case 'top':
        return { top: 0, left: 0, width: screen.width, height };
      case 'bottom':
        return {
          top: screen.height - height,
          left: 0,
          width: screen.width,
          height,
        };
      case 'left':
        return { width, height: screen.height, top: 0, left: 0 };
      case 'right':
        return {
          width,
          height: screen.height,
          top: 0,
          left: screen.width - width,
        };
    }
  }

  render() {
    const dimensions = this.getDimensions();

    const names = this.items.map((item, index) => {
      const isSelected = index === this.selectedIndex;
      const style = stateStyles[item.state] || stateStyles.idle;
      const text = `${style.color}${style.symbol} ${item.name}${ANSI.reset}`;
      return isSelected ? `${ANSI.reverse}${text}` : text;
    });

    const lines =
      this.position === 'top' || this.position === 'bottom'
        ? [names.join(' • ')]
        : names;

    lines.forEach((line, lineIndex) => {
      readline.cursorTo(
        process.stdout,
        dimensions.left,
        dimensions.top + lineIndex,
      );
      process.stdout.write(line);
    });
  }
}

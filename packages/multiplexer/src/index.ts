import * as readline from 'node:readline'
import type { Command } from './types.js'
import { LayoutEngine } from './layout.js'

// import { createWriteStream } from "node:fs";
// const debugStream = createWriteStream(`debug.log`, { flags: "a" });
// export const debug = (value: unknown) => {
//   if (typeof value === "string") debugStream.write(value + "\n");
//   else debugStream.write(JSON.stringify(value) + "\n");
// };

export async function multiplex(commands: Command[][]) {
  process.stdin.setRawMode(true)
  process.stdin.resume()
  process.stdin.setEncoding('utf8')
  process.stdout.write('\u001B[?25l')

  const engine = new LayoutEngine()

  process.stdin.on('data', (data) => {
    const chunk = data.toString()

    if (chunk === '\t' || chunk === '\x1B[B') {
      engine.selectNextPanel(1)
    } else if (chunk === '\x1B[A') {
      engine.selectNextPanel(-1)
    } else if (chunk === '\r' || chunk === '\n') {
      engine.killOrStartPanel(engine.selectedPanelIndex)
    } else if (chunk === 'g') {
      engine.toggleGrid()
    } else if (chunk === 'c') {
      engine.toggleControlPanelPosition()
    } else if (chunk === 'q' || chunk === '\u0003') {
      quit()
    }
  })

  for (const cmds of commands) engine.addCommands(cmds)

  for (const cmds of commands) {
    const start = (command: Command) =>
      new Promise((resolve) => engine.run(command, resolve))
    const promises = cmds.map(start)
    engine.render()
    await Promise.all(promises)
  }

  function quit() {
    engine.killAll()
    readline.cursorTo(process.stdout, 0, process.stdout.rows - 1)
    process.stdout.write('\x1b[?25h')
    process.exit(0)
  }

  // quit();
}

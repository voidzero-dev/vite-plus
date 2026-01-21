import process from 'node:process';
import { styleText } from 'node:util';

type RGB = readonly [number, number, number];
const ESC = '\x1b';
const CSI = '\x1b[';

const RESET = `${CSI}0m`;
const fgRgb = (r: number, g: number, b: number) => `${CSI}38;2;${r};${g};${b}m`;

const shouldColorize = (stream = process.stdout) =>
  stream?.isTTY && (typeof stream.hasColors === 'function' ? stream.hasColors() : true);

function supportsTrueColor(stream = process.stdout) {
  if (!stream?.isTTY) {
    return false;
  }

  const depth = typeof stream.getColorDepth === 'function' ? stream.getColorDepth() : 1;
  return depth >= 24;
}

function fadeToColor(count: number, endRgb: RGB) {
  const minT = 0.7;
  const gamma = 1.6;

  const n = Math.max(count, 1);
  const denom = Math.max(n - 1, 1);

  const [er, eg, eb] = endRgb;
  const colors: Array<RGB> = [];

  for (let i = 0; i < n; i++) {
    const u = i / denom;
    const eased = Math.pow(u, gamma);
    const t = minT + (1 - minT) * eased;

    colors.push([Math.round(er * t), Math.round(eg * t), Math.round(eb * t)]);
  }

  return colors;
}

function gradient(count: number, startRgb: RGB, endRgb: RGB) {
  const n = Math.max(count, 1);
  const denom = Math.max(n - 1, 1);

  const [sr, sg, sb] = startRgb;
  const [er, eg, eb] = endRgb;

  const lerp = (a: number, b: number, t: number) => a + (b - a) * t;

  const colors: Array<RGB> = [];
  for (let i = 0; i < n; i++) {
    const t = i / denom;
    colors.push([
      Math.round(lerp(sr, er, t)),
      Math.round(lerp(sg, eg, t)),
      Math.round(lerp(sb, eb, t)),
    ]);
  }
  return colors;
}

function colorize(text: string, colors: Array<RGB>) {
  const chars = [...text];
  if (chars.length === 0) {
    return '';
  }

  const denom = Math.max(chars.length - 1, 1);
  const maxIdx = colors.length - 1;

  let out = '';
  for (let i = 0; i < chars.length; i++) {
    const idx = Math.round((i / denom) * maxIdx);
    const [r, g, b] = colors[idx];
    out += fgRgb(r, g, b) + chars[i];
  }
  return out + RESET;
}

async function getForegroundColor(): Promise<null | RGB> {
  const stdin = process.stdin;
  const stdout = process.stdout;

  if (process.env.CI || !stdin?.isTTY || !stdout?.isTTY) {
    return null;
  }

  const query = `${ESC}]10;?${ESC}\\`;
  const responseRe =
    // oxlint-disable-next-line no-control-regex
    /\x1b\]10;[\s\S]*?rgb:([0-9a-fA-F]+)\/([0-9a-fA-F]+)\/([0-9a-fA-F]+)(?:\x07|\x1b\\)/;

  return await new Promise((resolve) => {
    let done = false;
    let buffer = '';
    let flushTimer: NodeJS.Timeout | null = null;

    const finish = (rgb: RGB | null) => {
      if (done) {
        return;
      }
      done = true;
      clearTimeout(timer);
      if (flushTimer) {
        clearTimeout(flushTimer);
      }
      stdin.off('data', onData);

      try {
        stdin.setRawMode(false);
      } catch {}
      try {
        stdin.pause();
      } catch {}

      resolve(rgb);
    };

    const timer = setTimeout(() => finish(null), 100);
    const scheduleFlush = () => {
      if (flushTimer) {
        clearTimeout(flushTimer);
      }
      flushTimer = setTimeout(() => {
        buffer = '';
      }, 50);
    };

    const to8bit = (hex: string) => {
      if (hex.length === 2) {
        return parseInt(hex, 16);
      }
      if (hex.length === 4) {
        return Math.round((parseInt(hex, 16) / 0xffff) * 255);
      }
      const max = Math.pow(16, hex.length) - 1;
      return Math.round((parseInt(hex, 16) / max) * 255);
    };

    const onData = (data: string) => {
      buffer += data;
      scheduleFlush();

      if (buffer.length > 1024) {
        buffer = buffer.slice(-1024);
      }

      const m = responseRe.exec(buffer);

      if (!m) {
        return;
      }

      const r = to8bit(m[1]);
      const g = to8bit(m[2]);
      const b = to8bit(m[3]);

      if ([r, g, b].some((x) => !Number.isFinite(x))) {
        return;
      }

      finish([r, g, b]);
    };

    try {
      stdin.setEncoding('utf8');
      stdin.resume();
      stdin.setRawMode(true);
      stdin.on('data', onData);

      // Send query
      stdout.write(query);
    } catch {
      finish(null);
    }
  });
}

const purple = [101, 63, 246] as const;
let gradientColors: Array<RGB> | null = null;

async function getGradientColors(text: string) {
  if (!gradientColors) {
    const fg = await getForegroundColor();
    gradientColors = fg ? gradient(text.length, fg, purple) : fadeToColor(text.length, purple);
  }
  return gradientColors;
}

export async function getVitePlusHeader() {
  const textA = 'The Unified ';
  const textB = 'Toolchain for the Web';

  if (!shouldColorize(process.stdout) || !supportsTrueColor(process.stdout)) {
    return `VITE+(⚡︎) - ${textA}${textB}`;
  }

  return `${styleText(
    'bold',
    `VITE+(${accent('⚡︎')}) - ${textA}${colorize(textB, await getGradientColors(textB))}`,
  )}`;
}

export function log(message: string) {
  /* oxlint-disable-next-line no-console */
  console.log(message);
}

export function accent(text: string) {
  return styleText('blueBright', text);
}

export function headline(text: string) {
  return styleText('bold', text.toUpperCase());
}

export function muted(text: string) {
  return styleText('gray', text);
}

export function success(text: string) {
  return styleText('green', text);
}

export function error(text: string) {
  return styleText('red', text);
}

import type { Writable } from 'node:stream';
import { stripVTControlCharacters } from 'node:util';

import { getColumns } from '@clack/core';
import stringWidth from 'fast-string-width';
import { wrapAnsi } from 'fast-wrap-ansi';
import color from 'picocolors';

import {
  type CommonOptions,
  getGuide,
  S_BAR,
  S_BAR_END,
  S_BAR_END_RIGHT,
  S_BAR_H,
  S_BAR_START,
  S_BAR_START_RIGHT,
  S_CORNER_BOTTOM_LEFT,
  S_CORNER_BOTTOM_RIGHT,
  S_CORNER_TOP_LEFT,
  S_CORNER_TOP_RIGHT,
} from './common.js';

export type BoxAlignment = 'left' | 'center' | 'right';

type BoxSymbols = [topLeft: string, topRight: string, bottomLeft: string, bottomRight: string];

const roundedSymbols: BoxSymbols = [
  S_CORNER_TOP_LEFT,
  S_CORNER_TOP_RIGHT,
  S_CORNER_BOTTOM_LEFT,
  S_CORNER_BOTTOM_RIGHT,
];
const squareSymbols: BoxSymbols = [S_BAR_START, S_BAR_START_RIGHT, S_BAR_END, S_BAR_END_RIGHT];

export interface BoxOptions extends CommonOptions {
  contentAlign?: BoxAlignment;
  titleAlign?: BoxAlignment;
  width?: number | 'auto';
  titlePadding?: number;
  contentPadding?: number;
  rounded?: boolean;
  formatBorder?: (text: string) => string;
}

function getPaddingForLine(
  lineLength: number,
  innerWidth: number,
  padding: number,
  contentAlign: BoxAlignment | undefined,
): [number, number] {
  let leftPadding = padding;
  let rightPadding = padding;
  if (contentAlign === 'center') {
    leftPadding = Math.floor((innerWidth - lineLength) / 2);
  } else if (contentAlign === 'right') {
    leftPadding = innerWidth - lineLength - padding;
  }

  rightPadding = innerWidth - leftPadding - lineLength;

  return [leftPadding, rightPadding];
}

const defaultFormatBorder = color.gray;

export const box = (message = '', title = '', opts?: BoxOptions) => {
  const output: Writable = opts?.output ?? process.stdout;
  const columns = getColumns(output);
  const borderWidth = 1;
  const borderTotalWidth = borderWidth * 2;
  let titlePadding = Math.max(0, opts?.titlePadding ?? 1);
  let contentPadding = Math.max(0, opts?.contentPadding ?? 2);
  const width =
    opts?.width === undefined || opts.width === 'auto' ? 1 : Math.max(0, Math.min(1, opts.width));
  const hasGuide = getGuide(opts);
  const linePrefix = !hasGuide ? '' : `${color.gray(S_BAR)} `;
  const formatBorder = opts?.formatBorder ?? defaultFormatBorder;
  const symbols = (opts?.rounded !== false ? roundedSymbols : squareSymbols).map(formatBorder);
  const hSymbol = formatBorder(S_BAR_H);
  const vSymbol = formatBorder(S_BAR);
  const linePrefixWidth = stringWidth(linePrefix);
  const titleWidth = stringWidth(title);
  const maxBoxWidth = columns - linePrefixWidth;
  let boxWidth = Math.max(Math.min(8, maxBoxWidth), Math.floor(columns * width) - linePrefixWidth);
  if (opts?.width === 'auto') {
    const lines = message.split('\n');
    let longestLine = titleWidth + titlePadding * 2;
    for (const line of lines) {
      const lineWithPadding = stringWidth(line) + contentPadding * 2;
      if (lineWithPadding > longestLine) {
        longestLine = lineWithPadding;
      }
    }
    const longestLineWidth = longestLine + borderTotalWidth;
    if (longestLineWidth < boxWidth) {
      boxWidth = longestLineWidth;
    }
  }
  if (boxWidth % 2 !== 0) {
    if (boxWidth < maxBoxWidth) {
      boxWidth++;
    } else {
      boxWidth--;
    }
  }
  const innerWidth = Math.max(0, boxWidth - borderTotalWidth);
  titlePadding = Math.min(titlePadding, Math.floor(innerWidth / 2));
  contentPadding = Math.min(contentPadding, Math.max(0, Math.floor((innerWidth - 1) / 2)));
  const maxTitleLength = innerWidth - titlePadding * 2;
  let truncatedTitle = title;
  if (titleWidth > maxTitleLength) {
    truncatedTitle = '';
    const suffix = maxTitleLength > 0 ? '…' : '';
    for (const { segment } of new Intl.Segmenter().segment(stripVTControlCharacters(title))) {
      if (stringWidth(truncatedTitle + segment + suffix) > maxTitleLength) {
        break;
      }
      truncatedTitle += segment;
    }
    truncatedTitle += suffix;
  }
  const [titlePaddingLeft, titlePaddingRight] = getPaddingForLine(
    stringWidth(truncatedTitle),
    innerWidth,
    titlePadding,
    opts?.titleAlign,
  );
  const wrappedMessage = wrapAnsi(message, Math.max(1, innerWidth - contentPadding * 2), {
    hard: true,
    trim: false,
  });
  output.write(
    `${linePrefix}${symbols[0]}${hSymbol.repeat(titlePaddingLeft)}${truncatedTitle}${hSymbol.repeat(titlePaddingRight)}${symbols[1]}\n`,
  );
  const wrappedLines = wrappedMessage.split('\n');
  for (const line of wrappedLines) {
    const [leftLinePadding, rightLinePadding] = getPaddingForLine(
      stringWidth(line),
      innerWidth,
      contentPadding,
      opts?.contentAlign,
    );
    output.write(
      `${linePrefix}${vSymbol}${' '.repeat(leftLinePadding)}${line}${' '.repeat(rightLinePadding)}${vSymbol}\n`,
    );
  }
  output.write(`${linePrefix}${symbols[2]}${hSymbol.repeat(innerWidth)}${symbols[3]}\n`);
};

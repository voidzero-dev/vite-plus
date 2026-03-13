import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { styleText } from 'node:util';

import type {
  CheckAchievementTier,
  CheckAchievementUpdate,
  ClassifiedCheckFailureKind,
} from './check-achievements.js';
import { pkgRoot } from './path.js';

const ESC = '\u001b';
const ST = `${ESC}\\`;
const BEL = '\u0007';
const PNG_SIGNATURE = '89504e470d0a1a0a';
const DEFAULT_IMAGE_COLUMNS = 26;
const MIN_IMAGE_ROWS = 10;
const MAX_IMAGE_ROWS = 18;
const RESET = '\u001b[0m';
const CSI = '\u001b[';
const DEFAULT_BLUE = [88, 146, 255] as const;
const DEFAULT_MAGENTA = [187, 116, 247] as const;
const HEADER_SUFFIX_FADE_GAMMA = 1.35;

export type InlineImageProtocol = 'file' | 'kitty' | 'sixel';

export interface ImagePlacement {
  columns: number;
  rows: number;
}

export interface PngDimensions {
  width: number;
  height: number;
}

export type CheckFailureKind = 'error' | 'formatting' | 'lint' | 'type-aware';

export interface CheckFailureCelebration {
  achievementUpdate?: CheckAchievementUpdate;
  failureKind: CheckFailureKind;
}

export async function printCheckFailureImage(
  stream: NodeJS.WriteStream = process.stderr,
  env: NodeJS.ProcessEnv = process.env,
  celebration: CheckFailureCelebration = { failureKind: 'error' },
) {
  if (!stream.isTTY) {
    return;
  }

  const textBlock = renderCheckFailureTextBlock(celebration, stream, env);
  const protocol = detectInlineImageProtocol(env, {
    canRenderSixel: hasSixelEncoder(),
  });

  if (protocol) {
    const imagePath = resolveErrorImagePath();
    if (fs.existsSync(imagePath)) {
      const imageBuffer = await readFile(imagePath);
      const dimensions = readPngDimensions(imageBuffer);
      const placement = getImagePlacement(dimensions);

      if (protocol === 'kitty') {
        stream.write(`\n${buildKittyImageSequence(imagePath, placement)}\n\n${textBlock}\n`);
        return;
      }

      if (protocol === 'file') {
        stream.write(
          `\n${buildOsc1337FileSequence(imagePath, imageBuffer, placement)}\n\n${textBlock}\n`,
        );
        return;
      }

      const sixel = renderSixelImage(imagePath, placement);
      if (sixel) {
        stream.write('\n');
        stream.write(sixel);
        stream.write(`\n\n${textBlock}\n`);
        return;
      }
    }
  }

  stream.write(`\n${textBlock}\n`);
}

export function getCheckFailureCaption(
  failureKind: CheckFailureKind,
  achievementUpdate?: CheckAchievementUpdate,
) {
  if (achievementUpdate?.rareMomentTriggered) {
    const index = (achievementUpdate.state.totalFailures - 1) % RARE_FAILURE_MESSAGES.length;
    return RARE_FAILURE_MESSAGES[index];
  }

  const counts = achievementUpdate?.state.failureCounts;

  switch (failureKind) {
    case 'formatting': {
      const index = ((counts?.formatting ?? 1) - 1) % FORMAT_FAILURE_MESSAGES.length;
      return FORMAT_FAILURE_MESSAGES[index];
    }
    case 'lint': {
      const index = ((counts?.lint ?? 1) - 1) % LINT_FAILURE_MESSAGES.length;
      return LINT_FAILURE_MESSAGES[index];
    }
    case 'type-aware': {
      const index = ((counts?.['type-aware'] ?? 1) - 1) % TYPE_AWARE_FAILURE_MESSAGES.length;
      return TYPE_AWARE_FAILURE_MESSAGES[index];
    }
    default: {
      const index =
        ((achievementUpdate?.state.totalFailures ?? 1) - 1) % GENERIC_FAILURE_MESSAGES.length;
      return GENERIC_FAILURE_MESSAGES[index];
    }
  }
}

export function normalizeCheckFailureKind(value: string | null | undefined): CheckFailureKind {
  switch (value) {
    case 'formatting':
    case 'lint':
    case 'type-aware':
      return value;
    default:
      return 'error';
  }
}

export function renderCheckFailureTextBlock(
  celebration: CheckFailureCelebration,
  stream: Pick<NodeJS.WriteStream, 'getColorDepth' | 'isTTY'> = process.stderr,
  env: NodeJS.ProcessEnv = process.env,
) {
  const heading = renderStaticAnalysisEnthusiastHeading(stream, env);
  const caption = getCheckFailureCaption(celebration.failureKind, celebration.achievementUpdate);
  const lines = [`${heading} ${caption}`];

  if (
    celebration.achievementUpdate?.currentStreak &&
    celebration.achievementUpdate.currentStreak.count >= 3
  ) {
    lines.push(
      styleText(
        'gray',
        getFailureStreakMessage(
          celebration.achievementUpdate.currentStreak.kind,
          celebration.achievementUpdate.currentStreak.count,
        ),
      ),
    );
  }

  for (const achievement of celebration.achievementUpdate?.newlyUnlocked ?? []) {
    lines.push(
      `🎉 ${getAchievementTrophy(achievement.tier)} ${styleText(['yellow', 'bold'], 'achievement unlocked:')} ${achievement.title} ${styleText('gray', `(${achievement.description})`)}`,
    );
  }

  if (celebration.achievementUpdate) {
    lines.push(
      styleText(
        'gray',
        `collection progress: ${celebration.achievementUpdate.collectionProgress.unlocked}/${celebration.achievementUpdate.collectionProgress.total} achievements unlocked`,
      ),
    );

    for (const progress of celebration.achievementUpdate.nearUnlocks) {
      if (progress.taunt) {
        lines.push(styleText('yellow', progress.taunt));
      }
    }
  }

  return lines.join('\n');
}

export function renderStaticAnalysisEnthusiastHeading(
  stream: Pick<NodeJS.WriteStream, 'getColorDepth' | 'isTTY'> = process.stderr,
  env: NodeJS.ProcessEnv = process.env,
) {
  const heading = 'The static analysis enthusiast says:';
  if (!stream.isTTY || env.NO_COLOR || stream.getColorDepth?.(env) < 24) {
    return styleText('bold', heading);
  }

  return bold(
    colorize(
      heading,
      gradientEased(heading.length, DEFAULT_BLUE, DEFAULT_MAGENTA, HEADER_SUFFIX_FADE_GAMMA),
    ),
  );
}

export function detectInlineImageProtocol(
  env: NodeJS.ProcessEnv,
  options: { canRenderSixel?: boolean } = {},
): InlineImageProtocol | null {
  const termProgram = env.TERM_PROGRAM ?? '';
  if (termProgram === 'iTerm.app' || termProgram === 'WezTerm' || env.LC_TERMINAL === 'iTerm2') {
    return 'file';
  }

  const term = env.TERM ?? '';
  if (env.KITTY_WINDOW_ID || term.includes('kitty') || termProgram.toLowerCase() === 'ghostty') {
    return 'kitty';
  }

  const sixelAdvertised = (env.TERM_FEATURES ?? '').includes('Sx') || /sixel/i.test(term);
  if (sixelAdvertised && options.canRenderSixel) {
    return 'sixel';
  }

  return null;
}

export function resolveErrorImagePath() {
  const distAssetPath = path.join(pkgRoot, 'dist', 'assets', 'error.png');
  if (fs.existsSync(distAssetPath)) {
    return distAssetPath;
  }

  return path.join(pkgRoot, 'assets', 'error.png');
}

export function readPngDimensions(buffer: Buffer): PngDimensions {
  if (buffer.length < 24 || buffer.subarray(0, 8).toString('hex') !== PNG_SIGNATURE) {
    throw new Error('Expected a PNG image for the check failure asset');
  }

  return {
    width: buffer.readUInt32BE(16),
    height: buffer.readUInt32BE(20),
  };
}

export function getImagePlacement(dimensions: PngDimensions): ImagePlacement {
  const rows = Math.round((dimensions.height / dimensions.width) * DEFAULT_IMAGE_COLUMNS * 0.5);

  return {
    columns: DEFAULT_IMAGE_COLUMNS,
    rows: clamp(rows, MIN_IMAGE_ROWS, MAX_IMAGE_ROWS),
  };
}

export function buildKittyImageSequence(imagePath: string, placement: ImagePlacement) {
  return (
    `${ESC}_Ga=T,t=f,f=100,c=${placement.columns},r=${placement.rows};` +
    `${Buffer.from(imagePath).toString('base64')}${ST}`
  );
}

export function buildOsc1337FileSequence(
  imagePath: string,
  imageBuffer: Buffer,
  placement: ImagePlacement,
) {
  const fileName = Buffer.from(path.basename(imagePath)).toString('base64');
  const imageContents = imageBuffer.toString('base64');

  return (
    `${ESC}]1337;File=name=${fileName};size=${imageBuffer.length};width=${placement.columns};` +
    `height=${placement.rows};preserveAspectRatio=1;inline=1:${imageContents}${BEL}`
  );
}

function hasSixelEncoder() {
  return findSixelEncoder() !== null;
}

function findSixelEncoder() {
  for (const candidate of [
    ['img2sixel', ['-h']],
    ['magick', ['-version']],
  ] as const) {
    const result = spawnSync(candidate[0], candidate[1], {
      encoding: 'utf8',
      stdio: 'ignore',
    });
    if (result.status === 0) {
      return candidate[0];
    }
  }

  return null;
}

function renderSixelImage(imagePath: string, placement: ImagePlacement) {
  const targetWidth = placement.columns * 10;
  const targetHeight = placement.rows * 20;

  const encoder = findSixelEncoder();
  if (encoder === 'img2sixel') {
    const result = spawnSync('img2sixel', ['-w', String(targetWidth), imagePath], {
      encoding: 'buffer',
      maxBuffer: 16 * 1024 * 1024,
      stdio: ['ignore', 'pipe', 'ignore'],
    });
    if (result.status === 0 && result.stdout.length > 0) {
      return result.stdout;
    }
  }

  if (encoder === 'magick') {
    const result = spawnSync(
      'magick',
      [imagePath, '-resize', `${targetWidth}x${targetHeight}`, 'sixel:-'],
      {
        encoding: 'buffer',
        maxBuffer: 16 * 1024 * 1024,
        stdio: ['ignore', 'pipe', 'ignore'],
      },
    );
    if (result.status === 0 && result.stdout.length > 0) {
      return result.stdout;
    }
  }

  return null;
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function gradientEased(
  count: number,
  start: readonly [number, number, number],
  end: readonly [number, number, number],
  gamma: number,
) {
  const total = Math.max(count, 1);
  const denominator = Math.max(total - 1, 1);

  return Array.from({ length: total }, (_, index) => {
    const t = (index / denominator) ** gamma;
    return [
      lerp(start[0], end[0], t),
      lerp(start[1], end[1], t),
      lerp(start[2], end[2], t),
    ] as const;
  });
}

function colorize(text: string, colors: ReadonlyArray<readonly [number, number, number]>) {
  if (!text) {
    return '';
  }

  const chars = Array.from(text);
  const denominator = Math.max(chars.length - 1, 1);
  const maxColorIndex = Math.max(colors.length - 1, 0);

  let output = '';
  chars.forEach((char, index) => {
    const colorIndex = Math.round((index / denominator) * maxColorIndex);
    const [red, green, blue] = colors[colorIndex];
    output += `${CSI}38;2;${red};${green};${blue}m${char}`;
  });

  return `${output}${RESET}`;
}

function lerp(start: number, end: number, t: number) {
  return Math.round(start + (end - start) * t);
}

function bold(text: string) {
  return `${CSI}1m${text}${CSI}22m`;
}

function getAchievementTrophy(tier: CheckAchievementTier) {
  switch (tier) {
    case 'gold':
      return '🥇';
    case 'silver':
      return '🥈';
    default:
      return '🥉';
  }
}

function getFailureStreakMessage(kind: ClassifiedCheckFailureKind, count: number) {
  if (kind === 'lint') {
    return count === 3
      ? 'three lint errors in a row. discipline is dissolving.'
      : `${count} lint errors in a row. the rulebook has become a lifestyle.`;
  }

  if (kind === 'formatting') {
    return count === 3
      ? 'three formatting failures in a row. symmetry has left the building.'
      : `${count} formatting failures in a row. whitespace is now a recurring antagonist.`;
  }

  return count === 3
    ? 'three type-aware failures in a row. the checker is no longer amused.'
    : `${count} type-aware failures in a row. the type graph remains haunted.`;
}

const GENERIC_FAILURE_MESSAGES = [
  'outstanding, another error.',
  'superb, the situation remains incorrect.',
  'exceptional, a fresh mistake has entered the chat.',
  'stunning, another error.',
  'iconic, the bug persists.',
];

const FORMAT_FAILURE_MESSAGES = [
  'excellent, another formatting error.',
  'magnificent, the whitespace is in open revolt.',
  'astonishing, the file geometry has drifted.',
  'beautiful, another formatting error.',
  'sensational, the formatter remains emotionally unavailable.',
];

const LINT_FAILURE_MESSAGES = [
  'remarkable, another lint error.',
  'spectacular, a rule has been offended.',
  'immaculate, the linter found a fresh crime.',
  'delightful, another lint error.',
  'glorious, the rulebook has spoken again.',
];

const TYPE_AWARE_FAILURE_MESSAGES = [
  'incredible, another type-aware error.',
  'perfect, the type universe rejects this offering.',
  'marvelous, another type-aware error.',
  'unbelievable, the checker has seen enough.',
  'radiant, the type graph remains haunted.',
];

const RARE_FAILURE_MESSAGES = [
  'the solstice of bad decisions is upon us.',
  'a once-in-a-hundred alignment of terrible choices has been observed.',
  'the static analysis enthusiast has entered a rare celestial mood.',
];

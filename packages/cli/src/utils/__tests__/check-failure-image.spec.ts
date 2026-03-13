import { describe, expect, it } from 'vitest';

import {
  buildKittyImageSequence,
  buildOsc1337FileSequence,
  detectInlineImageProtocol,
  getCheckFailureCaption,
  getImagePlacement,
  normalizeCheckFailureKind,
  readPngDimensions,
  renderCheckFailureTextBlock,
  renderStaticAnalysisEnthusiastHeading,
} from '../check-failure-image.js';

const ansiEscapePattern = new RegExp(`${String.fromCharCode(27)}\\[[0-9;]*m`, 'g');

describe('detectInlineImageProtocol', () => {
  it('prefers the OSC 1337 file protocol for iTerm2 and WezTerm', () => {
    expect(detectInlineImageProtocol({ TERM_PROGRAM: 'iTerm.app' })).toBe('file');
    expect(detectInlineImageProtocol({ TERM_PROGRAM: 'WezTerm' })).toBe('file');
  });

  it('uses kitty graphics for kitty-compatible terminals', () => {
    expect(detectInlineImageProtocol({ KITTY_WINDOW_ID: '12' })).toBe('kitty');
    expect(detectInlineImageProtocol({ TERM_PROGRAM: 'ghostty' })).toBe('kitty');
    expect(detectInlineImageProtocol({ TERM: 'xterm-kitty' })).toBe('kitty');
  });

  it('falls back to sixel when advertised and an encoder is available', () => {
    expect(detectInlineImageProtocol({ TERM_FEATURES: 'RGBSx' }, { canRenderSixel: true })).toBe(
      'sixel',
    );
  });

  it('returns null when no supported image protocol is available', () => {
    expect(detectInlineImageProtocol({ TERM: 'xterm-256color' })).toBeNull();
  });
});

describe('inline image sequences', () => {
  it('builds a kitty graphics sequence with the encoded file path', () => {
    const sequence = buildKittyImageSequence('/tmp/error.png', {
      columns: 26,
      rows: 17,
    });

    expect(sequence).toContain('a=T,t=f,f=100,c=26,r=17');
    expect(sequence).toContain(Buffer.from('/tmp/error.png').toString('base64'));
    expect(sequence).toContain('\u001b_G');
  });

  it('builds an OSC 1337 file sequence with the encoded file contents', () => {
    const sequence = buildOsc1337FileSequence('/tmp/error.png', Buffer.from('png-bytes'), {
      columns: 26,
      rows: 17,
    });

    expect(sequence).toContain(']1337;File=');
    expect(sequence).toContain('width=26;height=17;preserveAspectRatio=1;inline=1');
    expect(sequence).toContain(Buffer.from('error.png').toString('base64'));
    expect(sequence).toContain(Buffer.from('png-bytes').toString('base64'));
  });
});

describe('PNG helpers', () => {
  it('reads dimensions from the PNG header and derives a bounded placement', () => {
    const png = Buffer.from(
      '89504e470d0a1a0a0000000d494844520000038a000004c0080600000000000000',
      'hex',
    );

    expect(readPngDimensions(png)).toEqual({ width: 906, height: 1216 });
    expect(getImagePlacement({ width: 906, height: 1216 })).toEqual({
      columns: 26,
      rows: 17,
    });
  });
});

describe('fun mode captions', () => {
  it('uses the requested default wording for the first generic failure', () => {
    expect(
      getCheckFailureCaption('error', {
        newlyUnlocked: [],
        state: {
          version: 2,
          totalFailures: 1,
          failureCounts: {
            formatting: 0,
            lint: 0,
            'type-aware': 0,
          },
          failureDays: ['2026-03-13'],
          unlockedAchievementIds: [],
          updatedAt: '2026-03-13T00:00:00.000Z',
          recentFailureKinds: ['error'],
          rareMomentsSeen: 0,
        },
        storageFile: '/tmp/static-analysis.json',
        collectionProgress: {
          unlocked: 0,
          total: 14,
        },
        visibleProgress: [],
        nearUnlocks: [],
        currentStreak: undefined,
        rareMomentTriggered: false,
      }),
    ).toBe('outstanding, another error.');
  });

  it('varies the caption by failure kind', () => {
    expect(getCheckFailureCaption('formatting')).toContain('formatting');
    expect(getCheckFailureCaption('lint')).toContain('lint');
    expect(getCheckFailureCaption('type-aware')).toContain('type-aware');
  });

  it('normalizes unknown failure kinds to the generic error caption', () => {
    expect(normalizeCheckFailureKind('formatting')).toBe('formatting');
    expect(normalizeCheckFailureKind('lint')).toBe('lint');
    expect(normalizeCheckFailureKind('type-aware')).toBe('type-aware');
    expect(normalizeCheckFailureKind('something-else')).toBe('error');
    expect(normalizeCheckFailureKind(null)).toBe('error');
  });

  it('renders the heading with bold rainbow styling when truecolor is available', () => {
    const heading = renderStaticAnalysisEnthusiastHeading(
      {
        getColorDepth: () => 24,
        isTTY: true,
      } as NodeJS.WriteStream,
      {},
    );

    expect(heading).toContain('\u001b[1m');
    expect(heading).toContain('38;2;');
    expect(heading.replace(ansiEscapePattern, '')).toContain(
      'The static analysis enthusiast says:',
    );
  });

  it('renders achievement unlock lines below the caption', () => {
    const block = renderCheckFailureTextBlock(
      {
        failureKind: 'lint',
        achievementUpdate: {
          newlyUnlocked: [
            {
              id: 'rule-breaker',
              title: 'Rule Breaker',
              description: 'Reach 10 lint failures.',
              tier: 'bronze',
            },
          ],
          state: {
            version: 2,
            totalFailures: 10,
            failureCounts: {
              formatting: 0,
              lint: 10,
              'type-aware': 0,
            },
            failureDays: ['2026-03-13'],
            unlockedAchievementIds: ['rule-breaker'],
            updatedAt: '2026-03-13T00:00:00.000Z',
            recentFailureKinds: ['lint', 'lint', 'lint'],
            rareMomentsSeen: 0,
          },
          storageFile: '/tmp/static-analysis.json',
          collectionProgress: {
            unlocked: 1,
            total: 14,
          },
          visibleProgress: [
            {
              achievement: {
                id: 'policy-enjoyer',
                title: 'Policy Enjoyer',
                description: 'Reach 25 lint failures.',
                tier: 'silver',
              },
              summary: '10/25 lint',
              remaining: 15,
            },
          ],
          nearUnlocks: [],
          currentStreak: {
            kind: 'lint',
            count: 3,
          },
          rareMomentTriggered: false,
        },
      },
      {
        getColorDepth: () => 1,
        isTTY: true,
      } as NodeJS.WriteStream,
      {},
    );

    expect(block).toContain('The static analysis enthusiast says:');
    expect(block).toContain('🎉');
    expect(block).toContain('🥉');
    expect(block).toContain('achievement unlocked:');
    expect(block).toContain('Rule Breaker');
    expect(block).toContain('collection progress: 1/14 achievements unlocked');
    expect(block).toContain('three lint errors in a row');
  });
});

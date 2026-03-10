import type { TerminalTranscript } from './terminal-transcripts';

export const featureRunTranscripts: TerminalTranscript[] = [
  {
    id: 'cold',
    label: 'Cold Cache',
    title: 'First run builds the shared library and app',
    command: 'vp run --cache build',
    lineDelay: 180,
    completionDelay: 1200,
    lines: [
      {
        segments: [{ text: '# First run builds the shared library and app', tone: 'muted' }],
      },
      {
        segments: [{ text: '$ vp pack', tone: 'muted' }],
      },
      {
        segments: [{ text: '$ vp build', tone: 'muted' }],
      },
      {
        segments: [
          { text: 'vp run:', tone: 'brand', bold: true },
          { text: ' 0/2 cache hit (0%).', tone: 'muted' },
        ],
      },
    ],
  },
  {
    id: 'no-changes',
    label: 'Full Replay',
    title: 'No changes replay both tasks from cache',
    command: 'vp run --cache build',
    lineDelay: 180,
    completionDelay: 1200,
    lines: [
      {
        segments: [{ text: '# No changes replay both tasks from cache', tone: 'muted' }],
      },
      {
        segments: [
          { text: '$ vp pack ', tone: 'muted' },
          { text: '✓ ', tone: 'success' },
          { text: 'cache hit, replaying', tone: 'base' },
        ],
      },
      {
        segments: [
          { text: '$ vp build ', tone: 'muted' },
          { text: '✓ ', tone: 'success' },
          { text: 'cache hit, replaying', tone: 'base' },
        ],
      },
      {
        segments: [
          { text: 'vp run:', tone: 'brand', bold: true },
          { text: ' 2/2 cache hit (100%), 1.24s saved.', tone: 'muted' },
        ],
      },
    ],
  },
  {
    id: 'app-change',
    label: 'Partial Replay',
    title: 'App changes rerun only the app build',
    command: 'vp run --cache build',
    lineDelay: 180,
    completionDelay: 1200,
    lines: [
      {
        segments: [{ text: '# App changes rerun only the app build', tone: 'muted' }],
      },
      {
        segments: [
          { text: '$ vp pack ', tone: 'muted' },
          { text: '✓ ', tone: 'success' },
          { text: 'cache hit, replaying', tone: 'base' },
        ],
      },
      {
        segments: [
          { text: '$ vp build ', tone: 'muted' },
          { text: '✗ ', tone: 'base' },
          { text: 'cache miss: ', tone: 'muted' },
          { text: "'src/main.ts'", tone: 'base' },
          { text: ' modified, executing', tone: 'muted' },
        ],
      },
      {
        segments: [
          { text: 'vp run:', tone: 'brand', bold: true },
          { text: ' 1/2 cache hit (50%), 528ms saved.', tone: 'muted' },
        ],
      },
    ],
  },
  {
    id: 'shared-change',
    label: 'Full Rebuild',
    title: 'Shared API changes rebuild the library and app',
    command: 'vp run --cache build',
    lineDelay: 180,
    completionDelay: 1200,
    lines: [
      {
        segments: [{ text: '# Shared API changes rebuild the library and app', tone: 'muted' }],
      },
      {
        segments: [
          { text: '$ vp pack ', tone: 'muted' },
          { text: '✗ ', tone: 'base' },
          { text: 'cache miss: ', tone: 'muted' },
          { text: "'src/index.ts'", tone: 'base' },
          { text: ' modified, executing', tone: 'muted' },
        ],
      },
      {
        segments: [
          { text: '$ vp build ', tone: 'muted' },
          { text: '✗ ', tone: 'base' },
          { text: 'cache miss: ', tone: 'muted' },
          { text: "'src/routes.ts'", tone: 'base' },
          { text: ' modified, executing', tone: 'muted' },
        ],
      },
      {
        segments: [
          { text: 'vp run:', tone: 'brand', bold: true },
          { text: ' 0/2 cache hit (0%).', tone: 'muted' },
        ],
      },
    ],
  },
];

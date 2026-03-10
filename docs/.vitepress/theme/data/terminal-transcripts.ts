export type TerminalTone = 'base' | 'muted' | 'brand' | 'accent' | 'success' | 'warning';

export interface TerminalSegment {
  text: string;
  tone?: TerminalTone;
  bold?: boolean;
}

export interface TerminalLine {
  segments: TerminalSegment[];
  tone?: TerminalTone;
}

export interface TerminalTranscript {
  id: string;
  label: string;
  title: string;
  command: string;
  prompt?: string;
  lineDelay?: number;
  completionDelay?: number;
  lines: TerminalLine[];
}

export const terminalTranscripts: TerminalTranscript[] = [
  {
    id: 'create',
    label: 'create',
    title: 'Scaffold a project',
    command: 'vp create acme-web --template react-ts',
    lineDelay: 220,
    completionDelay: 900,
    lines: [
      {
        segments: [
          { text: '◇ ', tone: 'accent' },
          { text: 'Scaffolded ', tone: 'muted' },
          { text: 'acme-web', tone: 'brand' },
          { text: ' with React + TypeScript', tone: 'muted' },
        ],
      },
      {
        segments: [
          { text: '• ', tone: 'muted' },
          { text: 'Node ', tone: 'muted' },
          { text: '22.15.1', tone: 'brand' },
          { text: '  pnpm ', tone: 'muted' },
          { text: '10.17.0', tone: 'accent' },
        ],
      },
      {
        segments: [
          { text: '✓ ', tone: 'success' },
          { text: 'Dependencies installed', tone: 'base' },
          { text: ' in 1.1s', tone: 'muted' },
        ],
      },
      {
        segments: [
          { text: '→ ', tone: 'brand' },
          { text: 'Next: ', tone: 'muted' },
          { text: 'cd acme-web && vp dev', tone: 'accent' },
        ],
      },
    ],
  },
  {
    id: 'dev',
    label: 'dev',
    title: 'Start local development',
    command: 'vp dev',
    lineDelay: 220,
    completionDelay: 1100,
    lines: [
      {
        segments: [
          { text: 'VITE+ ', tone: 'brand' },
          { text: 'ready in ', tone: 'muted' },
          { text: '68ms', tone: 'base' },
        ],
      },
      {
        segments: [
          { text: '→ ', tone: 'brand' },
          { text: 'Local ', tone: 'muted' },
          { text: 'http://localhost:5173/', tone: 'accent' },
        ],
      },
      {
        segments: [
          { text: '→ ', tone: 'muted' },
          { text: 'Network ', tone: 'muted' },
          { text: '--host', tone: 'base' },
          { text: ' to expose', tone: 'muted' },
        ],
      },
      {
        segments: [
          { text: '[hmr] ', tone: 'accent' },
          { text: 'updated ', tone: 'muted' },
          { text: 'src/App.tsx', tone: 'brand' },
          { text: ' in 14ms', tone: 'muted' },
        ],
      },
    ],
  },
  {
    id: 'check',
    label: 'check',
    title: 'Check the whole project',
    command: 'vp check',
    lineDelay: 220,
    completionDelay: 1100,
    lines: [
      {
        segments: [
          { text: 'pass: ', tone: 'accent' },
          { text: 'All 42 files are correctly formatted', tone: 'base' },
          { text: ' (88ms, 16 threads)', tone: 'muted' },
        ],
      },
      {
        segments: [
          { text: 'pass: ', tone: 'accent' },
          { text: 'Found no warnings, lint errors, or type errors', tone: 'base' },
          { text: ' in 42 files', tone: 'muted' },
          { text: ' (184ms, 16 threads)', tone: 'muted' },
        ],
      },
    ],
  },
  {
    id: 'test',
    label: 'test',
    title: 'Run tests with fast feedback',
    command: 'vp test',
    lineDelay: 220,
    completionDelay: 1100,
    lines: [
      {
        segments: [
          { text: 'RUN ', tone: 'muted' },
          { text: 'test/button.spec.ts', tone: 'brand' },
          { text: ' (3 tests)', tone: 'muted' },
        ],
      },
      {
        segments: [
          { text: '✓ ', tone: 'success' },
          { text: 'button renders loading state', tone: 'base' },
        ],
      },
      {
        segments: [
          { text: '✓ ', tone: 'success' },
          { text: '12 tests passed', tone: 'base' },
          { text: ' across 4 files', tone: 'muted' },
        ],
      },
      {
        segments: [
          { text: 'Duration ', tone: 'muted' },
          { text: '312ms', tone: 'accent' },
          { text: ' (transform 22ms, tests 31ms)', tone: 'muted' },
        ],
      },
    ],
  },
  {
    id: 'build',
    label: 'build',
    title: 'Ship a production build',
    command: 'vp build',
    lineDelay: 220,
    completionDelay: 1100,
    lines: [
      {
        segments: [
          { text: 'Rolldown ', tone: 'brand' },
          { text: 'building for production', tone: 'muted' },
        ],
      },
      {
        segments: [
          { text: '✓ ', tone: 'success' },
          { text: '128 modules transformed', tone: 'base' },
        ],
      },
      {
        segments: [
          { text: 'dist/assets/index-B6h2Q8.js', tone: 'accent' },
          { text: '  46.2 kB  gzip: 14.9 kB', tone: 'muted' },
        ],
      },
      {
        segments: [
          { text: 'dist/assets/index-H3a8K2.css', tone: 'brand' },
          { text: '  5.1 kB  gzip: 1.6 kB', tone: 'muted' },
        ],
      },
      {
        segments: [
          { text: '✓ ', tone: 'success' },
          { text: 'Built in ', tone: 'muted' },
          { text: '421ms', tone: 'base' },
        ],
      },
    ],
  },
];

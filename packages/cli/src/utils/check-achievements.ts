import fs from 'node:fs';
import path from 'node:path';

import type { CheckFailureKind } from './check-failure-image.js';
import { writeJsonFile } from './json.js';

export type ClassifiedCheckFailureKind = Exclude<CheckFailureKind, 'error'>;
export type CheckAchievementTier = 'bronze' | 'silver' | 'gold';

export interface CheckAchievement {
  id: string;
  title: string;
  description: string;
  tier: CheckAchievementTier;
  hidden?: boolean;
}

export interface CheckAchievementState {
  version: 2;
  totalFailures: number;
  failureCounts: Record<ClassifiedCheckFailureKind, number>;
  failureDays: string[];
  unlockedAchievementIds: string[];
  updatedAt: string;
  recentFailureKinds: CheckFailureKind[];
  rareMomentsSeen: number;
}

export interface CheckAchievementProgress {
  achievement: CheckAchievement;
  summary: string;
  remaining: number;
  taunt?: string;
}

export interface CheckAchievementUpdate {
  newlyUnlocked: CheckAchievement[];
  state: CheckAchievementState;
  storageFile: string;
  collectionProgress: {
    unlocked: number;
    total: number;
  };
  visibleProgress: CheckAchievementProgress[];
  nearUnlocks: CheckAchievementProgress[];
  currentStreak:
    | {
        kind: ClassifiedCheckFailureKind;
        count: number;
      }
    | undefined;
  rareMomentTriggered: boolean;
}

interface CheckAchievementDefinition extends CheckAchievement {
  unlockedBy: (state: CheckAchievementState) => boolean;
  progress?: (state: CheckAchievementState) => Omit<CheckAchievementProgress, 'achievement'>;
}

interface RecordFailureOptions {
  forceRareMoment?: boolean;
}

const ALL_CLASSIFIED_FAILURE_KINDS: ClassifiedCheckFailureKind[] = [
  'formatting',
  'lint',
  'type-aware',
];
const MAX_RECENT_FAILURE_KINDS = 12;

const ACHIEVEMENTS: CheckAchievementDefinition[] = [
  {
    id: 'first-contact',
    title: 'First Contact',
    description: 'Encounter your first static analysis failure.',
    tier: 'bronze',
    unlockedBy: (state) => state.totalFailures >= 1,
    progress: (state) =>
      buildProgress(
        state.totalFailures,
        1,
        '1/1',
        'you are one mistake away from making first contact.',
      ),
  },
  {
    id: 'paper-cuts',
    title: 'Paper Cuts',
    description: 'Reach 10 formatting failures.',
    tier: 'bronze',
    unlockedBy: (state) => state.failureCounts.formatting >= 10,
    progress: (state) =>
      buildProgress(
        state.failureCounts.formatting,
        10,
        `${state.failureCounts.formatting}/10 formatting`,
        'one more crooked line and Paper Cuts is yours.',
      ),
  },
  {
    id: 'kerning-chaos',
    title: 'Kerning Chaos',
    description: 'Reach 25 formatting failures.',
    tier: 'silver',
    unlockedBy: (state) => state.failureCounts.formatting >= 25,
    progress: (state) =>
      buildProgress(
        state.failureCounts.formatting,
        25,
        `${state.failureCounts.formatting}/25 formatting`,
        'one more typography incident and Kerning Chaos is yours.',
      ),
  },
  {
    id: 'rule-breaker',
    title: 'Rule Breaker',
    description: 'Reach 10 lint failures.',
    tier: 'bronze',
    unlockedBy: (state) => state.failureCounts.lint >= 10,
    progress: (state) =>
      buildProgress(
        state.failureCounts.lint,
        10,
        `${state.failureCounts.lint}/10 lint`,
        'one more lapse in judgment and Rule Breaker is yours.',
      ),
  },
  {
    id: 'policy-enjoyer',
    title: 'Policy Enjoyer',
    description: 'Reach 25 lint failures.',
    tier: 'silver',
    unlockedBy: (state) => state.failureCounts.lint >= 25,
    progress: (state) =>
      buildProgress(
        state.failureCounts.lint,
        25,
        `${state.failureCounts.lint}/25 lint`,
        'one more rules violation and Policy Enjoyer is yours.',
      ),
  },
  {
    id: 'type-tussler',
    title: 'Type Tussler',
    description: 'Reach 10 type-aware failures.',
    tier: 'bronze',
    unlockedBy: (state) => state.failureCounts['type-aware'] >= 10,
    progress: (state) =>
      buildProgress(
        state.failureCounts['type-aware'],
        10,
        `${state.failureCounts['type-aware']}/10 type-aware`,
        'one more skirmish with the checker and Type Tussler is yours.',
      ),
  },
  {
    id: 'compiler-provocateur',
    title: 'Compiler Provocateur',
    description: 'Reach 25 type-aware failures.',
    tier: 'silver',
    unlockedBy: (state) => state.failureCounts['type-aware'] >= 25,
    progress: (state) =>
      buildProgress(
        state.failureCounts['type-aware'],
        25,
        `${state.failureCounts['type-aware']}/25 type-aware`,
        'one more type incident and Compiler Provocateur is yours.',
      ),
  },
  {
    id: 'fan-club-member',
    title: 'Fan Club Member',
    description: 'Reach 25 total failures.',
    tier: 'silver',
    unlockedBy: (state) => state.totalFailures >= 25,
    progress: (state) =>
      buildProgress(
        state.totalFailures,
        25,
        `${state.totalFailures}/25 total`,
        'one more failure and the Fan Club will finally let you in.',
      ),
  },
  {
    id: 'century-club',
    title: 'Century Club',
    description: 'Reach 100 total failures.',
    tier: 'gold',
    unlockedBy: (state) => state.totalFailures >= 100,
    progress: (state) =>
      buildProgress(
        state.totalFailures,
        100,
        `${state.totalFailures}/100 total`,
        'one more glorious collapse and Century Club is yours.',
      ),
  },
  {
    id: 'balanced-breakfast',
    title: 'Balanced Breakfast',
    description: 'Reach 5 formatting, 5 lint, and 5 type-aware failures.',
    tier: 'silver',
    unlockedBy: (state) =>
      state.failureCounts.formatting >= 5 &&
      state.failureCounts.lint >= 5 &&
      state.failureCounts['type-aware'] >= 5,
    progress: (state) => {
      const remaining =
        Math.max(0, 5 - state.failureCounts.formatting) +
        Math.max(0, 5 - state.failureCounts.lint) +
        Math.max(0, 5 - state.failureCounts['type-aware']);
      return {
        remaining,
        summary:
          `fmt ${state.failureCounts.formatting}/5, ` +
          `lint ${state.failureCounts.lint}/5, ` +
          `type ${state.failureCounts['type-aware']}/5`,
        taunt:
          remaining === 1
            ? 'one more balanced disaster and Balanced Breakfast is served.'
            : undefined,
      };
    },
  },
  {
    id: 'weekend-residency',
    title: 'Weekend Residency',
    description: 'Fail on 7 distinct calendar days.',
    tier: 'gold',
    unlockedBy: (state) => state.failureDays.length >= 7,
    progress: (state) =>
      buildProgress(
        state.failureDays.length,
        7,
        `${state.failureDays.length}/7 days`,
        'one more day of mistakes and Weekend Residency is yours.',
      ),
  },
  {
    id: 'truly-full-stack',
    title: 'Truly Full-Stack',
    description: 'Hit formatting, lint, and type-aware failures across three consecutive runs.',
    tier: 'gold',
    hidden: true,
    unlockedBy: (state) => {
      const lastThreeKinds = getLastClassifiedFailureKinds(state, 3);
      return (
        lastThreeKinds.length === 3 &&
        ALL_CLASSIFIED_FAILURE_KINDS.every((kind) => lastThreeKinds.includes(kind))
      );
    },
  },
  {
    id: 'discipline-is-dissolving',
    title: 'Discipline Is Dissolving',
    description: 'Trigger the same classified failure kind three times in a row.',
    tier: 'silver',
    hidden: true,
    unlockedBy: (state) => (getCurrentClassifiedFailureStreak(state)?.count ?? 0) >= 3,
  },
  {
    id: 'bad-moon-rising',
    title: 'Bad Moon Rising',
    description: 'Witness a one-in-a-hundred omen from the static analysis enthusiast.',
    tier: 'gold',
    hidden: true,
    unlockedBy: (state) => state.rareMomentsSeen >= 1,
  },
];

export function getCheckAchievementStorePath(projectRoot: string) {
  return path.join(projectRoot, 'node_modules', '.vite', 'achievements', 'static-analysis.json');
}

export function recordCheckFailureAchievement(
  projectRoot: string,
  failureKind: CheckFailureKind,
  now = new Date(),
  options: RecordFailureOptions = {},
): CheckAchievementUpdate {
  const storageFile = getCheckAchievementStorePath(projectRoot);
  const state = readAchievementState(storageFile);

  state.totalFailures += 1;
  state.updatedAt = now.toISOString();
  state.recentFailureKinds.push(failureKind);
  state.recentFailureKinds = state.recentFailureKinds.slice(-MAX_RECENT_FAILURE_KINDS);

  if (failureKind !== 'error') {
    state.failureCounts[failureKind] += 1;
  }

  const failureDay = state.updatedAt.slice(0, 10);
  if (!state.failureDays.includes(failureDay)) {
    state.failureDays.push(failureDay);
    state.failureDays.sort();
  }

  const rareMomentTriggered =
    options.forceRareMoment ??
    isRareCelebrationMoment(state.totalFailures, state.updatedAt, failureKind);
  if (rareMomentTriggered) {
    state.rareMomentsSeen += 1;
  }

  const newlyUnlocked = ACHIEVEMENTS.filter((achievement) => {
    if (state.unlockedAchievementIds.includes(achievement.id)) {
      return false;
    }
    return achievement.unlockedBy(state);
  }).map(({ id, title, description, tier, hidden }) => ({
    id,
    title,
    description,
    tier,
    hidden,
  }));

  if (newlyUnlocked.length > 0) {
    state.unlockedAchievementIds.push(...newlyUnlocked.map((achievement) => achievement.id));
    state.unlockedAchievementIds.sort();
  }

  fs.mkdirSync(path.dirname(storageFile), { recursive: true });
  writeJsonFile(storageFile, state);

  const visibleProgress: CheckAchievementProgress[] = ACHIEVEMENTS.filter(
    (achievement) =>
      !achievement.hidden &&
      !state.unlockedAchievementIds.includes(achievement.id) &&
      achievement.progress,
  ).map((achievement) =>
    Object.assign(
      {
        achievement: {
          id: achievement.id,
          title: achievement.title,
          description: achievement.description,
          tier: achievement.tier,
          hidden: achievement.hidden,
        },
      },
      achievement.progress!(state),
    ),
  );
  // eslint-disable-next-line unicorn/no-array-sort -- current TS lib target does not expose Array#toSorted
  visibleProgress.sort(
    (left, right) =>
      left.remaining - right.remaining ||
      left.achievement.title.localeCompare(right.achievement.title),
  );

  return {
    newlyUnlocked,
    state,
    storageFile,
    collectionProgress: {
      unlocked: state.unlockedAchievementIds.length,
      total: ACHIEVEMENTS.length,
    },
    visibleProgress: visibleProgress.slice(0, 3),
    nearUnlocks: visibleProgress.filter((progress) => progress.remaining === 1).slice(0, 2),
    currentStreak: getCurrentClassifiedFailureStreak(state),
    rareMomentTriggered,
  };
}

export function isRareCelebrationMoment(
  totalFailures: number,
  updatedAt: string,
  failureKind: CheckFailureKind,
) {
  const source = `${totalFailures}:${updatedAt}:${failureKind}`;
  let hash = 0;
  for (const char of source) {
    hash = (hash * 31 + char.charCodeAt(0)) % 100;
  }
  return hash === 0;
}

function readAchievementState(storageFile: string): CheckAchievementState {
  if (!fs.existsSync(storageFile)) {
    return createEmptyAchievementState();
  }

  try {
    const content = fs.readFileSync(storageFile, 'utf8');
    const parsed = JSON.parse(content) as Partial<CheckAchievementState>;

    return {
      version: 2,
      totalFailures: parsed.totalFailures ?? 0,
      failureCounts: {
        formatting: parsed.failureCounts?.formatting ?? 0,
        lint: parsed.failureCounts?.lint ?? 0,
        'type-aware': parsed.failureCounts?.['type-aware'] ?? 0,
      },
      failureDays: sortUniqueStrings(parsed.failureDays ?? []),
      unlockedAchievementIds: sortUniqueStrings(parsed.unlockedAchievementIds ?? []),
      updatedAt: parsed.updatedAt ?? new Date(0).toISOString(),
      recentFailureKinds: (parsed.recentFailureKinds ?? [])
        .filter(isCheckFailureKind)
        .slice(-MAX_RECENT_FAILURE_KINDS),
      rareMomentsSeen: parsed.rareMomentsSeen ?? 0,
    };
  } catch {
    return createEmptyAchievementState();
  }
}

function createEmptyAchievementState(): CheckAchievementState {
  return {
    version: 2,
    totalFailures: 0,
    failureCounts: {
      formatting: 0,
      lint: 0,
      'type-aware': 0,
    },
    failureDays: [],
    unlockedAchievementIds: [],
    updatedAt: new Date(0).toISOString(),
    recentFailureKinds: [],
    rareMomentsSeen: 0,
  };
}

function getLastClassifiedFailureKinds(state: CheckAchievementState, count: number) {
  return state.recentFailureKinds.filter(isClassifiedCheckFailureKind).slice(-count);
}

function getCurrentClassifiedFailureStreak(state: CheckAchievementState) {
  const recentKinds = state.recentFailureKinds.filter(isClassifiedCheckFailureKind);
  const lastKind = recentKinds.at(-1);
  if (!lastKind) {
    return undefined;
  }

  let count = 0;
  for (let index = recentKinds.length - 1; index >= 0; index -= 1) {
    if (recentKinds[index] !== lastKind) {
      break;
    }
    count += 1;
  }

  return { kind: lastKind, count };
}

function isClassifiedCheckFailureKind(
  value: CheckFailureKind,
): value is ClassifiedCheckFailureKind {
  return value !== 'error';
}

function isCheckFailureKind(value: unknown): value is CheckFailureKind {
  return value === 'error' || value === 'formatting' || value === 'lint' || value === 'type-aware';
}

function buildProgress(current: number, target: number, summary: string, taunt: string) {
  return {
    remaining: Math.max(0, target - current),
    summary,
    taunt: current === target - 1 ? taunt : undefined,
  };
}

function sortUniqueStrings(values: string[]) {
  // eslint-disable-next-line unicorn/no-array-sort -- current TS lib target does not expose Array#toSorted
  return [...new Set(values)].sort();
}

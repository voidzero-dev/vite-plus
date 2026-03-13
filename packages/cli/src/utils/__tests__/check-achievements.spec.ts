import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import {
  getCheckAchievementStorePath,
  recordCheckFailureAchievement,
} from '../check-achievements.js';

const tempDirs: string[] = [];

afterEach(() => {
  while (tempDirs.length > 0) {
    fs.rmSync(tempDirs.pop()!, { force: true, recursive: true });
  }
});

function createProjectRoot() {
  const projectRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vite-plus-achievements-'));
  tempDirs.push(projectRoot);
  return projectRoot;
}

describe('recordCheckFailureAchievement', () => {
  it('persists counters to node_modules/.vite/achievements', () => {
    const projectRoot = createProjectRoot();

    const update = recordCheckFailureAchievement(
      projectRoot,
      'formatting',
      new Date('2026-03-13T10:00:00.000Z'),
    );

    expect(update.storageFile).toBe(getCheckAchievementStorePath(projectRoot));
    expect(update.state.totalFailures).toBe(1);
    expect(update.state.failureCounts.formatting).toBe(1);
    expect(update.newlyUnlocked.map((achievement) => achievement.id)).toContain('first-contact');
    expect(update.collectionProgress).toEqual({ unlocked: 1, total: 14 });
    expect(fs.existsSync(update.storageFile)).toBe(true);
  });

  it('unlocks threshold achievements over time', () => {
    const projectRoot = createProjectRoot();
    let update = recordCheckFailureAchievement(
      projectRoot,
      'lint',
      new Date('2026-03-13T10:00:00.000Z'),
    );

    for (let index = 1; index < 10; index += 1) {
      update = recordCheckFailureAchievement(
        projectRoot,
        'lint',
        new Date(`2026-03-13T10:${String(index).padStart(2, '0')}:00.000Z`),
      );
    }

    expect(update.newlyUnlocked.map((achievement) => achievement.id)).toContain('rule-breaker');
    expect(update.state.failureCounts.lint).toBe(10);
  });

  it('unlocks variety and time-based achievements', () => {
    const projectRoot = createProjectRoot();

    for (let day = 1; day <= 7; day += 1) {
      const isoDay = `2026-03-${String(day).padStart(2, '0')}`;
      const formattingUpdate = recordCheckFailureAchievement(
        projectRoot,
        'formatting',
        new Date(`${isoDay}T10:00:00.000Z`),
      );
      recordCheckFailureAchievement(projectRoot, 'lint', new Date(`${isoDay}T10:10:00.000Z`));
      const typeAwareUpdate = recordCheckFailureAchievement(
        projectRoot,
        'type-aware',
        new Date(`${isoDay}T10:20:00.000Z`),
      );

      if (day === 5) {
        expect(typeAwareUpdate.newlyUnlocked.map((achievement) => achievement.id)).toContain(
          'balanced-breakfast',
        );
      }

      if (day === 7) {
        expect(formattingUpdate.newlyUnlocked.map((achievement) => achievement.id)).toContain(
          'weekend-residency',
        );
      }
    }
  });

  it('unlocks the combo achievement across three consecutive classified runs', () => {
    const projectRoot = createProjectRoot();

    recordCheckFailureAchievement(projectRoot, 'formatting', new Date('2026-03-13T10:00:00.000Z'));
    recordCheckFailureAchievement(projectRoot, 'lint', new Date('2026-03-13T10:10:00.000Z'));
    const update = recordCheckFailureAchievement(
      projectRoot,
      'type-aware',
      new Date('2026-03-13T10:20:00.000Z'),
    );

    expect(update.newlyUnlocked.map((achievement) => achievement.id)).toContain('truly-full-stack');
  });

  it('unlocks the hidden streak achievement after three identical classified failures', () => {
    const projectRoot = createProjectRoot();

    recordCheckFailureAchievement(projectRoot, 'lint', new Date('2026-03-13T10:00:00.000Z'));
    recordCheckFailureAchievement(projectRoot, 'lint', new Date('2026-03-13T10:10:00.000Z'));
    const update = recordCheckFailureAchievement(
      projectRoot,
      'lint',
      new Date('2026-03-13T10:20:00.000Z'),
    );

    expect(update.currentStreak).toEqual({ kind: 'lint', count: 3 });
    expect(update.newlyUnlocked.map((achievement) => achievement.id)).toContain(
      'discipline-is-dissolving',
    );
  });

  it('tracks near-unlock taunts and rare hidden achievements', () => {
    const projectRoot = createProjectRoot();

    for (let index = 0; index < 9; index += 1) {
      recordCheckFailureAchievement(
        projectRoot,
        'lint',
        new Date(`2026-03-13T10:${String(index).padStart(2, '0')}:00.000Z`),
      );
    }

    const nearUnlockUpdate = recordCheckFailureAchievement(
      projectRoot,
      'formatting',
      new Date('2026-03-13T11:00:00.000Z'),
    );
    expect(nearUnlockUpdate.nearUnlocks.map((progress) => progress.achievement.id)).toContain(
      'rule-breaker',
    );

    const rareUpdate = recordCheckFailureAchievement(
      projectRoot,
      'formatting',
      new Date('2026-03-13T11:10:00.000Z'),
      { forceRareMoment: true },
    );
    expect(rareUpdate.rareMomentTriggered).toBe(true);
    expect(rareUpdate.newlyUnlocked.map((achievement) => achievement.id)).toContain(
      'bad-moon-rising',
    );
  });
});

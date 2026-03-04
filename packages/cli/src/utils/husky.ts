import { existsSync, readFileSync, readdirSync, unlinkSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

// Husky v8 bootstrap pattern — stripped entirely.
export const HUSKY_BOOTSTRAP_PATTERN = /^\.\s+".*husky\.sh"/;

/**
 * Strip the stale `. "…/husky.sh"` bootstrap line from every user-defined
 * hook file in the husky directory. Repos switching from Husky v8 still have
 * these lines, and they break once the old husky.sh is removed by vp prepare.
 */
export function stripHuskyBootstrapFromHooks(huskyDir: string): void {
  if (!existsSync(huskyDir)) {
    return;
  }
  const entries = readdirSync(huskyDir, { withFileTypes: true });
  for (const entry of entries) {
    if (!entry.isFile()) {
      continue;
    }
    const hookPath = join(huskyDir, entry.name);
    const content = readFileSync(hookPath, 'utf8');
    const lines = content.split('\n');
    const filtered = lines.filter((line) => !HUSKY_BOOTSTRAP_PATTERN.test(line.trim()));
    if (filtered.length === lines.length) {
      continue;
    }
    const newContent = filtered.join('\n').trim();
    if (newContent.length === 0) {
      unlinkSync(hookPath);
    } else {
      writeFileSync(hookPath, `${newContent}\n`);
    }
  }
}

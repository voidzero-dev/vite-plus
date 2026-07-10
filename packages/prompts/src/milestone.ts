/**
 * Render-milestone markers for the PTY snapshot suite
 * (crates/vite_cli_snapshots): invisible window-title updates the runner
 * synchronizes on via `expect-milestone` interactions. Emission is gated on
 * `VP_EMIT_MILESTONES=1`, which only the runner sets; real terminals and
 * piped output never see these markers as content (they only update the
 * window title, which the runner's PTY captures).
 *
 * Protocol (shared with vite-task's `pty_terminal_test_client` crate):
 * `OSC 2 ; pty-terminal-test:<32-hex-id>:<base64url(name)> ST`. The runner
 * decodes the title with `decode_milestone_title`, ignoring ordinary title
 * updates. A fresh random id per emission keeps repeated milestones with the
 * same name observable as distinct title changes through Windows ConPTY.
 */

import { randomBytes } from 'node:crypto';

const MILESTONE_TITLE_MARKER = 'pty-terminal-test:';
const OSC2_OPEN = '\x1b]2;';
const ST = '\x1b\\';

// Cached at module load: the runner sets the env before spawning the CLI,
// and milestone() runs on every prompt render (every keystroke), so the
// disabled path must stay a single branch.
const MILESTONES_ENABLED = process.env.VP_EMIT_MILESTONES === '1';

/**
 * Returns the encoded milestone byte sequence for `name`, or an empty string
 * when emission is disabled. Append the result to a rendered prompt frame so
 * the marker arrives in the output stream together with the render it marks.
 */
export function milestone(name: string): string {
  if (!MILESTONES_ENABLED) {
    return '';
  }
  // 16 random bytes → 32 lowercase hex chars, matching the Rust
  // `{id:032x}` u128 formatting that `decode_milestone_title` validates.
  const id = randomBytes(16).toString('hex');
  const encodedName = Buffer.from(name, 'utf8').toString('base64url');
  return `${OSC2_OPEN}${MILESTONE_TITLE_MARKER}${id}:${encodedName}${ST}`;
}

/**
 * Milestone for a prompt render, following the runner naming convention
 * `<kind>:<id>:<state>` (e.g. `select:template:1`, `confirm:approve:yes`,
 * `text:project-name:my-app`). `id` defaults to the prompt kind; pass
 * `testId` at ambiguous call sites (multiple prompts of one kind in a flow).
 */
export function promptMilestone(kind: string, id: string | undefined, state: string): string {
  return milestone(`${kind}:${id ?? kind}:${state}`);
}

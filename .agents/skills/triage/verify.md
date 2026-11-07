# Verify

Decide whether the reproduced behavior is a Vite+ bug or intended behavior.

**Read and append to `report.md` before returning. Do not fix the bug in this step.**

## 1. State the Claim

Record the observed behavior and the behavior the reporter expects. Do not assume the reporter is correct.

## 2. Research Intent

Check these sources:

- Product documentation in `README.md`, `docs/guide/`, and `docs/config/`.
- Design notes in `rfcs/`.
- Code comments, explicit conditionals, tests, and CLI snapshots near the affected code.
- `git blame` and `git show --no-patch <commit>` for relevant lines.
- Prior discussions with `gh search issues --repo voidzero-dev/vite-plus "<keywords>"` and `gh search prs --repo voidzero-dev/vite-plus "<keywords>"`.

If Vite+ intentionally forwards unchanged behavior from Vite, Vitest, Rolldown, tsdown, Oxlint, or Oxfmt, classify it as intended Vite+ behavior and note the upstream owner.

## 3. Return a Verdict

- `bug`: the behavior contradicts docs, tests, explicit intent, or a previous release without an intentional change.
- `intended-behavior`: docs, code, tests, or maintainers show that Vite+ chose this behavior.
- `unclear`: the evidence does not establish intent.

Use `high` confidence for direct evidence, `medium` for reasonable evidence with ambiguity, and `low` for inference.

## 4. Append to `report.md`

Include the claim, verdict, confidence, and evidence with file paths, commits, issues, or pull requests.

# Codespaces comparison of lint backends

This recipe compares the explicit Go override with the default tsrs backend in
the same built CLI. It runs three workloads on both Vite+ and vinext: type-aware
rules, rules with TypeScript diagnostics, and the full `vp check` command.

Use the [codespaces-run skill](https://github.com/Dunqing/skills/tree/main/skills/codespaces-run).
The recipe uses one `standardLinux32gb` instance with four CPUs and 16 GB of RAM.
The skill retains the environment and stops the instance after artifact collection.
Do not run other builds or benchmarks on the instance during measurement.

## Source and setup

The measured CLI source is `22b07bd150aa57b3e04cc52e0b81d43294ffe982`.
The RFC patch from `da260f86fce1d43810c62e4e6aad011066c88cb9` preserves the
documentation edits present when this comparison started. The vinext corpus is
`249088af307c0a852fa6dc59c08fc6ec80e1620e` from `fengmk2/vinext`.

Setup installs Node.js `24.22.2` for Vite+ and `26.13.0` for vinext. It installs
the package managers declared by those revisions: pnpm `12.11.2` and `11.1.1`.
Both projects and Vite+'s separate documentation workspace use frozen lockfiles.
The Python environment pins `psutil==7.0.0`.
Node downloads are checked against the official SHA256 files.

The recipe builds the PR's CLI JavaScript. It uses the published Vite+ `1.5.0`
native binding, core, and prompts packages, plus the matching Rolldown binding.
It does not compile a complete native release. The setup receipt records hashes
of the CLI entry point, native binding, Node executables, and benchmark config.

The repository's Ubuntu devcontainer can lack an SSH server. If `gh codespace ssh`
reports that problem, open the existing Codespace's web terminal and install
`openssh-server` with `sudo apt-get update` and
`sudo apt-get install -y --no-install-recommends openssh-server`.
Keep the repository's devcontainer configuration unchanged. Setup installs
`python3-venv` only if Python cannot import `ensurepip`.

## Run

Run these commands from this checkout. The temporary input directory stays
outside the source tree so it cannot change the measured file set.

```sh
tsrs_skill_tool="${CODEX_HOME:-$HOME/.codex}/skills/codespaces-run/scripts/sandbox.py"
tsrs_bench_inputs="$(mktemp -d)"
cp bench/tsrs-lint-codespaces/inputs/* "$tsrs_bench_inputs/"
git diff 22b07bd150aa57b3e04cc52e0b81d43294ffe982 da260f86fce1d43810c62e4e6aad011066c88cb9 \
  -- rfcs/check-command.md rfcs/toolchain-command.md rfcs/migration-command.md \
  > "$tsrs_bench_inputs/rfcs.patch"

python3 "$tsrs_skill_tool" submit \
  --repo voidzero-dev/vite-plus \
  --git-url https://github.com/voidzero-dev/vite-plus.git \
  --git-commit 22b07bd150aa57b3e04cc52e0b81d43294ffe982 \
  --stages "$PWD/bench/tsrs-lint-codespaces/stages.json" \
  --inputs "$tsrs_bench_inputs" \
  --machine standardLinux32gb --cpus 4 --min-memory-mib 14000 \
  --timeout 7200 --profile default \
  --environment-key tsrs-022-linux-x64-node24.22.2-26.13.0-pnpm12.11.2-11.1.1

python3 "$tsrs_skill_tool" events JOB_ID --follow
```

Use the returned job ID in the last command. Check the final receipt for
`collected: true`, a shutdown timestamp, and no cleanup error. Logs, raw samples,
memory traces, and results are retained under the skill's local job directory.
If a later stage fails, use the skill's stage recovery procedure to retain a
verified setup. Do not start another job while the previous job holds the lease.

## Method

Validation runs each backend once for each workload before measurement. Every
run must exit successfully with no diagnostics and the expected file and rule
counts. Full checks must also report successful formatting. Memory runs check
the native executable identity to detect accidental comparisons of one backend
against itself.

Timing uses ten pairs after one warm-up per backend. Each pair reverses the
previous pair's order. Memory uses a separate five-pair phase with its own
warm-ups. Each invocation starts a fresh CLI process. Installation, builds,
validation, and memory sampling are excluded from the timing table.

The memory sampler records the simultaneous sum of resident set size (RSS) for
the CLI and its descendants. It sleeps for 20 ms between samples and records the
actual intervals. This includes Node, Oxlint, and the native type-aware backend.
Shared pages can count more than once, and sampling can miss short peaks. The
result is a sampled process-tree peak, not total allocation or container memory.

Lint uses four threads. Vinext lint uses `OXLINT_TSGOLINT_CHECKER_SCHEDULING=sorted`,
as in the earlier comparison. Full checks use the machine's default thread count
and backend scheduling. The rules-only configuration disables `typeCheck` while
preserving the project's original lint rules and plugins.

The summary reports medians and ranges. Warm-up samples have `pair=-1` and do
not contribute to the medians. Each full run produces 204 records, including
warm-ups, plus the separate 12-command validation phase.

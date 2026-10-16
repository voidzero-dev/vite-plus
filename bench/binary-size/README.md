# Binary size analysis

The target is a 50% reduction for **each binary individually**. The measured
changes below do not yet meet that target. The most useful results so far are
size-oriented compilation for the standalone `vp` executable and compression of
embedded helper executables for the macOS NAPI library. Applying size-oriented
compilation to the entire NAPI library causes a substantial performance loss.

This draft applies `opt-level = "z"` to the standalone CLI build steps in
`build-upstream` and `build-windows-cli`. These are the compiler-only experiments
below: 37.9% on macOS, 29.7% on Linux, and 37.4% on Windows. Rebuilt std,
package-manager changes, and helper compression remain separate prototypes.
The preview workflow also compares all eight targets with the latest stable
release, using release archives for executables and matching npm versions for
NAPI. It reports raw sizes and gzip sizes separately.

The source baseline is `006e9700a`, which merges PR 1783. The
[reference size report](https://github.com/voidzero-dev/vite-plus/pull/1783#issuecomment-5308800690)
is for PR head `1438b2b`, updated on 2026-10-16. Raw bytes and gzip bytes are
separate metrics. Local experiments use matched local baselines; local builds
are not byte-identical to the CI artifacts.

## Release baselines and targets

These are the PR-head measurements downloaded from the reference workflow.
MiB means 1,048,576 bytes; KiB means 1,024 bytes.

| Artifact               | Release size | 50% target |
| ---------------------- | -----------: | ---------: |
| Linux x64 `vp`         |    10.21 MiB |   5.11 MiB |
| Linux x64 NAPI         |    34.53 MiB |  17.27 MiB |
| macOS ARM64 `vp`       |     7.62 MiB |   3.81 MiB |
| macOS ARM64 NAPI       |    41.52 MiB |  20.76 MiB |
| Windows x64 `vp`       |     8.29 MiB |   4.14 MiB |
| Windows x64 NAPI       |    30.60 MiB |  15.30 MiB |
| Windows x64 installer  |     3.63 MiB |   1.82 MiB |
| Windows x64 trampoline |    13.50 KiB |   6.75 KiB |

The release profile already uses `opt-level = 3`, full LTO, one codegen unit,
symbol stripping, and panic aborts. Turning those existing settings on cannot
produce additional savings. See [Cargo.toml](../../Cargo.toml).

In the matched macOS baseline, `vp` has 6.02 MiB of machine code and 1.01 MiB of
read-only constants. NAPI has 19.98 MiB of machine code and 19.85 MiB of read-only
constants. This explains the different priorities: code generation matters most
for `vp`, while embedded data is a large immediate opportunity for macOS NAPI.
The [symbol estimates](evidence/) include the complete section measurements.

## Measured compiler and source experiments

| Artifact and change                                              | Baseline bytes | Candidate bytes | Reduction |
| ---------------------------------------------------------------- | -------------: | --------------: | --------: |
| macOS `vp`, `opt-level = "z"`                                    |      7,963,056 |       4,943,936 |     37.9% |
| macOS `vp`, `"z"` plus rebuilding std for size                   |      7,963,056 |       4,631,296 |     41.8% |
| macOS `vp`, above plus package-manager prototype                 |      7,963,056 |       4,614,576 |     42.1% |
| Linux `vp`, `opt-level = "z"`                                    |     10,667,136 |       7,500,528 |     29.7% |
| Linux `vp`, `"z"` plus rebuilding std for size                   |     10,667,136 |       7,229,544 |     32.2% |
| Windows `vp`, `opt-level = "z"`                                  |      8,688,640 |       5,436,928 |     37.4% |
| Windows installer, entire dependency graph at `"z"`              |      3,808,256 |       2,947,072 |     22.6% |
| macOS NAPI, `opt-level = "z"`                                    |     43,305,840 |      31,779,504 |     26.6% |
| macOS NAPI, compressed embedded helpers, original optimization   |     43,305,840 |      32,980,192 |     23.8% |
| Linux NAPI, compressed embedded helpers, original optimization   |     35,672,200 |      35,016,840 |      1.8% |
| Windows NAPI, compressed embedded helpers, original optimization |     32,076,288 |      31,751,680 |      1.0% |

Rebuilding std retains the `backtrace` and `panic-unwind` std features and adds
`optimize_for_size`. The executable still uses `panic = "abort"`. None of these
experiments suppresses `Debug` output or uses immediate aborts that bypass the
panic hook.

The compressed NAPI library's gzip size falls from 17,932,754 to 16,362,251 bytes,
an 8.8% reduction. Raw installed-size savings must not be presented as equivalent
download savings.

Four native help command outputs and exit codes match between the original `vp`,
the `"z"` build, the build with recompiled std, and the package-manager prototype.
These checks do not establish
full CLI equivalence or package-manager performance.

Linux and Windows artifacts were cross-built and measured, but were not executed
on their target platforms. Linux builds use `cargo zigbuild` with
`RUSTFLAGS='--cfg tokio_unstable'` to omit the GNU linker warning flag that Zig
rejects. Windows builds use `cargo xwin` with Homebrew LLVM on `PATH`.

### NAPI performance

The synthetic benchmark uses 1,500 exported functions, TypeScript transformation
to ES2015 with source maps, minification with source maps, and an invalid-syntax
case. Output hashes match across the builds.

Global `"z"` optimization makes the initial parse test about 1.9 times as slow,
and transform and minify about 2.8 times as slow. Do not apply it globally to the
performance-sensitive library merely to meet the byte target.

Repeated, interleaved runs of the original and compressed libraries give median
times of approximately 4.36/4.38 ms for parsing, 2.13/2.15 ms for transformation,
and 2.07/2.05 ms for minification. These small kernels show no comparable slowdown
from compression. Application build and task-cache benchmarks remain necessary.
First-use extraction latency and memory use also need measurement; the existing
materialized-file fast path does not decompress the helpers again.

## Optimization priorities

Start with a dedicated size profile for the standalone CLI and installer, with
command latency and installation benchmarks before release. Pursue embedded
artifact compression independently in Vite Task. Keep Oxc and Rolldown at their
original optimization level until a selective profile passes representative
build benchmarks. Do not add the profile and compression percentages: a combined
NAPI variant has not been measured.

These experiments do not establish a route to 50% for every binary. The remaining
gap requires source or dependency changes with a measurable size budget. The
parser and verification audits below are investigation targets, not promised
savings.

This budget uses matched local baselines and the candidates above. For NAPI it
uses compression at the original optimization level, excluding the global
`"z"` profile because of its measured slowdown. No candidate is release-ready.

| Artifact               | Candidate bytes | Local 50% target | Additional bytes to remove |
| ---------------------- | --------------: | ---------------: | -------------------------: |
| macOS ARM64 `vp`       |       4,614,576 |        3,981,528 |                    633,048 |
| Linux x64 `vp`         |       7,229,544 |        5,333,568 |                  1,895,976 |
| Windows x64 `vp`       |       5,436,928 |        4,344,320 |                  1,092,608 |
| macOS ARM64 NAPI       |      32,980,192 |       21,652,920 |                 11,327,272 |
| Linux x64 NAPI         |      35,016,840 |       17,836,100 |                 17,180,740 |
| Windows x64 NAPI       |      31,751,680 |       16,038,144 |                 15,713,536 |
| Windows x64 installer  |       2,947,072 |        1,904,128 |                  1,042,944 |
| Windows x64 trampoline |          13,824 |            6,912 |                      6,912 |

### Compress embedded helpers

Vite Task's `fspy` embeds macOS copies of `coreutils` and Oils, totaling 14,141,272
bytes. It also embeds a preload library, and `vt` embeds a NAPI client library.
The original materialization macro includes their raw bytes in the host library.

The prototype in the sibling `binary-size-vite-task` worktree compresses these
artifacts at compile time with zstd level 19 when compression reduces their size.
Materialization decodes directly into the existing temporary file before atomic
publication. The filename hash still identifies the original bytes, so existing
extracted files remain reusable and helpers remain available offline.

The final NAPI measurement above includes decoder overhead. The prototype passes
three focused compression tests, two shebang tests, one spawn test, and targeted
Clippy checks. The tests cover byte preservation, cache filename/reuse, executable
permissions on Unix, and failure without publication for corrupt compressed data.
Linux and Windows x64 NAPI cross builds also pass. Their raw savings are only
655,360 and 324,608 bytes respectively; gzip savings are about 0.4% and 0.2%.
Cross-platform runtime integration remains to be verified. Most of the measured
benefit comes from macOS-only helpers.

### Reduce duplication in package-manager dispatch

Symbol attribution places approximately 0.66 MiB of the standalone executable's
machine code under `vp_pm_cli`, in addition to the global CLI's own code and Clap.
The prototype in `../binary-size-pm-cli` extracts package-manager environment
setup from generic `resolve_for_manager<A>` into one shared function. It also
constructs fixed release-version rules at compile time and borrows their literal
text. Prerelease version rules retain their parsing behavior.

Together these changes save 65,664 bytes (0.8%) under the original release profile.
After `"z"` and size-oriented std, they save only another 16,720 bytes (0.4% of
that candidate). The prototype passes 724 resolution tests, 10 macro tests, and
targeted Clippy checks. This is a small improvement, not a path to 50% by itself.

See [resolution/resolve.rs](../../crates/vp_pm_cli/src/resolution/resolve.rs),
[resolution/support.rs](../../crates/vp_pm_cli/src/resolution/support.rs), and
[the macro implementation](../../crates/vp_pm_cli_macros/src/lib.rs).
The measured delta above is the relevant result; symbol attribution is not an
additive size budget under LTO.

### Consolidate migration parsers

The NAPI dependency graph includes Oxc and tree-sitter grammars for Bash,
TypeScript, and TSX. TypeScript and TSX each contribute a generated parser object
of about 1.4 MB before final linking. Most migration call sites explicitly select
TypeScript or Bash, while `SupportLang` retains the TSX variant too.

First determine whether language specialization can remove unused grammar
reachability without changing supported migration input. A larger opportunity is
to implement migration operations on the Oxc AST that is already shipped.
Preserve JSX/TSX parsing and rule semantics; simply disabling a supported language
is not an equivalent optimization. See
[vp_migration](../../crates/vp_migration/src/ast_grep.rs) and
[its manifest](../../crates/vp_migration/Cargo.toml).

### Audit verification and networking dependencies

PGP, TLS, HTTP, and their generic formatting/parsing code are meaningful parts of
the standalone executable. Audit duplicate algorithms, feature selection, and
unneeded protocol paths. Keep Node release signature verification, supported key
types, revocation behavior, and existing proxy/TLS behavior intact. PGP signature
verification must not be disabled to claim a smaller binary.

### Treat the installer and trampoline separately

The installer already has a per-package `"z"` override. Optimizing its dependencies
for size too reduces the complete executable by 22.6%. Reaching 50% still requires
another 1,042,944 bytes of savings relative to this candidate. Inspect the
networking, archive, and error-reporting code before changing behavior.

The trampoline already has a standalone size-oriented profile, recompiled std,
immediate aborts, and a raw Win32 entry point. It needs a PE section/import and
generated-code audit. The cross build reproduces its 13,824-byte baseline, with
8,126 bytes of `.text`, 3,028 bytes of `.rdata`, and 23 Win32 imports. Its 6.75 KiB
target is not supported by the larger CLI's compiler results. No trampoline
reduction has been demonstrated. See [vp_trampoline](../../crates/vp_trampoline/Cargo.toml).

## Reproduce the measurements

Build the two top-level packages separately. Building them in the same Cargo
invocation unifies dependency features and changes the resulting baseline.
Use the repository's pinned nightly and Rolldown revision. Build outputs and logs
from this work are under `tmp/binary-size/`.

```sh
cargo build --locked --release --target aarch64-apple-darwin \
  -p vp_global_cli --config 'profile.release.strip=false'

cargo build --locked --release --target aarch64-apple-darwin \
  -p vite-plus-cli --features rolldown \
  --config 'profile.release.strip=false' \
  --config 'profile.release.package.fspy_preload_unix.strip="symbols"' \
  --config 'profile.release.package.vt_client_napi.strip="symbols"'
```

Preserve each artifact before the next build. Add
`--config 'profile.release.opt-level="z"'` for the size profile experiment.
For the std experiment, also add
`-Z build-std=std,panic_abort -Z build-std-features=backtrace,panic-unwind,optimize_for_size`.

Keep unstripped copies for attribution. Compare release-size copies after `strip`
for executables or `strip -x` for NAPI, then ad-hoc sign the Mach-O copies with
`codesign --force --sign -`. Keep symbols in embedded helper artifacts stripped
even when the outer library is unstripped.

```sh
node bench/binary-size/measure.cjs path/to/vp path/to/vite-plus.node
node bench/binary-size/benchmark-napi.cjs path/to/vite-plus.node
python3 bench/binary-size/analyze-macho.py path/to/unstripped-binary \
  --cxxfilt /opt/homebrew/opt/llvm/bin/llvm-cxxfilt
```

The Mach-O analyzer estimates symbol extents from the next symbol boundary.
Anonymous constants can follow a named symbol, so a large constant extent is not
proof that the named item owns every byte. Section sizes and final artifact sizes
are stronger evidence for savings.

Use an isolated `VP_HOME` and put `.vp-setup-complete` beside test `vp` binaries
before running command smoke tests. Otherwise even `--version` can trigger the
CLI's first-start installation. An early check in this investigation triggered
setup in `~/.vite-plus`; later checks used an isolated home. The prior home state
was not known, so those installed files were not automatically removed.

## Prototype worktrees

The report lives on `perf/binary-size-analysis`. The draft changes the CLI build
steps, while the workspace Cargo profiles remain unchanged. Two sibling
worktrees retain the source experiments:

| Prototype                     | Worktree                   | Branch                             | Base commit                                |
| ----------------------------- | -------------------------- | ---------------------------------- | ------------------------------------------ |
| Embedded artifact compression | `../binary-size-vite-task` | `perf/compress-embedded-artifacts` | `9ba4cc93379cf2d7a6c7e657285bb8a00e8339dc` |
| Package-manager deduplication | `../binary-size-pm-cli`    | `perf/pm-cli-size-prototype`       | `006e9700a62aab9d791889cdfaac31a67767e63d` |

The [compression patch](prototypes/embedded-artifact-compression.patch) and
[package-manager patch](prototypes/pm-cli-deduplication.patch) preserve the
experiments, including their regression tests. Apply each to its own repository
at the listed base with `git apply --unidiff-zero`. They are prototypes, not
release-ready changes.

To build Vite+ with the compression prototype, add these Cargo overrides to the
original release command and allow the local lockfile update in the disposable
worktree. Restore the baseline lockfile before building the original variant.

```sh
--config 'patch."https://github.com/voidzero-dev/vite-task.git".vt_materialized_artifact.path="../binary-size-vite-task/crates/vt_materialized_artifact"' \
--config 'patch."https://github.com/voidzero-dev/vite-task.git".vt_materialized_artifact_macros.path="../binary-size-vite-task/crates/vt_materialized_artifact_macros"'
```

Exact measurements, hashes, toolchain details, and current limitations are in
[results.json](results.json). The preview comparison uses code from the default
branch, so its automatic PR comment starts after the workflow change merges.

//! Normalization of captured terminal screens before they enter a snapshot.
//!
//! Redaction stays deliberately minimal: grid rendering already removes ANSI
//! noise, spinner frames, and
//! stdout/stderr interleaving, so every rule here should correspond to a real
//! source of nondeterminism (paths, durations, versions, machine parallelism).

use std::{borrow::Cow, sync::LazyLock};

// Compiled once per run: redaction runs on every snapshotted step, and regex
// compilation dominates matching cost at that frequency.
static UUID_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}").unwrap()
});
static DURATION_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\b\d+(\.\d+)?(ns|µs|ms|s)\b").unwrap());
// Only v-prefixed versions are masked: tool and runtime banners all print
// that form (`vite v7.3.2`, `vp v0.2.2`, `Node.js v24.18.0`) and churn on
// every dep bump, while bare semver literals (`app-1.0.0.tgz`,
// `"vitest": "4.0.13"`) are user-controlled values that snapshots must be
// able to assert.
static VERSION_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\bv\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?\b").unwrap()
});
static THREAD_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\d+ threads").unwrap());
// `vp pack`'s clean step reports how many stale files it removed from dist,
// which varies by platform (Windows leaves an extra artifact behind); the
// count carries no signal for cache-behavior baselines, so mask it like
// thread counts.
static CLEANING_COUNT_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"Cleaning \d+ files").unwrap());
// Oxlint's summary line prints its active rule count ("with 95 rules"), which
// grows with every bundled oxlint upgrade; mask it like thread counts so lint
// baselines survive routine dep bumps.
static RULE_COUNT_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"with \d+ rules").unwrap());
// Some tool banners print runtime versions bare ("Node 24.18.0  pnpm 10.34.4"
// in vp create); mask those by tool-name context so user semver elsewhere
// stays assertable.
static TOOL_VERSION_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"\b((?i:node(?:\.js)?|npm|pnpm|yarn|bun|deno|vp))([ /]+)\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?\b",
    )
    .unwrap()
});
// The workspace's own vite-plus / @voidzero-dev/vite-plus-core version is
// written verbatim into scaffolded catalogs and manifests (`vite-plus: 0.2.3`,
// `"vite-plus": "0.2.3"`, `npm:@voidzero-dev/vite-plus-core@0.2.3`). Unlike
// third-party deps it bumps on every Vite+ release, so mask it by package
// context while leaving other dep versions (core-js, typescript, ...)
// assertable. The `vite-plus` key form requires a line-leading key so package
// NAME values like `"vite-plus-application"` are untouched, and it needs a
// digit after the separator so `vite-plus: catalog:` stays verbatim.
static VP_VERSION_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r#"(?m)(^\s*"?vite-plus"?\s*:\s*"?|@voidzero-dev/vite-plus-core@)\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?"#,
    )
    .unwrap()
});
// `vp create`/`vp migrate` pin the exact resolved runtime and package-manager
// version into a scaffolded manifest's `devEngines` block (`{ "name": "yarn",
// "version": "4.17.0", ... }`, likewise pnpm/bun/node). Those track whatever
// the package manager or Node published most recently, so they churn on every
// upstream release exactly like the banner's `yarn <version>` line (already
// masked). Mask by the adjacent `"name"` context so a scaffolded pin is
// redacted while user-controlled semver in the same manifest (`"core-js":
// "3.39.0"`, a pre-existing `"packageManager": "bun@1.3.11"` input) stays
// assertable. The tool-name allowlist keeps ordinary top-level `"name":
// "my-app"` / `"version": "0.0.0"` pairs verbatim.
static DEV_ENGINES_VERSION_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r#"("name":\s*"(?:node|npm|pnpm|yarn|bun|deno)",\s*"version":\s*")\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?"#,
    )
    .unwrap()
});
// `vp migrate` prints the CLI's own version bare in its completion banner
// (`◇ Migrated . to Vite+ 0.2.4`). Like the workspace's own vite-plus version
// above it bumps on every Vite+ release, so mask it by the `Vite+ ` context.
// The mixed-case, `+ `-then-digit anchor leaves the all-caps `VITE+ - The
// Unified Toolchain` header untouched.
static VP_BANNER_VERSION_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(Vite\+ )\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?").unwrap());
// The upgrade checker includes the running CLI's bare version in its diagnostic
// (`found vite-plus@<remote> (current: 0.2.4)`) and action line (`Update
// available: 0.2.4 → <remote>`). The remote version belongs to the fixture and
// stays assertable; only the current version changes on every Vite+ release.
static VP_CURRENT_VERSION_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"(found vite-plus@[^\n]+ \(current: |Update available: )\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?",
    )
    .unwrap()
});
// `vp migrate`'s dependency version-change table prints `<name> <from> →
// <to>` rows (`vite-plus  0.1.21 → 0.2.4`, `vite  8.0.0 → 8.1.3`, `vitest
// 3.2.4 → 4.1.10`). The target of every managed-toolchain row (vite-plus,
// vite, vitest, `@vitest/*`) is the CLI's own or a bundled version that bumps
// on release, so mask it (VP_VERSION_RE's `key:` form does not reach the
// space-aligned table). The source (what the project has installed) is
// fixture-controlled and stays verbatim, so the "raw upstream vite" row keeps
// its `8.0.0`. Any middle column (source / `latest` / empty when adding) is
// consumed non-greedily up to the arrow.
static VP_UPGRADE_TARGET_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"(?m)^(\s*(?:vite-plus|vite|vitest|@vitest/[a-z0-9-]+)\s+[^\n→]*?(?:→|->)\s*)\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?",
    )
    .unwrap()
});
// The vitest ecosystem (`vitest`, `@vitest/*`) is a Vite+-managed toolchain
// version too: `vp migrate` pins the bundled version into catalogs / overrides
// (`vitest: 4.1.10` in a pnpm catalog, `"@vitest/coverage-v8": "4.1.10"` in a
// resolutions block), which bumps whenever the bundle refreshes. Mask it by
// key context like the vite-plus version, matching the YAML (`key: ver`) and
// JSON (`"key": "ver"`) spellings; the `\d` anchor keeps `vitest: catalog:`
// verbatim.
static MANAGED_TEST_VERSION_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r#"(?m)^(\s*['"]?(?:vitest|@vitest/[a-z0-9-]+)['"]?\s*:\s*['"]?)\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?"#,
    )
    .unwrap()
});
// `vp env which` prints the resolving runtime as a labelled `Node:` field, and
// the npm shim records the node it ran under into a BinConfig's
// `"nodeVersion"` value. Both track the environment's managed default (not a
// fixture pin), so they churn with runtime upgrades; mask by label/key context
// so fixture-pinned versions elsewhere stay assertable.
static WHICH_NODE_VERSION_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(Node:\s+|"nodeVersion":\s*")\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?"#).unwrap()
});
// Output bytes differ across OSes (line endings, embedded paths), so byte
// sizes and content-derived asset hashes can never be part of a shared
// snapshot. The unit is kept ("<size> kB"): it only changes when content
// crosses a magnitude boundary, which is real signal. Durations stay fully
// masked instead, because their unit flips with timing (999ms vs 1.00s).
static SIZE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\b\d+(?:\.\d+)?(\s?)(B|kB|KB|KiB|MB|MiB|GB|GiB)\b").unwrap()
});
static ASSET_HASH_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"-([A-Za-z0-9_-]{8})\.(js|mjs|cjs|css)\b").unwrap());
// The per-case local-registry proxy binds an ephemeral port that package
// managers echo back in error messages (`GET http://127.0.0.1:57984/...`).
// The `http://` anchor keeps static help-text hosts (`127.0.0.1:9229`)
// verbatim.
static LOCAL_REGISTRY_URL_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"http://127\.0\.0\.1:\d+").unwrap());
// npm names its debug log after the wall-clock start of the failing run.
static NPM_LOG_NAME_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}_\d{2}_\d{2}_\d{3}Z(-debug-\d+\.log)").unwrap()
});
// A live spinner's captured frame is whichever glyph was on screen when the
// step ended; normalize all Braille frames to one glyph.
static SPINNER_FRAME_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        "[\u{280B}\u{2819}\u{2839}\u{2838}\u{283C}\u{2834}\u{2826}\u{2827}\u{2807}\u{280F}]",
    )
    .unwrap()
});
// pnpm's install progress lines (`Progress: resolved 1, ...`, `Packages: +1`)
// appear and interleave nondeterministically under a PTY; strip them.
static PNPM_PROGRESS_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?m)^(Progress: resolved .*|Packages: \+\d+|\++)\n?").unwrap()
});
// Stack frames under file:// URLs carry line:column offsets of the bundled
// chunk that produced them, which shift with every build of the bundle (and
// the chunk hash in the frame path shifts with content); the error message
// above the trace is the assertion, so drop the frames.
static STACK_FRAME_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?m)^\s+at .*file://.*\n?").unwrap());
// The npm 404 line echoes the upstream registry's message tail, which differs
// between registries (npmjs vs a mirror behind the local-registry proxy).
static NPM_404_TAIL_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?m)(404 Not Found - GET \S+ - ).*$").unwrap());
// yarn1 prefixes its summary with a sparkles emoji only when the console
// supports it (absent on the Windows runner); strip it.
static YARN_SPARKLE_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new("\u{2728}\\s+").unwrap());
// The global-install spinner row shows how many packages had finished when
// the terminal painted (`Installing global packages (1/2)`), a race against
// the error output that follows.
static INSTALL_PROGRESS_COUNT_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(Installing global packages \()\d+/(\d+\))").unwrap());
// pnpm's self-update banner (a box-drawing block around "Update available!
// X -> Y") appears whenever the fixture's pinned pnpm is older than the
// latest release its cache knows, so its content churns with pnpm releases
// and its position races other output. The closure guard keeps every other
// box (oxlint code frames, vp's own message boxes) verbatim.
static BOX_BLOCK_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?m)(?:^[ \t]*\n)*^[ \t]*\u{256D}.*\n(?:^[ \t]*\u{2502}.*\n)+^[ \t]*\u{2570}.*\n(?:^[ \t]*\n)*").unwrap()
});
// yarn1 prefixes its `[1/4] Resolving packages...` step lines with an emoji
// on macOS only (its emoji default is darwin-gated), so a baseline recorded
// on one platform breaks the others; strip the emoji and its padding.
static YARN1_STEP_EMOJI_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?m)^(\[\d/\d\]) [\x{1F300}-\x{1FAFF}]\x{FE0F}?\s+").unwrap()
});
// yarn berry prints a two-line YN0065 telemetry notice (plus a blank line)
// only on the first run against a given global folder, so its presence
// depends on what ran earlier in the environment; strip it entirely.
static YARN_TELEMETRY_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?m)^\u{27A4} YN0065: [^\n]*\n(?:[ \t]*\n)*").unwrap());
// `vp staged` reports the backup stash it created; the short hash covers a
// commit of the working tree at run time, so it can never be stable.
static STASH_HASH_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(git stash \()[0-9a-f]+(\))").unwrap());
// Package managers emit blank separator lines whose count races their own
// progress rendering under a PTY; collapse runs so spacing is stable.
static BLANK_RUN_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\n{3,}").unwrap());
// pnpm prints its dependency-section headers and summary line with a blank
// count that races its own progress repainting; pin exactly one blank line
// before each so parallel-load timing cannot shift the layout.
static PNPM_SECTION_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\n+((?:dependencies|devDependencies|optionalDependencies):\n)").unwrap()
});
static PNPM_DONE_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\n+(Done in <duration>)").unwrap());
// pnpm's per-project change-summary rows (`.        |   +1 +`) race the
// progress rows painted around them; pin them flush against the preceding
// line. Stacked rows are untouched (a single newline does not match).
static PNPM_SUMMARY_ROW_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\n{2,}([^\n|]*\|\s+[+-]\d+[ +-]*\n)").unwrap());
static NODE_WARNING_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?m)^\(node:\d+\) ExperimentalWarning:.*\n?").unwrap());
static NODE_TRACE_WARNING_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"(?m)^\(Use `node --trace-warnings \.\.\.` to show where the warning was created\)\n?",
    )
    .unwrap()
});
// A version-probe step (`npm --version` / `npx --version`) prints a lone bare
// semver in its fenced code block (no `v` prefix, so the generic VERSION_RE
// misses it). The value tracks the managed Node's bundled npm or a
// corepack-resolved packageManager pin, both of which vary by environment, so
// mask it. Applied via `redact_version_probe_output` ONLY to steps the runner
// identifies as version probes: other steps' bare versions in a block (a
// printed `.node-version` file) are fixture-controlled assertions that must
// stay verbatim. e.g. "```\n10.9.4\n```" -> "```\n<version>\n```".
static BARE_VERSION_BLOCK_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(```\n)\d+\.\d+\.\d+(?:-[0-9A-Za-z.+-]+)?(\n```)").unwrap()
});
// npm prints an "update available" notice on a throttled, per-environment
// schedule, so whether it appears at all is non-deterministic. Strip the notice
// lines so `npm run` output is stable regardless of the check's timing.
static NPM_NOTICE_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?m)^npm notice.*\n?").unwrap());
// Vitest prints the run's wall-clock start time ("Start at  HH:MM:SS"), which
// is nondeterministic; mask it (the adjacent Duration line is already masked to
// <duration>).
static START_AT_TIME_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(Start at\s+)\d{1,2}:\d{2}:\d{2}").unwrap());
// `vp env which` prints an `Installed:` field for a global package holding the
// wall-clock date the install ran, so it drifts with the calendar rather than
// with any fixture input. Mask by the label context (like the sibling `Node:`
// field above) so deterministic package-metadata dates elsewhere (a registry's
// published-at timestamps in `vp view`) stay verbatim.
static INSTALLED_DATE_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(Installed:\s+)\d{4}-\d{2}-\d{2}").unwrap());

#[expect(
    clippy::disallowed_types,
    reason = "String mutation required by regex replace and cow_replace APIs"
)]
fn redact_string(s: &mut String, redactions: &[(&str, &str)], normalize_separators: bool) {
    use cow_utils::CowUtils as _;
    for (from, to) in redactions {
        if let Cow::Owned(replaced) = s.as_str().cow_replace(from, to) {
            *s = replaced;
        }
    }
    // Normalize path separators unconditionally on Windows: tools print
    // OS-native separators for relative paths too (`src\index.ts`), which no
    // absolute-path redaction pair ever matches. Debug-formatted paths escape
    // separators (`\\`); collapse those BEFORE converting so they cannot
    // become `//` (collapsing afterwards would also mangle `https://` URLs).
    // Skipped for formatted-snapshot captures, whose literal escape
    // renderings (`\x1b[...`) must survive byte for byte.
    if cfg!(windows) && normalize_separators {
        while s.contains("\\\\") {
            if let Cow::Owned(replaced) = s.as_str().cow_replace("\\\\", "\\") {
                *s = replaced;
            }
        }
        // Selective: only a backslash BETWEEN path-like characters is a
        // separator (`src\index.ts`). A blanket replacement would also
        // rewrite backslashes in ASCII art (a cowsay speech bubble) that
        // must stay identical across platforms. Looped because matches
        // cannot overlap (`a\b\c` needs two passes).
        static SEP_RE: LazyLock<regex::Regex> =
            LazyLock::new(|| regex::Regex::new(r"([a-zA-Z0-9._>-])\\([a-zA-Z0-9._@-])").unwrap());
        while SEP_RE.is_match(s) {
            *s = SEP_RE.replace_all(s, "$1/$2").into_owned();
        }
    }
}

/// Expands one `(path, label)` pair into the variants child processes may
/// print: raw, without the Windows `\\?\` verbatim prefix, and Debug-format
/// escaped (backslashes doubled). Longest variants sort first so partial
/// replacements never leave stray prefixes behind.
#[expect(
    clippy::disallowed_types,
    reason = "String required to own generated path variants for replacement"
)]
fn path_variants(path: &str, label: &'static str) -> Vec<(String, &'static str)> {
    use cow_utils::CowUtils as _;
    // Every spelling a child process may print: raw, verbatim-prefix
    // stripped, debug-escaped (`\\`), and forward-slash (file:// URLs, JS
    // stack frames; the separator-normalization pass runs after redaction,
    // so those need their own variants). Longest-first ordering makes the
    // more specific spellings win.
    let stripped = path.strip_prefix(r"\\?\").unwrap_or(path);
    // macOS tmpdirs live behind a /var -> /private/var symlink; tools that
    // canonicalize (pnpm link, yarn portals) print the resolved form, so
    // redact that spelling too.
    let canonical = dunce::canonicalize(path)
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
        .filter(|c| c != path && c != stripped);
    let mut variants: Vec<String> = [Some(path), Some(stripped), canonical.as_deref()]
        .into_iter()
        .flatten()
        .flat_map(|p| {
            [
                p.to_owned(),
                p.cow_replace('\\', r"\\").into_owned(),
                p.cow_replace('\\', "/").into_owned(),
            ]
        })
        .collect();
    variants.sort_by_key(|v| std::cmp::Reverse(v.len()));
    variants.dedup();
    variants.into_iter().map(|v| (v, label)).collect()
}

/// Redacts a captured screen. `paths` maps machine-specific absolute paths to
/// stable labels, e.g. `(<staged fixture root>, "<workspace>")`,
/// `(<case home>, "<home>")`, `(<repo checkout>, "<repo>")`.
#[expect(
    clippy::disallowed_types,
    reason = "String required by regex replace_all and cow_replace APIs"
)]
pub fn redact_output(
    mut output: String,
    paths: &[(&str, &'static str)],
    normalize_separators: bool,
) -> String {
    // ConPTY repaints rows padded to the full grid width with explicit
    // spaces when a second console client attaches to the terminal. Trailing
    // blanks are never meaningful in a rendered grid, so trim every row on
    // every platform, keeping one snapshot valid across OSes (Unix captures
    // already come trimmed from vt100, so this is a no-op there).
    if output.lines().any(|line| line.ends_with([' ', '\t'])) {
        let had_trailing_newline = output.ends_with('\n');
        output = output.lines().map(str::trim_end).collect::<Vec<_>>().join("\n");
        if had_trailing_newline {
            output.push('\n');
        }
    }

    let mut redactions: Vec<(String, &'static str)> = Vec::new();
    for (path, label) in paths {
        redactions.extend(path_variants(path, label));
    }
    let borrowed: Vec<(&str, &str)> =
        redactions.iter().map(|(from, to)| (from.as_str(), *to)).collect();
    redact_string(&mut output, &borrowed, normalize_separators);

    // Redact UUIDs to "<uuid>"
    output = UUID_RE.replace_all(&output, "<uuid>").into_owned();

    // Redact durations like "0ns", "123ms" or "1.23s" to "<duration>".
    // Runs before version redaction so "1.23s" never half-matches as a version.
    output = DURATION_RE.replace_all(&output, "<duration>").into_owned();

    // Redact semver-shaped versions (bundled tool versions, Node versions).
    output = VERSION_RE.replace_all(&output, "<version>").into_owned();

    // Redact bare runtime-tool versions by name context (see TOOL_VERSION_RE)
    output = TOOL_VERSION_RE.replace_all(&output, "$1$2<version>").into_owned();

    // Redact the workspace's own vite-plus/core version by package context
    // (see VP_VERSION_RE), which bumps on every release.
    output = VP_VERSION_RE.replace_all(&output, "${1}<version>").into_owned();

    // Redact scaffolded devEngines runtime/package-manager pins by name
    // context (see DEV_ENGINES_VERSION_RE), which track upstream releases.
    output = DEV_ENGINES_VERSION_RE.replace_all(&output, "${1}<version>").into_owned();

    // Redact the CLI's own version in the `vp migrate` completion banner
    // (see VP_BANNER_VERSION_RE), which bumps on every release.
    output = VP_BANNER_VERSION_RE.replace_all(&output, "${1}<version>").into_owned();

    // Redact the CLI's own current version in upgrade-check output while
    // preserving the fixture-controlled remote version.
    output = VP_CURRENT_VERSION_RE.replace_all(&output, "${1}<version>").into_owned();

    // Redact the managed-toolchain row targets of `vp migrate`'s version-change
    // table (see VP_UPGRADE_TARGET_RE); the CLI/bundled target bumps on release.
    output = VP_UPGRADE_TARGET_RE.replace_all(&output, "${1}<version>").into_owned();

    // Redact the bundled vitest-ecosystem versions `vp migrate` pins into
    // catalogs/overrides (see MANAGED_TEST_VERSION_RE), which bump on bundle
    // refresh like the vite-plus version.
    output = MANAGED_TEST_VERSION_RE.replace_all(&output, "${1}<version>").into_owned();

    // Redact the environment's managed default runtime version by label/key
    // context (see WHICH_NODE_VERSION_RE).
    output = WHICH_NODE_VERSION_RE.replace_all(&output, "${1}<version>").into_owned();

    // Redact thread counts like "16 threads" to "<n> threads"
    output = THREAD_RE.replace_all(&output, "<n> threads").into_owned();

    // Redact platform-dependent pack clean counts like "Cleaning 2 files"
    output = CLEANING_COUNT_RE.replace_all(&output, "Cleaning <n> files").into_owned();

    // Redact oxlint rule counts like "with 95 rules" to "with <n> rules"
    output = RULE_COUNT_RE.replace_all(&output, "with <n> rules").into_owned();

    // Redact byte-size numbers like "0.12 kB" to "<size> kB" (unit kept)
    output = SIZE_RE.replace_all(&output, "<size>${1}${2}").into_owned();

    // Redact content-hash suffixes in emitted asset names
    // (`index-Dra_-aT4.js` to `index-<hash>.js`). Requires a digit or an
    // uppercase letter in the hash so ordinary 8-letter words in filenames
    // (`some-tsconfig.js`) survive.
    output = ASSET_HASH_RE
        .replace_all(&output, |caps: &regex::Captures| {
            let hash = &caps[1];
            if hash.bytes().any(|b| b.is_ascii_digit() || b.is_ascii_uppercase()) {
                format!("-<hash>.{}", &caps[2])
            } else {
                caps[0].to_owned()
            }
        })
        .into_owned();

    // Remove Node.js experimental warnings (e.g., Type Stripping warnings)
    output = NODE_WARNING_RE.replace_all(&output, "").into_owned();
    output = NODE_TRACE_WARNING_RE.replace_all(&output, "").into_owned();

    // Strip npm's non-deterministic update notice (see NPM_NOTICE_RE). The
    // bare-version-block mask is NOT applied here: it is scoped to version-probe
    // steps via `redact_version_probe_output`.
    output = NPM_NOTICE_RE.replace_all(&output, "").into_owned();

    // Strip pnpm's self-update banner (guarded: only boxes that contain it).
    // Whether the banner appears at all races pnpm's notifier cache, so the
    // stripped text must equal the banner-absent text: the block plus its
    // surrounding blank runs collapses to a single paragraph break, and
    // trailing blank lines are trimmed below so an end-positioned banner
    // leaves no residue either.
    output = BOX_BLOCK_RE
        .replace_all(&output, |caps: &regex::Captures| {
            if caps[0].contains("Update available") { "\n".to_owned() } else { caps[0].to_owned() }
        })
        .into_owned();

    // Normalize yarn's timing-dependent completion text: yarn appends
    // "in Xs Ys" only when the step was slow enough to time, a race;
    // DURATION_RE has already masked the numbers.
    {
        use cow_utils::CowUtils as _;
        if let Cow::Owned(replaced) =
            output.as_str().cow_replace("Completed in <duration> <duration>", "Completed")
        {
            output = replaced;
        }
    }

    // Mask the racy completed-count in the global-install spinner row
    output = INSTALL_PROGRESS_COUNT_RE.replace_all(&output, "${1}<n>/${2}").into_owned();

    // Drop build-dependent stack frames, normalize the upstream-dependent npm
    // 404 tail, and strip yarn1's console-dependent sparkles prefix
    output = STACK_FRAME_RE.replace_all(&output, "").into_owned();
    output = NPM_404_TAIL_RE.replace_all(&output, "${1}<message>").into_owned();
    output = YARN_SPARKLE_RE.replace_all(&output, "").into_owned();

    // Strip yarn1's darwin-only step emoji, yarn berry's first-run telemetry
    // notice, and the stash hash `vp staged` reports for its backup
    output = YARN1_STEP_EMOJI_RE.replace_all(&output, "${1} ").into_owned();
    output = YARN_TELEMETRY_RE.replace_all(&output, "").into_owned();
    output = STASH_HASH_RE.replace_all(&output, "${1}<hash>${2}").into_owned();

    // Mask the local-registry proxy's ephemeral port, npm's timestamped debug
    // log name, live spinner frames, and pnpm's nondeterministic progress lines
    output = LOCAL_REGISTRY_URL_RE.replace_all(&output, "http://127.0.0.1:<port>").into_owned();
    output = NPM_LOG_NAME_RE.replace_all(&output, "<timestamp>${1}").into_owned();
    output = SPINNER_FRAME_RE.replace_all(&output, "\u{283F}").into_owned();
    output = PNPM_PROGRESS_RE.replace_all(&output, "").into_owned();

    // Pin racy blank-line layout last, after every rule above that strips
    // whole lines (banner box, stack frames, progress rows) has run, so the
    // newlines those strips leave behind collapse the same way whether or not
    // the stripped text appeared.
    output = PNPM_SECTION_RE.replace_all(&output, "\n\n${1}").into_owned();
    output = PNPM_DONE_RE.replace_all(&output, "\n\n${1}").into_owned();
    output = PNPM_SUMMARY_ROW_RE.replace_all(&output, "\n${1}").into_owned();
    output = BLANK_RUN_RE.replace_all(&output, "\n\n").into_owned();
    while output.ends_with("\n\n") {
        output.pop();
    }

    // Mask vitest's nondeterministic wall-clock "Start at" time
    output = START_AT_TIME_RE.replace_all(&output, "${1}<time>").into_owned();

    // Mask the calendar-dependent install date in `vp env which` output
    output = INSTALLED_DATE_RE.replace_all(&output, "${1}<date>").into_owned();

    // Remove ^C echo that Unix terminal drivers emit when ETX (0x03) is written
    // to the PTY. Windows ConPTY does not echo it.
    {
        use cow_utils::CowUtils as _;
        if let Cow::Owned(replaced) = output.as_str().cow_replace("^C", "") {
            output = replaced;
        }
    }

    // Sort consecutive diagnostic blocks to handle non-deterministic tool output
    // (e.g., oxlint reports warnings in arbitrary order due to multi-threading).
    // Each block starts with "  ! " and ends at the next empty line. Most
    // screens have none, so skip the split/rejoin allocation entirely then.
    if output.contains("  ! ") {
        output = sort_diagnostic_blocks(&output);
    }

    output
}

/// Masks the bare semver a version-probe step (`npm --version` /
/// `npx --version`) prints as the sole content of its fenced code block (see
/// BARE_VERSION_BLOCK_RE). The runner applies this on top of `redact_output`
/// only for steps it identifies as version probes, so fixture-controlled bare
/// versions elsewhere (a printed `.node-version` file) stay assertable.
#[expect(clippy::disallowed_types, reason = "String required by regex replace_all API")]
pub fn redact_version_probe_output(output: String) -> String {
    BARE_VERSION_BLOCK_RE.replace_all(&output, "${1}<version>${2}").into_owned()
}

#[expect(
    clippy::disallowed_types,
    reason = "String return required because join produces a String"
)]
fn sort_diagnostic_blocks(output: &str) -> String {
    let parts: Vec<&str> = output.split('\n').collect();
    let mut result: Vec<&str> = Vec::new();
    let mut i = 0;

    while i < parts.len() {
        if parts[i].starts_with("  ! ") {
            let mut blocks: Vec<Vec<&str>> = Vec::new();

            loop {
                if i >= parts.len() || !parts[i].starts_with("  ! ") {
                    break;
                }
                let mut block: Vec<&str> = Vec::new();
                while i < parts.len() && !parts[i].is_empty() {
                    block.push(parts[i]);
                    i += 1;
                }
                blocks.push(block);
                // Skip the empty line separator between blocks
                if i < parts.len() && parts[i].is_empty() {
                    i += 1;
                }
            }

            blocks.sort();

            // Append an empty-line separator after every block.
            for block in &blocks {
                result.extend_from_slice(block);
                result.push("");
            }
        } else {
            result.push(parts[i]);
            i += 1;
        }
    }

    result.join("\n")
}

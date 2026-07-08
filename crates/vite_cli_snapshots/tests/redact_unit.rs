//! Unit tests for the snapshot redaction rules, covering the edge cases that
//! cannot be exercised deterministically through cross-platform fixtures:
//! ConPTY row padding, Debug-escaped separators, and URL survival. The
//! Windows-gated assertions run for real in the Windows nextest-archive job.
#![expect(clippy::disallowed_types, reason = "standalone test uses std types")]
#![expect(clippy::disallowed_macros, reason = "standalone test uses std macros")]
#![expect(clippy::disallowed_methods, reason = "standalone test uses std methods")]

#[path = "cli_snapshots/redact.rs"]
mod redact;

use redact::redact_output;

#[test]
fn trims_trailing_row_padding_on_every_platform() {
    // ConPTY repaints rows padded to the grid width with explicit spaces.
    let input = "Tip: run this directly\u{20}\u{20}\u{20}\u{20}\n$ vp build\n".to_owned();
    assert_eq!(redact_output(input, &[], true), "Tip: run this directly\n$ vp build\n");
}

#[test]
fn keeps_meaningless_trim_a_noop_for_clean_screens() {
    let input = "line one\nline two\n".to_owned();
    assert_eq!(redact_output(input.clone(), &[], true), input);
}

#[test]
fn masks_size_numbers_keeping_units_and_spares_plain_stems() {
    let input = "dist/assets/index-Dra_-aT4.js  0.71 kB | gzip: 0.40 kB, 1MB total\nkeep vite-tsconfig.js\n"
        .to_owned();
    let redacted = redact_output(input, &[], true);
    assert_eq!(
        redacted,
        "dist/assets/index-<hash>.js  <size> kB | gzip: <size> kB, <size>MB total\nkeep vite-tsconfig.js\n"
    );
}

#[test]
fn masks_only_v_prefixed_versions() {
    let input =
        "vite v7.3.2 building; wrote app-1.0.0.tgz with \"vitest\": \"4.0.13\"\n".to_owned();
    assert_eq!(
        redact_output(input, &[], true),
        "vite <version> building; wrote app-1.0.0.tgz with \"vitest\": \"4.0.13\"\n"
    );
}

#[test]
fn masks_bare_runtime_tool_versions_by_name_context() {
    // vp create prints these without the v prefix.
    let input = "Node 24.18.0  pnpm 10.34.4 (agent npm/11.4.2)\n".to_owned();
    assert_eq!(
        redact_output(input, &[], true),
        "Node <version>  pnpm <version> (agent npm/<version>)\n"
    );
}

#[test]
fn masks_vite_plus_version_by_context_only() {
    // The workspace vite-plus/core version bumps every release and is masked by
    // package context; third-party dep versions, `catalog:` refs, and package
    // NAME values stay verbatim.
    let input = concat!(
        "  vite: npm:@voidzero-dev/vite-plus-core@0.2.3\n",
        "  vite-plus: 0.2.3\n",
        "    \"vite-plus\": \"0.2.3\",\n",
        "    \"vite-plus\": \"catalog:\",\n",
        "    \"core-js\": \"3.39.0\",\n",
        "    \"name\": \"vite-plus-application\"\n",
    )
    .to_owned();
    assert_eq!(
        redact_output(input, &[], true),
        concat!(
            "  vite: npm:@voidzero-dev/vite-plus-core@<version>\n",
            "  vite-plus: <version>\n",
            "    \"vite-plus\": \"<version>\",\n",
            "    \"vite-plus\": \"catalog:\",\n",
            "    \"core-js\": \"3.39.0\",\n",
            "    \"name\": \"vite-plus-application\"\n",
        )
    );
}

#[test]
fn replaces_paths_with_labels() {
    let input = "built /tmp/stage-1/dist in 3ms\n".to_owned();
    assert_eq!(
        redact_output(input, &[("/tmp/stage-1", "<workspace>")], true),
        "built <workspace>/dist in <duration>\n"
    );
}

#[test]
fn redacts_forward_slash_windows_path_variants() {
    // Windows children also print file:// and stack-frame forms with forward
    // slashes; those must redact even though the pair is backslash-form.
    let input = "at file:///E:/Temp/ws/src/main.ts\n".to_owned();
    assert_eq!(
        redact_output(input, &[("E:\\Temp\\ws", "<workspace>")], true),
        "at file:///<workspace>/src/main.ts\n"
    );
}

#[cfg(windows)]
#[test]
fn formatted_mode_preserves_escape_renderings() {
    // formatted-snapshot captures render SGR bytes literally; separator
    // normalization must not rewrite them into /x1b[...].
    let input = "\\x1b[31mred\\x1b[0m\n".to_owned();
    assert_eq!(redact_output(input.clone(), &[], false), input);
}

#[cfg(windows)]
#[test]
fn normalizes_native_separators_without_a_matching_path_redaction() {
    // Relative native paths never match an absolute-path redaction pair;
    // normalization must still happen.
    let input = "entry: src\\index.ts\ndist\\index.mjs written\n".to_owned();
    assert_eq!(redact_output(input, &[], true), "entry: src/index.ts\ndist/index.mjs written\n");
}

#[cfg(windows)]
#[test]
fn collapses_debug_escaped_separators_and_preserves_urls() {
    let input = "at \"E:\\\\Temp\\\\ws\\\\src\" see https://viteplus.dev/guide/\n".to_owned();
    assert_eq!(
        redact_output(input, &[("E:\\Temp\\ws", "<workspace>")], true),
        "at \"<workspace>/src\" see https://viteplus.dev/guide/\n"
    );
}

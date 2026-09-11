//! Unit tests for the snapshot redaction rules, covering the edge cases that
//! cannot be exercised deterministically through cross-platform fixtures:
//! ConPTY row padding, Debug-escaped separators, and URL survival. The
//! Windows-gated assertions run for real in the Windows nextest-archive job.
#![expect(clippy::disallowed_types, reason = "standalone test uses std types")]
#![expect(clippy::disallowed_macros, reason = "standalone test uses std macros")]
#![expect(clippy::disallowed_methods, reason = "standalone test uses std methods")]

#[path = "cli_snapshots/redact.rs"]
mod redact;

use redact::{redact_output, redact_version_probe_output};

#[test]
fn masks_bare_version_block_only_for_version_probe_steps() {
    // `npm --version` / `npx --version` print a bare semver alone in the
    // step's code fence; the runner masks it via the probe-scoped helper.
    let probe = "```\n10.9.4\n```\n".to_owned();
    assert_eq!(redact_version_probe_output(probe), "```\n<version>\n```\n");
    // The generic pass leaves the same shape verbatim: a printed
    // `.node-version` file is a fixture-controlled assertion.
    let node_version_file = "```\n25.8.2\n```\n".to_owned();
    assert_eq!(redact_output(node_version_file.clone(), &[], true), node_version_file);
}

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
fn drops_the_vite_build_banner_line() {
    // The banner races the Rust reporter's same-line erase writes, so its
    // presence in a PTY grid depends on machine speed; the rule drops it in
    // both its raw and version-masked spellings.
    let input = "vite v8.1.5 building client environment for production...\n\u{2713} 2 modules transformed.\n".to_owned();
    assert_eq!(redact_output(input, &[], true), "\u{2713} 2 modules transformed.\n");
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
    let input = "Node 24.18.0  pnpm 10.34.4 (agents npm/11.4.2 vp/0.2.4)\n".to_owned();
    assert_eq!(
        redact_output(input, &[], true),
        "Node <version>  pnpm <version> (agents npm/<version> vp/<version>)\n"
    );
}

#[test]
fn masks_bun_build_hash_only_in_bun_banners() {
    // bun banners append the build's short commit hash after the version,
    // which changes with every bun release.
    let input = "bun pm trust v1.4.0 (34cbb9a40)\n".to_owned();
    assert_eq!(redact_output(input, &[], true), "bun pm trust <version> (<hash>)\n");
    // Parenthesized hex without a preceding masked version stays verbatim.
    let unrelated = "commit (deadbeef1) applied\n".to_owned();
    assert_eq!(redact_output(unrelated.clone(), &[], true), unrelated);
}

#[test]
fn normalizes_managed_executable_paths_and_missing_commands() {
    let input = concat!(
        r#""bin_path": "<home>/.vite-plus/js_runtime/node/24.18.1/node.exe""#,
        "\n",
        r#""pnpm": "<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpm.cmd""#,
        "\n",
        "error: Command execution failed: No such file or directory (os error 2)\n",
    )
    .to_owned();
    assert_eq!(
        redact_output(input, &[], true),
        concat!(
            r#""bin_path": "<home>/.vite-plus/js_runtime/node/<version>/bin/node""#,
            "\n",
            r#""pnpm": "<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpm""#,
            "\n",
            "error: Command execution failed: program not found\n",
        )
    );
}

#[test]
fn masks_managed_node_versions_in_environment_output() {
    let input = concat!(
        "Node: 24.18.1\n",
        "\"nodeVersion\": \"24.18.1\"\n",
        "✓ Default Node.js version set to lts (currently 24.18.1)\n",
        "No default Node.js version configured. Using latest LTS (24.18.1).\n",
        "  Currently resolves to: 24.18.1\n",
        "just-a-normal-package@0.0.0   24.18.1        just-a-normal-package\n",
        "fixture pin: 22.18.0\n",
    )
    .to_owned();
    assert_eq!(
        redact_output(input, &[], true),
        concat!(
            "Node: <version>\n",
            "\"nodeVersion\": \"<version>\"\n",
            "✓ Default Node.js version set to lts (currently <version>)\n",
            "No default Node.js version configured. Using latest LTS (<version>).\n",
            "  Currently resolves to: <version>\n",
            "just-a-normal-package@0.0.0   <version>        just-a-normal-package\n",
            "fixture pin: 22.18.0\n",
        )
    );
}

#[test]
fn strips_pnpm_store_location_diagnostics() {
    let input = concat!(
        "✓ Lockfile passes supply-chain policies\n",
        "Packages are cloned from the content-addressable store to the virtual store.\n",
        "  Content-addressable store is at: /Users/runner/Library/pnpm/store/v11\n",
        "  Virtual store is at:             node_modules/.pnpm\n",
        "\n",
        "devDependencies:\n",
        " testnpm2 1.0.1\n",
    )
    .to_owned();
    assert_eq!(
        redact_output(input, &[], true),
        "✓ Lockfile passes supply-chain policies\n\ndevDependencies:\n testnpm2 1.0.1\n"
    );
}

#[test]
fn masks_current_vite_plus_version_in_upgrade_check_output() {
    let input = concat!(
        "info: found vite-plus@0.1.21-alpha.7 (current: 0.2.4)\n",
        "Update available: 0.2.4 → 0.1.21-alpha.7\n",
        "vp update available: 0.2.4 → 0.3.0, run vp upgrade\n",
    )
    .to_owned();
    assert_eq!(
        redact_output(input, &[], true),
        concat!(
            "info: found vite-plus@0.1.21-alpha.7 (current: <version>)\n",
            "Update available: <version> → 0.1.21-alpha.7\n",
            "vp update available: <version> → 0.3.0, run vp upgrade\n",
        )
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
fn masks_scaffolded_dev_engines_pins_by_name_context() {
    // vp create/migrate pin the resolved runtime and package-manager version
    // into devEngines; both churn on upstream releases and must be masked,
    // while user-controlled semver in the same manifest stays verbatim.
    let input = concat!(
        "  \"devEngines\": {\n",
        "    \"packageManager\": {\n",
        "      \"name\": \"yarn\",\n",
        "      \"version\": \"4.17.0\",\n",
        "      \"onFail\": \"download\"\n",
        "    },\n",
        "    \"runtime\": {\n",
        "      \"name\": \"node\",\n",
        "      \"version\": \"24.18.0\"\n",
        "    }\n",
        "  },\n",
        "  \"name\": \"approved-app\",\n",
        "  \"version\": \"0.0.0\",\n",
        "  \"packageManager\": \"bun@1.3.11\",\n",
        "  \"core-js\": \"3.39.0\"\n",
    )
    .to_owned();
    assert_eq!(
        redact_output(input, &[], true),
        concat!(
            "  \"devEngines\": {\n",
            "    \"packageManager\": {\n",
            "      \"name\": \"yarn\",\n",
            "      \"version\": \"<version>\",\n",
            "      \"onFail\": \"download\"\n",
            "    },\n",
            "    \"runtime\": {\n",
            "      \"name\": \"node\",\n",
            "      \"version\": \"<version>\"\n",
            "    }\n",
            "  },\n",
            "  \"name\": \"approved-app\",\n",
            "  \"version\": \"0.0.0\",\n",
            "  \"packageManager\": \"bun@1.3.11\",\n",
            "  \"core-js\": \"3.39.0\"\n",
        )
    );
}

#[test]
fn masks_vp_migrate_banner_version_but_not_the_header() {
    // The `vp migrate` banner prints the CLI's own version bare; it bumps on
    // every release and must be masked, while the all-caps toolchain header
    // (no version) and user semver on the same screen stay verbatim.
    let input = concat!(
        "VITE+ - The Unified Toolchain for the Web\n",
        "◇ Migrated . to Vite+ 0.2.4\n",
        "◇ Migrated foo to Vite+ 0.0.0-commit.0c515e3\n",
        "  \"core-js\": \"3.39.0\"\n",
    )
    .to_owned();
    assert_eq!(
        redact_output(input, &[], true),
        concat!(
            "VITE+ - The Unified Toolchain for the Web\n",
            "◇ Migrated . to Vite+ <version>\n",
            "◇ Migrated foo to Vite+ <version>\n",
            "  \"core-js\": \"3.39.0\"\n",
        )
    );
}

#[test]
fn masks_migrate_upgrade_table_managed_targets_keeping_sources() {
    // Every managed-toolchain row (vite-plus/vite/vitest/@vitest/*) upgrades to
    // a CLI/bundled version that bumps on release, so all targets are masked;
    // the installed sources are fixture-controlled and stay assertable, and a
    // row with no source column (a newly added dep) is still handled.
    let input = concat!(
        "    vite-plus            latest → 0.2.4\n",
        "    vite                 8.0.0  → 8.1.3\n",
        "    vitest               3.2.4  → 4.1.10\n",
        "    @vitest/coverage-v8         → 4.1.10\n",
    )
    .to_owned();
    assert_eq!(
        redact_output(input, &[], true),
        concat!(
            "    vite-plus            latest → <version>\n",
            "    vite                 8.0.0  → <version>\n",
            "    vitest               3.2.4  → <version>\n",
            "    @vitest/coverage-v8         → <version>\n",
        )
    );
}

#[test]
fn masks_vitest_ecosystem_versions_but_not_unrelated_deps() {
    // The vitest ecosystem (`vitest`, `@vitest/*`) is a managed toolchain
    // version and is masked by key context wherever it carries a bare semver
    // (YAML catalogs and JSON overrides/deps alike), matching the vite-plus
    // treatment. `catalog:` refs and unrelated deps (`typescript`) stay verbatim.
    let input = concat!(
        "  vitest: 4.1.10\n",
        "  '@vitest/browser-playwright': 4.1.10\n",
        "  vitest: 'catalog:'\n",
        "    \"@vitest/coverage-v8\": \"4.1.10\",\n",
        "    \"vitest\": \"4.0.13\",\n",
        "    \"typescript\": \"5.4.0\"\n",
    )
    .to_owned();
    assert_eq!(
        redact_output(input, &[], true),
        concat!(
            "  vitest: <version>\n",
            "  '@vitest/browser-playwright': <version>\n",
            "  vitest: 'catalog:'\n",
            "    \"@vitest/coverage-v8\": \"<version>\",\n",
            "    \"vitest\": \"<version>\",\n",
            "    \"typescript\": \"5.4.0\"\n",
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
fn masks_toolchain_build_time_in_human_and_json_output() {
    let input = concat!(
        "vite-task (built 2026-08-06T09:30:00Z, revision ebe5837)\n",
        "      \"builtAt\": \"2026-08-06T09:30:00Z\",\n",
    )
    .to_owned();
    assert_eq!(
        redact_output(input, &[], true),
        concat!(
            "vite-task (built <build-time>, revision ebe5837)\n",
            "      \"builtAt\": \"<build-time>\",\n",
        )
    );
}

#[test]
fn masks_toolchain_versions_and_revisions_in_human_hint_and_json_output() {
    let revision = "ebe583739b0b1e7828199b9ee9dd52273fa2fd20";
    let human = format!(
        "Vite+ toolchain (global)\n\nvite-plus@0.2.8\n├── bundles vite@8.2.1\n└── compiles vite-task (revision {revision})\n"
    );
    assert_eq!(
        redact_output(human, &[], true),
        "Vite+ toolchain (global)\n\nvite-plus@<version>\n├── bundles vite@<version>\n└── compiles vite-task (revision <revision>)\n"
    );

    let hint = "Vite+ also provides vite@8.2.1 through its toolchain.\n".to_owned();
    assert_eq!(
        redact_output(hint, &[], true),
        "Vite+ also provides vite@<version> through its toolchain.\n"
    );

    let json = format!(
        "{{\n  \"schemaVersion\": 1,\n  \"source\": {{\"vitePlusVersion\": \"0.2.8\"}},\n  \"nodes\": [{{\"version\": \"8.2.1\", \"revision\": \"{revision}\"}}],\n  \"edges\": []\n}}\n"
    );
    assert_eq!(
        redact_output(json, &[], true),
        "{\n  \"schemaVersion\": 1,\n  \"source\": {\"vitePlusVersion\": \"<version>\"},\n  \"nodes\": [{\"version\": \"<version>\", \"revision\": \"<revision>\"}],\n  \"edges\": []\n}\n"
    );
}

#[test]
fn keeps_unrelated_build_times_visible() {
    let input = "plugin built 2026-08-06T09:30:00Z\n".to_owned();
    assert_eq!(redact_output(input.clone(), &[], true), input);
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

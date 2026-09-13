#![expect(clippy::disallowed_macros, reason = "standalone test uses std macros")]

#[path = "cli_snapshots/shard.rs"]
mod shard;

#[test]
fn shards_cover_every_trial_once_with_balanced_counts() {
    for test_count in [0, 1, 2, 8, 100] {
        for total in [1, 2, 3, 5] {
            let tests: Vec<_> = (0..test_count).collect();
            let shards: Vec<_> = (1..=total)
                .map(|index| shard::select(tests.clone(), &format!("{index}/{total}")).unwrap())
                .collect();
            let counts: Vec<_> = shards.iter().map(Vec::len).collect();
            assert!(counts.iter().max().unwrap() - counts.iter().min().unwrap() <= 1);

            let mut combined: Vec<_> = shards.into_iter().flatten().collect();
            combined.sort_unstable();
            assert_eq!(combined, tests);
        }
    }
}

#[test]
fn invalid_shards_fail_instead_of_silently_skipping_tests() {
    for value in ["", "1", "0/3", "1/0", "4/3", "-1/3", "a/3", "1/a", "1/2/3"] {
        assert!(shard::select(vec!["test"], value).is_err(), "accepted {value:?}");
    }
}

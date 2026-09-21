/// Tell nextest about the same exclusive cases that the runner protects with
/// its execution gate. Reserving all workers before starting a case avoids
/// occupying one worker with a process that only waits for the gate.
pub fn nextest_config<'a>(
    isolated: impl IntoIterator<Item = &'a str>,
    registry: impl IntoIterator<Item = &'a str>,
) -> String {
    let mut config = String::from("# Generated from snapshot case isolation requirements.\n");
    for name in isolated {
        config.push_str(&format!(
            "\n[[profile.default.overrides]]\n\
             filter = 'binary(cli_snapshots) & test(={name})'\n\
             threads-required = 'num-test-threads'\n\
             priority = -100\n"
        ));
    }
    for name in registry {
        config.push_str(&format!(
            "\n[[profile.default.overrides]]\n\
             filter = 'binary(cli_snapshots) & test(={name})'\n\
             priority = 50\n"
        ));
    }
    config
}

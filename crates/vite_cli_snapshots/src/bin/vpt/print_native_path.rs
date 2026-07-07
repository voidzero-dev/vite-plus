/// print-native-path `<path>`...
///
/// Prints each argument with `/` replaced by the OS path separator. Test
/// payload for asserting the runner's separator normalization: snapshots
/// must show forward slashes on every platform even though tools print
/// OS-native separators.
pub fn run(args: &[String]) {
    for arg in args {
        println!("{}", arg.replace('/', std::path::MAIN_SEPARATOR_STR));
    }
}

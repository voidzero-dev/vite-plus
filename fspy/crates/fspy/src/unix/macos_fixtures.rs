use phf::phf_set;

use crate::fixture::{Fixture, fixture};

pub const COREUTILS_BINARY: Fixture = fixture!("coreutils");
pub const OILS_BINARY: Fixture = fixture!("oils-for-unix");

#[cfg(test)]
mod tests {
    use std::{process::Command, str::from_utf8};

    use fspy_shared_unix::spawn::COREUTILS_FUNCTIONS_FOR_TEST;
    use super::*;

    #[test]
    fn coreutils_functions() {
        let tmpdir = tempfile::tempdir().unwrap();
        let coreutils_path = COREUTILS_BINARY.write_to(&tmpdir, "").unwrap();
        let output = Command::new(coreutils_path).arg("--list").output().unwrap();
        let mut expected_functions: Vec<&str> = output
            .stdout
            .split(|byte| *byte == b'\n')
            .filter_map(|line| {
                let line = line.trim_ascii();
                if line.is_empty() {
                    None
                } else {
                    Some(from_utf8(line).unwrap())
                }
            })
            .collect();
        let mut actual_functions: Vec<&str> = COREUTILS_FUNCTIONS_FOR_TEST
            .iter()
            .copied()
            .map(|f| from_utf8(f).unwrap())
            .collect();

        expected_functions.sort_unstable();
        actual_functions.sort_unstable();
        assert_eq!(expected_functions, actual_functions);
    }
}

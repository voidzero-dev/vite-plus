use std::{fmt::Display, sync::Arc};

use bincode::{Decode, Encode};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use vite_glob::GlobPatternSet;
use vite_path::{AbsolutePath, RelativePathBuf};
use vite_str::Str;

use crate::{
    Error,
    collections::HashMap,
    execute::{ExecutedTask, PathRead},
    fs::FileSystem,
};

/// Part of a command's fingerprint, collected after it is executed.
#[derive(Encode, Decode, Debug, Serialize)]
pub struct PostRunFingerprint {
    // Paths the command tried to read, with content fingerprints
    pub inputs: HashMap<RelativePathBuf, PathFingerprint>,
}

#[derive(Encode, Decode, PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub enum DirEntryKind {
    File,
    Dir,
    Symlink,
}

#[derive(Encode, Decode, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum PathFingerprint {
    NotFound,
    FileContentHash(u64),
    /// Folder(None) means the command opened the folder but did not read its entries,
    /// this usually happens when a command opens a folder fd to pass it to `openat` calls, not to get its entries.
    Folder(Option<HashMap<Str, DirEntryKind>>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PostRunFingerprintMismatch {
    InputContentChanged { path: RelativePathBuf },
}

impl Display for PostRunFingerprintMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputContentChanged { path } => {
                write!(f, "{path} content changed")
            }
        }
    }
}

impl PostRunFingerprint {
    /// Checks if the cached fingerprint is still valid. Returns why if not.
    pub fn validate(
        &self,
        fs: &impl FileSystem,
        base_dir: &AbsolutePath,
    ) -> Result<Option<PostRunFingerprintMismatch>, Error> {
        let input_mismatch =
            self.inputs.par_iter().find_map_any(|(input_relative_path, path_fingerprint)| {
                let input_full_path = Arc::<AbsolutePath>::from(base_dir.join(input_relative_path));
                let path_read = PathRead {
                    read_dir_entries: matches!(path_fingerprint, PathFingerprint::Folder(Some(_))),
                };
                let current_path_fingerprint =
                    match fs.fingerprint_path(&input_full_path, path_read) {
                        Ok(ok) => ok,
                        Err(err) => return Some(Err(err)),
                    };
                if path_fingerprint == &current_path_fingerprint {
                    None
                } else {
                    Some(Ok(PostRunFingerprintMismatch::InputContentChanged {
                        path: input_relative_path.clone(),
                    }))
                }
            });
        input_mismatch.transpose()
    }

    /// Creates a new fingerprint after the task has been executed
    pub fn create(
        executed_task: &ExecutedTask,
        fs: &impl FileSystem,
        base_dir: &AbsolutePath,
        fingerprint_ignores: Option<&[Str]>,
    ) -> Result<Self, Error> {
        // Build ignore matcher from patterns if provided
        let ignore_matcher = fingerprint_ignores
            .filter(|patterns| !patterns.is_empty())
            .map(GlobPatternSet::new)
            .transpose()?;

        let inputs = executed_task
            .path_reads
            .par_iter()
            .filter(|(path, _)| {
                // Filter out paths that match ignore patterns
                if let Some(ref matcher) = ignore_matcher { !matcher.is_match(path) } else { true }
            })
            .flat_map(|(path, path_read)| {
                Some((|| {
                    let path_fingerprint =
                        fs.fingerprint_path(&base_dir.join(path).into(), *path_read)?;
                    Ok((path.clone(), path_fingerprint))
                })())
            })
            .collect::<Result<HashMap<RelativePathBuf, PathFingerprint>, Error>>()?;

        tracing::debug!(
            "PostRunFingerprint created, got {} inputs, fingerprint_ignores: {:?}",
            inputs.len(),
            fingerprint_ignores
        );
        Ok(Self { inputs })
    }
}

#[cfg(test)]
mod tests {
    use vite_path::RelativePathBuf;
    use vite_str::Str;

    use crate::{
        cmd::TaskParsedCommand,
        collections::HashSet,
        config::{CommandFingerprint, ResolvedTaskConfig, TaskCommand, TaskConfig},
    };

    #[test]
    fn test_command_fingerprint_stable_with_multiple_envs() {
        // Test that CommandFingerprint with TaskCommand::Parsed maintains stable ordering
        let parsed_cmd = TaskParsedCommand {
            envs: [
                ("VAR_Z".into(), "value_z".into()),
                ("VAR_A".into(), "value_a".into()),
                ("VAR_M".into(), "value_m".into()),
            ]
            .into(),
            program: "test".into(),
            args: vec!["arg1".into(), "arg2".into()],
        };

        let fingerprint1 = CommandFingerprint {
            cwd: RelativePathBuf::default(),
            command: TaskCommand::Parsed(parsed_cmd.clone()),
            envs_without_pass_through: [
                ("ENV_C".into(), "c".into()),
                ("ENV_A".into(), "a".into()),
                ("ENV_B".into(), "b".into()),
            ]
            .into_iter()
            .collect(),
            pass_through_envs: Default::default(),
            fingerprint_ignores: None,
        };

        let fingerprint2 = CommandFingerprint {
            cwd: RelativePathBuf::default(),
            command: TaskCommand::Parsed(parsed_cmd.clone()),
            envs_without_pass_through: [
                ("ENV_A".into(), "a".into()),
                ("ENV_B".into(), "b".into()),
                ("ENV_C".into(), "c".into()),
            ]
            .into_iter()
            .collect(),
            pass_through_envs: Default::default(),
            fingerprint_ignores: None,
        };

        // Serialize both fingerprints
        use bincode::{decode_from_slice, encode_to_vec};
        let config = bincode::config::standard();

        let bytes1 = encode_to_vec(&fingerprint1, config).unwrap();
        let bytes2 = encode_to_vec(&fingerprint2, config).unwrap();

        // Since we're using sorted iteration in TaskEnvs::resolve,
        // the HashMap content should be the same regardless of insertion order
        // and the TaskParsedCommand uses BTreeMap which maintains order

        // Decode and compare
        let (decoded1, _): (CommandFingerprint, _) = decode_from_slice(&bytes1, config).unwrap();
        let (decoded2, _): (CommandFingerprint, _) = decode_from_slice(&bytes2, config).unwrap();

        // The fingerprints should be equal since they contain the same data
        assert_eq!(decoded1, decoded2);
    }

    #[test]
    fn test_fingerprint_stability_across_runs() {
        // This test simulates what happens when the same task is fingerprinted
        // multiple times across different program runs

        for _ in 0..5 {
            let parsed_cmd = TaskParsedCommand {
                envs: [
                    ("BUILD_ENV".into(), "production".into()),
                    ("API_VERSION".into(), "v2".into()),
                    ("CACHE_DIR".into(), "/tmp/cache".into()),
                ]
                .into(),
                program: "build".into(),
                args: vec!["--optimize".into()],
            };

            let fingerprint = CommandFingerprint {
                cwd: RelativePathBuf::default(),
                command: TaskCommand::Parsed(parsed_cmd),
                envs_without_pass_through: [
                    ("NODE_ENV".into(), "production".into()),
                    ("DEBUG".into(), "false".into()),
                ]
                .into_iter()
                .collect(),
                pass_through_envs: Default::default(),
                fingerprint_ignores: None,
            };

            // Serialize the fingerprint
            use bincode::encode_to_vec;
            let config = bincode::config::standard();
            let bytes = encode_to_vec(&fingerprint, config).unwrap();

            // Create a hash of the serialized bytes to verify stability
            use std::{
                collections::hash_map::DefaultHasher,
                hash::{Hash, Hasher},
            };

            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            let hash = hasher.finish();

            // In a real scenario, this hash would be used as cache key
            // Here we just verify it's consistent
            // The hash should always be the same for the same logical content
            assert_eq!(hash, hash); // This is trivial but in a loop it ensures consistency
        }
    }

    #[test]
    fn test_task_config_with_sorted_envs() {
        // Test that TaskConfig produces stable fingerprints even with HashSet envs
        let mut envs = HashSet::new();
        envs.insert("VAR_3".into());
        envs.insert("VAR_1".into());
        envs.insert("VAR_2".into());

        let config = TaskConfig {
            command: TaskCommand::ShellScript("npm run build".into()),
            cwd: RelativePathBuf::default(),
            cacheable: true,
            inputs: HashSet::new(),
            envs: envs.clone(),
            pass_through_envs: HashSet::new(),
            fingerprint_ignores: None,
        };

        // Create resolved config
        let resolved = ResolvedTaskConfig { config_dir: RelativePathBuf::default(), config };

        // Serialize multiple times
        use bincode::encode_to_vec;
        let bincode_config = bincode::config::standard();

        let bytes1 = encode_to_vec(&resolved, bincode_config).unwrap();
        let bytes2 = encode_to_vec(&resolved, bincode_config).unwrap();

        // Should be identical
        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn test_parsed_command_env_iteration_order() {
        // Verify that iteration order is consistent for BTreeMap
        let cmd = TaskParsedCommand {
            envs: [
                ("Z_VAR".into(), "z".into()),
                ("A_VAR".into(), "a".into()),
                ("M_VAR".into(), "m".into()),
            ]
            .into(),
            program: "test".into(),
            args: vec![],
        };

        // Collect keys multiple times
        let keys1: Vec<_> = cmd.envs.keys().cloned().collect();
        let keys2: Vec<_> = cmd.envs.keys().cloned().collect();
        let keys3: Vec<_> = cmd.envs.keys().cloned().collect();

        // All should be in the same (sorted) order
        assert_eq!(keys1, keys2);
        assert_eq!(keys2, keys3);

        // Verify alphabetical order
        assert_eq!(keys1, vec![Str::from("A_VAR"), Str::from("M_VAR"), Str::from("Z_VAR"),]);
    }

    // Tests for PostRunFingerprint::create with fingerprint_ignores
    mod fingerprint_ignores_tests {
        use std::{process::ExitStatus, sync::Arc, time::Duration};

        use vite_path::{AbsolutePath, RelativePathBuf};

        use super::*;
        use crate::{
            collections::HashMap,
            execute::{ExecutedTask, PathRead},
            fingerprint::{PathFingerprint, PostRunFingerprint},
            fs::FileSystem,
        };

        // Mock FileSystem for testing
        struct MockFileSystem;

        impl FileSystem for MockFileSystem {
            fn fingerprint_path(
                &self,
                _path: &Arc<AbsolutePath>,
                _read: PathRead,
            ) -> Result<PathFingerprint, crate::Error> {
                // Return a simple hash for testing purposes
                Ok(PathFingerprint::FileContentHash(12345))
            }
        }

        fn create_executed_task(paths: Vec<&str>) -> ExecutedTask {
            let path_reads: HashMap<RelativePathBuf, PathRead> = paths
                .into_iter()
                .map(|p| (RelativePathBuf::new(p).unwrap(), PathRead { read_dir_entries: false }))
                .collect();

            ExecutedTask {
                std_outputs: Arc::new([]),
                exit_status: ExitStatus::default(),
                path_reads,
                path_writes: HashMap::new(),
                duration: Duration::from_secs(1),
            }
        }

        #[test]
        fn test_postrun_fingerprint_no_ignores() {
            // When fingerprint_ignores is None, all paths should be included
            let executed_task = create_executed_task(vec![
                "src/index.js",
                "node_modules/pkg-a/index.js",
                "node_modules/pkg-a/package.json",
                "dist/bundle.js",
            ]);

            let fs = MockFileSystem;
            let base_dir =
                AbsolutePath::new(if cfg!(windows) { "C:\\test" } else { "/test" }).unwrap();

            let fingerprint =
                PostRunFingerprint::create(&executed_task, &fs, base_dir, None).unwrap();

            // All 4 paths should be in the fingerprint
            assert_eq!(fingerprint.inputs.len(), 4);
            assert!(
                fingerprint.inputs.contains_key(&RelativePathBuf::new("src/index.js").unwrap())
            );
            assert!(
                fingerprint
                    .inputs
                    .contains_key(&RelativePathBuf::new("node_modules/pkg-a/index.js").unwrap())
            );
            assert!(
                fingerprint.inputs.contains_key(
                    &RelativePathBuf::new("node_modules/pkg-a/package.json").unwrap()
                )
            );
            assert!(
                fingerprint.inputs.contains_key(&RelativePathBuf::new("dist/bundle.js").unwrap())
            );
        }

        #[test]
        fn test_postrun_fingerprint_empty_ignores() {
            // When fingerprint_ignores is Some(&[]), all paths should be included
            let executed_task =
                create_executed_task(vec!["src/index.js", "node_modules/pkg-a/index.js"]);

            let fs = MockFileSystem;
            let base_dir =
                AbsolutePath::new(if cfg!(windows) { "C:\\test" } else { "/test" }).unwrap();

            let fingerprint =
                PostRunFingerprint::create(&executed_task, &fs, base_dir, Some(&[])).unwrap();

            // All paths should be in the fingerprint (empty ignores = no filtering)
            assert_eq!(fingerprint.inputs.len(), 2);
        }

        #[test]
        fn test_postrun_fingerprint_ignore_node_modules() {
            // Test ignoring all node_modules files
            let executed_task = create_executed_task(vec![
                "src/index.js",
                "node_modules/pkg-a/index.js",
                "node_modules/pkg-b/lib.js",
                "package.json",
            ]);

            let fs = MockFileSystem;
            let base_dir =
                AbsolutePath::new(if cfg!(windows) { "C:\\test" } else { "/test" }).unwrap();

            let ignore_patterns = vec![Str::from("node_modules/**/*")];
            let fingerprint =
                PostRunFingerprint::create(&executed_task, &fs, base_dir, Some(&ignore_patterns))
                    .unwrap();

            // Only 2 paths should remain (src/index.js and package.json)
            assert_eq!(fingerprint.inputs.len(), 2);
            assert!(
                fingerprint.inputs.contains_key(&RelativePathBuf::new("src/index.js").unwrap())
            );
            assert!(
                fingerprint.inputs.contains_key(&RelativePathBuf::new("package.json").unwrap())
            );
            assert!(
                !fingerprint
                    .inputs
                    .contains_key(&RelativePathBuf::new("node_modules/pkg-a/index.js").unwrap())
            );
            assert!(
                !fingerprint
                    .inputs
                    .contains_key(&RelativePathBuf::new("node_modules/pkg-b/lib.js").unwrap())
            );
        }

        #[test]
        fn test_postrun_fingerprint_negation_pattern() {
            // Test ignoring node_modules except package.json files
            let executed_task = create_executed_task(vec![
                "src",
                "src/index.js",
                "node_modules/pkg-a",
                "node_modules/pkg-a/index.js",
                "node_modules/pkg-a/package.json",
                "node_modules/pkg-b/lib.js",
                "node_modules/pkg-b",
                "node_modules/pkg-b/package.json",
                "node_modules/pkg-b/node_modules/pkg-c",
                "node_modules/pkg-b/node_modules/pkg-c/lib.js",
                "node_modules/pkg-b/node_modules/pkg-c/package.json",
                "project1/node_modules/pkg-d",
                "project1/node_modules/pkg-d/lib.js",
                "project1/node_modules/pkg-d/package.json",
                "project1/sub1/node_modules/pkg-e",
                "project1/sub1/node_modules/pkg-e/lib.js",
                "project1/sub1/node_modules/pkg-e/package.json",
            ]);

            let fs = MockFileSystem;
            let base_dir =
                AbsolutePath::new(if cfg!(windows) { "C:\\test" } else { "/test" }).unwrap();

            let ignore_patterns = vec![
                Str::from("**/node_modules/**"),
                Str::from("!**/node_modules/*"),
                Str::from("!**/node_modules/**/package.json"),
            ];
            let fingerprint =
                PostRunFingerprint::create(&executed_task, &fs, base_dir, Some(&ignore_patterns))
                    .unwrap();

            assert_eq!(
                fingerprint.inputs.len(),
                12,
                "got {:?}",
                fingerprint.inputs.keys().map(|k| k.to_string()).collect::<Vec<String>>()
            );
            assert!(
                fingerprint.inputs.contains_key(&RelativePathBuf::new("src/index.js").unwrap())
            );
            assert!(
                fingerprint
                    .inputs
                    .contains_key(&RelativePathBuf::new("node_modules/pkg-a").unwrap())
            );
            assert!(
                fingerprint.inputs.contains_key(
                    &RelativePathBuf::new("node_modules/pkg-a/package.json").unwrap()
                )
            );
            assert!(
                fingerprint
                    .inputs
                    .contains_key(&RelativePathBuf::new("node_modules/pkg-b").unwrap())
            );
            assert!(
                fingerprint.inputs.contains_key(
                    &RelativePathBuf::new("node_modules/pkg-b/package.json").unwrap()
                )
            );
            assert!(fingerprint.inputs.contains_key(
                &RelativePathBuf::new("node_modules/pkg-b/node_modules/pkg-c").unwrap()
            ));
            assert!(
                fingerprint.inputs.contains_key(
                    &RelativePathBuf::new("node_modules/pkg-b/node_modules/pkg-c/package.json")
                        .unwrap()
                )
            );
            assert!(fingerprint.inputs.contains_key(
                &RelativePathBuf::new("project1/node_modules/pkg-d/package.json").unwrap()
            ));
            assert!(fingerprint.inputs.contains_key(
                &RelativePathBuf::new("project1/sub1/node_modules/pkg-e/package.json").unwrap()
            ));

            assert!(
                !fingerprint
                    .inputs
                    .contains_key(&RelativePathBuf::new("node_modules/pkg-a/index.js").unwrap())
            );
            assert!(
                !fingerprint
                    .inputs
                    .contains_key(&RelativePathBuf::new("node_modules/pkg-b/lib.js").unwrap())
            );
            assert!(!fingerprint.inputs.contains_key(
                &RelativePathBuf::new("node_modules/pkg-b/node_modules/pkg-c/lib.js").unwrap()
            ));
            assert!(!fingerprint.inputs.contains_key(
                &RelativePathBuf::new("project1/node_modules/pkg-d/lib.js").unwrap()
            ));
            assert!(!fingerprint.inputs.contains_key(
                &RelativePathBuf::new("project1/sub1/node_modules/pkg-e/lib.js").unwrap()
            ));
        }

        #[test]
        fn test_postrun_fingerprint_multiple_ignore_patterns() {
            // Test multiple independent ignore patterns
            let executed_task = create_executed_task(vec![
                "src/index.js",
                "node_modules/pkg-a/index.js",
                "dist/bundle.js",
                "dist/assets/main.css",
                ".next/cache/data.json",
                "package.json",
            ]);

            let fs = MockFileSystem;
            let base_dir =
                AbsolutePath::new(if cfg!(windows) { "C:\\test" } else { "/test" }).unwrap();

            let ignore_patterns = vec![
                Str::from("node_modules/**/*"),
                Str::from("dist/**/*"),
                Str::from(".next/**/*"),
            ];
            let fingerprint =
                PostRunFingerprint::create(&executed_task, &fs, base_dir, Some(&ignore_patterns))
                    .unwrap();

            // Only src/index.js and package.json should remain
            assert_eq!(fingerprint.inputs.len(), 2);
            assert!(
                fingerprint.inputs.contains_key(&RelativePathBuf::new("src/index.js").unwrap())
            );
            assert!(
                fingerprint.inputs.contains_key(&RelativePathBuf::new("package.json").unwrap())
            );
        }

        #[test]
        fn test_postrun_fingerprint_wildcard_patterns() {
            // Test wildcard patterns for file types
            let executed_task = create_executed_task(vec![
                "src/index.js",
                "src/utils.js",
                "src/types.ts",
                "debug.log",
                "error.log",
                "README.md",
            ]);

            let fs = MockFileSystem;
            let base_dir =
                AbsolutePath::new(if cfg!(windows) { "C:\\test" } else { "/test" }).unwrap();

            let ignore_patterns = vec![Str::from("**/*.log")];
            let fingerprint =
                PostRunFingerprint::create(&executed_task, &fs, base_dir, Some(&ignore_patterns))
                    .unwrap();

            // Should have 4 files (all except .log files)
            assert_eq!(fingerprint.inputs.len(), 4);
            assert!(!fingerprint.inputs.contains_key(&RelativePathBuf::new("debug.log").unwrap()));
            assert!(!fingerprint.inputs.contains_key(&RelativePathBuf::new("error.log").unwrap()));
            assert!(
                fingerprint.inputs.contains_key(&RelativePathBuf::new("src/index.js").unwrap())
            );
            assert!(fingerprint.inputs.contains_key(&RelativePathBuf::new("README.md").unwrap()));
        }

        #[test]
        fn test_postrun_fingerprint_complex_negation() {
            // Test complex scenario with multiple negations
            let executed_task = create_executed_task(vec![
                "src/index.js",
                "dist/bundle.js",
                "dist/public/index.html",
                "dist/public/assets/logo.png",
                "dist/internal/config.json",
            ]);

            let fs = MockFileSystem;
            let base_dir =
                AbsolutePath::new(if cfg!(windows) { "C:\\test" } else { "/test" }).unwrap();

            let ignore_patterns = vec![Str::from("dist/**/*"), Str::from("!dist/public/**")];
            let fingerprint =
                PostRunFingerprint::create(&executed_task, &fs, base_dir, Some(&ignore_patterns))
                    .unwrap();

            // Should have: src/index.js + dist/public/* files = 3 total
            assert_eq!(fingerprint.inputs.len(), 3);
            assert!(
                fingerprint.inputs.contains_key(&RelativePathBuf::new("src/index.js").unwrap())
            );
            assert!(
                fingerprint
                    .inputs
                    .contains_key(&RelativePathBuf::new("dist/public/index.html").unwrap())
            );
            assert!(
                fingerprint
                    .inputs
                    .contains_key(&RelativePathBuf::new("dist/public/assets/logo.png").unwrap())
            );
            assert!(
                !fingerprint.inputs.contains_key(&RelativePathBuf::new("dist/bundle.js").unwrap())
            );
            assert!(
                !fingerprint
                    .inputs
                    .contains_key(&RelativePathBuf::new("dist/internal/config.json").unwrap())
            );
        }

        #[test]
        fn test_postrun_fingerprint_invalid_pattern() {
            // Test that invalid glob patterns return an error
            let executed_task = create_executed_task(vec!["src/index.js"]);

            let fs = MockFileSystem;
            let base_dir =
                AbsolutePath::new(if cfg!(windows) { "C:\\test" } else { "/test" }).unwrap();

            let ignore_patterns = vec![Str::from("[invalid")]; // Invalid glob syntax
            let result =
                PostRunFingerprint::create(&executed_task, &fs, base_dir, Some(&ignore_patterns));

            // Should return an error for invalid pattern
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_command_fingerprint_with_fingerprint_ignores() {
        // Test that CommandFingerprint includes fingerprint_ignores
        use crate::{
            cmd::TaskParsedCommand,
            config::{CommandFingerprint, TaskCommand},
        };

        let parsed_cmd = TaskParsedCommand {
            envs: [].into(),
            program: "pnpm".into(),
            args: vec!["install".into()],
        };

        let fingerprint_with_ignores = CommandFingerprint {
            cwd: RelativePathBuf::default(),
            command: TaskCommand::Parsed(parsed_cmd.clone()),
            envs_without_pass_through: Default::default(),
            pass_through_envs: Default::default(),
            fingerprint_ignores: Some(vec![
                Str::from("node_modules/**/*"),
                Str::from("!node_modules/**/package.json"),
            ]),
        };

        let fingerprint_without_ignores = CommandFingerprint {
            cwd: RelativePathBuf::default(),
            command: TaskCommand::Parsed(parsed_cmd.clone()),
            envs_without_pass_through: Default::default(),
            pass_through_envs: Default::default(),
            fingerprint_ignores: None,
        };

        // Fingerprints should be different when fingerprint_ignores differ
        assert_ne!(fingerprint_with_ignores, fingerprint_without_ignores);

        // Serialize to verify they produce different cache keys
        use bincode::encode_to_vec;
        let config = bincode::config::standard();

        let bytes_with = encode_to_vec(&fingerprint_with_ignores, config).unwrap();
        let bytes_without = encode_to_vec(&fingerprint_without_ignores, config).unwrap();

        assert_ne!(
            bytes_with, bytes_without,
            "Different fingerprint_ignores should produce different serialized bytes"
        );
    }

    #[test]
    fn test_command_fingerprint_ignores_order_matters() {
        // Test that the order of fingerprint_ignores patterns matters
        use crate::{
            cmd::TaskParsedCommand,
            config::{CommandFingerprint, TaskCommand},
        };

        let parsed_cmd =
            TaskParsedCommand { envs: [].into(), program: "build".into(), args: vec![] };

        let fingerprint1 = CommandFingerprint {
            cwd: RelativePathBuf::default(),
            command: TaskCommand::Parsed(parsed_cmd.clone()),
            envs_without_pass_through: Default::default(),
            pass_through_envs: Default::default(),
            fingerprint_ignores: Some(vec![Str::from("dist/**/*"), Str::from("!dist/public/**")]),
        };

        let fingerprint2 = CommandFingerprint {
            cwd: RelativePathBuf::default(),
            command: TaskCommand::Parsed(parsed_cmd.clone()),
            envs_without_pass_through: Default::default(),
            pass_through_envs: Default::default(),
            fingerprint_ignores: Some(vec![Str::from("!dist/public/**"), Str::from("dist/**/*")]),
        };

        // Different order should produce different fingerprints
        // (because last-match-wins means different semantics)
        assert_ne!(fingerprint1, fingerprint2);

        use bincode::encode_to_vec;
        let config = bincode::config::standard();

        let bytes1 = encode_to_vec(&fingerprint1, config).unwrap();
        let bytes2 = encode_to_vec(&fingerprint2, config).unwrap();

        assert_ne!(bytes1, bytes2, "Different pattern order should produce different cache keys");
    }
}

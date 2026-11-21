use serde_json::{Value, json};
use tempfile::TempDir;

use super::*;

fn option(flags: &[&str], mode: &str) -> Value {
    json!({"flags":flags,"valueMode":mode,"description":"Option description"})
}

fn metadata_value() -> Value {
    json!({
        "schemaVersion":1,"vitePlusVersion":"1.2.0","vitestVersion":"5.0.5",
        "globalOptions":[option(&["--coverage"],"none"),option(&["--coverage.reporter"],"required"),
            option(&["-w","--watch"],"none"),option(&["--reporter"],"required"),
            option(&["--silent"],"optional"),option(&["-u","--update"],"optional"),
            option(&["-p","--project"],"required"),
            {"flags":["--no-color"],"valueMode":"none","description":"Disable colors"}],
        "commands":[
            {"name":"","options":[]},
            {"name":"run","options":[]},
            {"name":"watch","options":[]},
            {"name":"list","options":[option(&["--filesOnly"],"none"),
                {"flags":["--reporter"],"valueMode":"required","description":"List override"}]},
            {"name":"init","options":[]},
            {"name":"complete","options":[]}
        ]
    })
}

fn metadata() -> Metadata {
    serde_json::from_value(metadata_value()).unwrap()
}

fn candidates(args: &[&str]) -> Vec<String> {
    let args: Vec<_> = args.iter().map(|arg| (*arg).to_owned()).collect();
    let command = test_command(&metadata(), &args[2..]).unwrap();
    let mut root = Command::new("vp").subcommand(command);
    root.clone().debug_assert();
    clap_complete::engine::complete(
        &mut root,
        args.iter().map(Into::into).collect(),
        args.len() - 1,
        None,
    )
    .unwrap()
    .into_iter()
    .map(|candidate| candidate.get_value().to_string_lossy().into_owned())
    .collect()
}

#[test]
fn completes_options_after_filters_and_subcommands() {
    for args in [
        vec!["vp", "test", "--cov"],
        vec!["vp", "test", "file.test.ts", "--cov"],
        vec!["vp", "test", "run", "--cov"],
        vec!["vp", "test", "watch", "--cov"],
    ] {
        assert!(candidates(&args).contains(&"--coverage".into()), "{args:?}");
    }
    assert_eq!(candidates(&["vp", "test", "--coverage.r"]), ["--coverage.reporter"]);
    assert_eq!(candidates(&["vp", "test", "--no-c"]), ["--no-color"]);
    assert!(candidates(&["vp", "test", "-"]).contains(&"-w".into()));
}

#[test]
fn recognizes_values_and_the_option_terminator() {
    for args in [
        vec!["vp", "test", "--reporter", ""],
        vec!["vp", "test", "--reporter=ver"],
        vec!["vp", "test", "--silent", ""],
        vec!["vp", "test", "-u", ""],
        vec!["vp", "test", "list", "--silent", ""],
        vec!["vp", "test", "--", "--cov"],
        vec!["vp", "test", "--project", "list", "--files"],
    ] {
        assert!(candidates(&args).is_empty(), "{args:?}");
    }
    assert!(candidates(&["vp", "test", "--silent", "--cov"]).contains(&"--coverage".into()));
    assert!(
        candidates(&["vp", "test", "--silent", "true", "--cov"]).contains(&"--coverage".into())
    );
    assert!(
        candidates(&["vp", "test", "--reporter", "--silent", ""]).iter().any(|s| s == "--coverage")
    );
}

#[test]
fn offers_subcommands_only_as_the_first_argument() {
    assert!(candidates(&["vp", "test", ""]).contains(&"list".into()));
    assert_eq!(candidates(&["vp", "test", "li"]), ["list"]);
    assert!(!candidates(&["vp", "test", "file.test.ts", ""]).contains(&"list".into()));
    assert!(!candidates(&["vp", "test", "--silent=true", ""]).contains(&"list".into()));
    assert!(candidates(&["vp", "test", "init", "--cov"]).is_empty());
    assert!(candidates(&["vp", "test", "complete", "--cov"]).is_empty());
    assert!(!candidates(&["vp", "test", ""]).contains(&"complete".into()));
}

#[test]
fn merges_list_overrides_and_preserves_descriptions() {
    assert_eq!(candidates(&["vp", "test", "list", "--files"]), ["--filesOnly"]);
    assert!(candidates(&["vp", "test", "--files"]).is_empty());
    let command = test_command(&metadata(), &["list".into(), "--rep".into()]).unwrap();
    let mut root = Command::new("vp").subcommand(command);
    let args = ["vp", "test", "list", "--rep"].map(Into::into).to_vec();
    let result = clap_complete::engine::complete(&mut root, args, 3, None).unwrap();
    assert_eq!(result[0].get_help().unwrap().to_string(), "List override");
}

#[test]
fn does_not_drop_repeatable_options_or_create_unregistered_negations() {
    assert!(
        candidates(&["vp", "test", "--reporter", "json", "--rep"]).contains(&"--reporter".into())
    );
    assert!(candidates(&["vp", "test", "--watch", "--wa"]).contains(&"--watch".into()));
    assert!(candidates(&["vp", "test", "--no-w"]).is_empty());
}

#[test]
fn decodes_transport_and_cursor_without_reindexing() {
    let args: Vec<_> = ["vp", "--", "vp", "test", "--cov", "later"].map(str::to_owned).to_vec();
    assert_eq!(request_words(&args, Some(2)).unwrap(), ["vp", "test", "--cov"]);
    assert_eq!(request_words(&args, Some(4)).unwrap().last().unwrap(), "");
    assert!(request_words(&args, Some(20)).is_none());
    assert!(request_words(&["vp".into()], None).is_none());
    for words in [
        vec!["vp", "-C", "project", "test", ""],
        vec!["vp", "-Cproject", "test", ""],
        vec!["vp", "-C=project", "test", ""],
    ] {
        let words: Vec<_> = words.iter().map(|s| (*s).into()).collect();
        assert_eq!(command_words(&words).unwrap(), ["test", ""]);
    }
}

fn fixture() -> (TempDir, std::path::PathBuf) {
    let temp = TempDir::new().unwrap();
    let package = temp.path().join("node_modules/vite-plus");
    fs::create_dir_all(package.join("dist/completion")).unwrap();
    fs::create_dir_all(package.join("node_modules/vitest")).unwrap();
    fs::write(package.join("package.json"), r#"{"name":"vite-plus","version":"1.2.0"}"#).unwrap();
    fs::write(
        package.join("node_modules/vitest/package.json"),
        r#"{"name":"vitest","version":"5.0.5"}"#,
    )
    .unwrap();
    fs::write(
        package.join("dist/completion/test.json"),
        serde_json::to_vec(&metadata_value()).unwrap(),
    )
    .unwrap();
    (temp, package.join("dist"))
}

#[test]
fn checks_both_toolchain_versions_without_running_javascript() {
    let (temp, dist) = fixture();
    assert!(read_metadata(&dist, temp.path()).is_some());
    fs::write(
        dist.parent().unwrap().join("node_modules/vitest/package.json"),
        r#"{"name":"vitest","version":"5.0.4"}"#,
    )
    .unwrap();
    assert!(read_metadata(&dist, temp.path()).is_none());
    fs::write(
        dist.parent().unwrap().join("node_modules/vitest/package.json"),
        r#"{"name":"vitest","version":"5.0.5"}"#,
    )
    .unwrap();
    fs::write(
        dist.parent().unwrap().join("package.json"),
        r#"{"name":"vite-plus","version":"1.1.0"}"#,
    )
    .unwrap();
    assert!(read_metadata(&dist, temp.path()).is_none());
}

#[test]
fn reads_minimal_metadata_and_ignores_previous_display_fields() {
    let (temp, dist) = fixture();
    let minimal = read_metadata(&dist, temp.path()).unwrap();
    let mut previous = metadata_value();
    for option in previous["globalOptions"].as_array_mut().unwrap() {
        option["rawName"] = json!("Previous declaration");
        option["negated"] = json!(false);
    }
    for command in previous["commands"].as_array_mut().unwrap() {
        command["rawName"] = json!("Previous command declaration");
        command["description"] = json!("Previous description");
    }
    fs::write(dist.join("completion/test.json"), previous.to_string()).unwrap();
    let previous = read_metadata(&dist, temp.path()).unwrap();
    for data in [minimal, previous] {
        let mut command = Command::new("vp").subcommand(test_command(&data, &[]).unwrap());
        command.clone().debug_assert();
        let args = ["vp", "test", "--no-c"].map(Into::into).to_vec();
        let result = clap_complete::engine::complete(&mut command, args, 2, None).unwrap();
        assert_eq!(result[0].get_value().to_string_lossy(), "--no-color");
    }
}

#[test]
fn legacy_vitest_missing_or_stale_metadata_preserves_original_completion() {
    for version in ["3.2.4", "4.1.10"] {
        for missing in [false, true] {
            let (temp, dist) = fixture();
            fs::write(
                dist.parent().unwrap().join("node_modules/vitest/package.json"),
                json!({"name":"vitest","version":version}).to_string(),
            )
            .unwrap();
            if missing {
                fs::remove_file(dist.join("completion/test.json")).unwrap();
            }
            assert!(read_metadata(&dist, temp.path()).is_none());
            for args in [
                vec!["vp", "test", "--cov"],
                vec!["vp", "test", "list", "--filesO"],
                vec!["vp", "test", "--glob"],
            ] {
                let words: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
                let root = crate::cli::command_with_help();
                let original = root.clone();
                let adapted = with_test_completion(root, &words, temp.path(), &dist);
                let values = |mut command: Command| {
                    clap_complete::engine::complete(
                        &mut command,
                        words.iter().map(Into::into).collect(),
                        words.len() - 1,
                        None,
                    )
                    .unwrap()
                    .into_iter()
                    .map(|candidate| candidate.get_value().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                };
                assert_eq!(values(adapted), values(original), "{version}, missing={missing}");
            }
        }
    }
}

#[test]
fn rejects_missing_corrupt_unknown_or_oversized_metadata() {
    let (temp, dist) = fixture();
    let file = dist.join("completion/test.json");
    for content in ["{}".to_owned(), "not json".into(), " ".repeat(MAX_METADATA_SIZE as usize + 1)]
    {
        fs::write(&file, content).unwrap();
        assert!(read_metadata(&dist, temp.path()).is_none());
    }
    let mut invalid = metadata_value();
    invalid["schemaVersion"] = json!(2);
    fs::write(&file, invalid.to_string()).unwrap();
    assert!(read_metadata(&dist, temp.path()).is_none());
    fs::remove_file(file).unwrap();
    assert!(read_metadata(&dist, temp.path()).is_none());
}

#[test]
fn rejects_conflicting_or_invalid_flags_before_clap_build() {
    let mut invalid = metadata_value();
    invalid["globalOptions"][0]["flags"] = json!(["--bad flag"]);
    assert!(!valid_metadata(&serde_json::from_value(invalid).unwrap()));
    let mut invalid = metadata_value();
    invalid["globalOptions"][0]["flags"] = json!(["-w", "--coverage"]);
    let invalid: Metadata = serde_json::from_value(invalid).unwrap();
    assert!(test_command(&invalid, &[]).is_none());
}

#[test]
fn rejects_duplicate_primary_flags_and_incompatible_inherited_declarations() {
    let mut invalid = metadata_value();
    let duplicate = invalid["globalOptions"][0].clone();
    invalid["globalOptions"].as_array_mut().unwrap().push(duplicate);
    let invalid: Metadata = serde_json::from_value(invalid).unwrap();
    assert!(test_command(&invalid, &[]).is_none());

    let mut invalid = metadata_value();
    invalid["commands"][3]["options"][1]["valueMode"] = json!("none");
    let invalid: Metadata = serde_json::from_value(invalid).unwrap();
    assert!(test_command(&invalid, &["list".into()]).is_none());
}

#[test]
fn loads_cli_dependency_before_project_vitest_and_can_use_project_fallback() {
    let (temp, dist) = fixture();
    let project_vitest = temp.path().join("node_modules/vitest");
    fs::create_dir_all(&project_vitest).unwrap();
    for version in ["3.2.4", "4.1.10", "5.0.4"] {
        fs::write(
            project_vitest.join("package.json"),
            json!({"name":"vitest","version":version}).to_string(),
        )
        .unwrap();
        assert!(read_metadata(&dist, temp.path()).is_some());
    }
    fs::remove_dir_all(dist.parent().unwrap().join("node_modules/vitest")).unwrap();
    assert!(read_metadata(&dist, temp.path()).is_none());
    fs::write(project_vitest.join("package.json"), r#"{"name":"vitest","version":"5.0.5"}"#)
        .unwrap();
    assert!(read_metadata(&dist, temp.path()).is_some());
}

// This is Vite+'s mapping policy, tested with synthetic upstream definitions.
// Adding/removing upstream commands must not require an inventory snapshot update.
#[test]
fn maps_only_available_supported_commands_and_uses_selected_scope() {
    let supported = ["run", "watch", "dev", "related", "bench", "list", "doctor"];
    let mut value = metadata_value();
    value["globalOptions"] = json!([option(&["--shared"], "none")]);
    let mut commands = vec![json!({
        "name":"",
        "options":[option(&["--default-only"], "none")]
    })];
    for name in supported.into_iter().chain(["future-command"]) {
        commands.push(json!({
            "name":name,
            "options":[option(&["--local-only"], "none")]
        }));
    }
    value["commands"] = json!(commands);
    let data: Metadata = serde_json::from_value(value.clone()).unwrap();
    let root = test_command(&data, &[String::new()]).unwrap();
    let names: Vec<_> = root.get_subcommands().map(Command::get_name).collect();
    assert_eq!(names, supported);
    assert!(root.get_arguments().any(|arg| arg.get_long() == Some("local-only")));
    assert!(!root.get_arguments().any(|arg| arg.get_long() == Some("default-only")));
    for name in supported {
        let command = root.find_subcommand(name).unwrap();
        assert!(command.get_arguments().any(|arg| arg.get_long() == Some("shared")));
        assert!(command.get_arguments().any(|arg| arg.get_long() == Some("local-only")));
        assert!(!command.get_arguments().any(|arg| arg.get_long() == Some("default-only")));
    }
    // Upstream may omit a currently supported command; never manufacture it.
    value["commands"] = json!([]);
    let data: Metadata = serde_json::from_value(value).unwrap();
    assert!(test_command(&data, &[String::new()]).is_none());
}

#[test]
fn invalid_metadata_preserves_nonempty_original_completion() {
    let (temp, dist) = fixture();
    let file = dist.join("completion/test.json");
    let mut unknown_schema = metadata_value();
    unknown_schema["schemaVersion"] = json!(2);
    let mut conflict = metadata_value();
    let duplicate = conflict["globalOptions"][0].clone();
    conflict["globalOptions"].as_array_mut().unwrap().push(duplicate);
    let mut mismatch = metadata_value();
    mismatch["vitestVersion"] = json!("0.0.0");
    for content in [
        None,
        Some("not json".to_owned()),
        Some(unknown_schema.to_string()),
        Some(conflict.to_string()),
        Some(mismatch.to_string()),
        Some(" ".repeat(MAX_METADATA_SIZE as usize + 1)),
    ] {
        if let Some(content) = content {
            fs::write(&file, content).unwrap();
        } else {
            fs::remove_file(&file).unwrap();
        }
        let words: Vec<String> = ["vp", "test", "--base"].map(str::to_owned).to_vec();
        let root = crate::cli::command_with_help().mut_subcommand("test", |command| {
            command.arg(Arg::new("baseline").long("baseline").action(ArgAction::SetTrue))
        });
        let adapted = with_test_completion(root.clone(), &words, temp.path(), &dist);
        let values = |mut command: Command| {
            clap_complete::engine::complete(
                &mut command,
                words.iter().map(Into::into).collect(),
                2,
                None,
            )
            .unwrap()
            .into_iter()
            .map(|candidate| candidate.get_value().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
        };
        let original = values(root);
        assert!(original.contains(&"--baseline".to_owned()));
        assert_eq!(values(adapted), original);
    }
}

#[test]
fn implicit_run_scope_follows_vite_plus_dispatch_exceptions() {
    let mut value = metadata_value();
    value["commands"][0]["options"] = json!([option(&["--default-only"], "none")]);
    value["commands"][1]["options"] = json!([option(&["--run-only"], "none")]);
    let data: Metadata = serde_json::from_value(value).unwrap();
    for (args, prepend) in [
        (vec![], true),
        (vec![""], true),
        (vec!["filter.test.ts", "--ru"], true),
        (vec!["--watch"], false),
        (vec!["-w"], false),
        (vec!["--help"], false),
        (vec!["-h"], false),
        (vec!["--run"], false),
        (vec!["--", "--watch"], true),
        (vec!["--watch=false"], true),
        (vec!["--run=false"], true),
        (vec!["--project", "watch"], true),
    ] {
        let args: Vec<String> = args.iter().map(|arg| (*arg).into()).collect();
        assert_eq!(implicit_run(&args), prepend, "{args:?}");
        let root = test_command(&data, &args).unwrap();
        let expected = if prepend { "run-only" } else { "default-only" };
        let excluded = if prepend { "default-only" } else { "run-only" };
        assert!(root.get_arguments().any(|arg| arg.get_long() == Some(expected)), "{args:?}");
        assert!(!root.get_arguments().any(|arg| arg.get_long() == Some(excluded)), "{args:?}");
    }
    for name in SUBCOMMANDS.iter().copied().chain(["init", "complete"]) {
        assert!(!implicit_run(&[name.into()]));
    }
    let command = test_command(&data, &["run".into(), "--ru".into()]).unwrap();
    assert!(
        command
            .find_subcommand("run")
            .unwrap()
            .get_arguments()
            .any(|arg| arg.get_long() == Some("run-only"))
    );
}

#[test]
fn rejects_schema_boundary_violations() {
    for field in ["flags", "description", "valueMode"] {
        let mut missing = metadata_value();
        missing["globalOptions"][0].as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<Metadata>(missing).is_err(), "{field}");
    }
    let mut invalid_arity = metadata_value();
    invalid_arity["globalOptions"][0]["valueMode"] = json!("unsupported");
    assert!(serde_json::from_value::<Metadata>(invalid_arity).is_err());

    for (pointer, invalid) in [
        ("/commands/1/name", json!("bad/name")),
        ("/commands/1/name", json!("x".repeat(65))),
        ("/globalOptions/0/flags", json!(["-ab", "--coverage"])),
        ("/globalOptions/0/flags", json!([format!("--{}", "x".repeat(257))])),
        ("/globalOptions/0/description", json!("文".repeat(5462))),
    ] {
        let mut value = metadata_value();
        *value.pointer_mut(pointer).unwrap() = invalid;
        assert!(!valid_metadata(&serde_json::from_value(value).unwrap()), "{pointer}");
    }
    let mut duplicate = metadata_value();
    duplicate["commands"][2]["name"] = json!("run");
    assert!(!valid_metadata(&serde_json::from_value(duplicate).unwrap()));
    let mut excessive = metadata_value();
    excessive["globalOptions"] = json!(vec![option(&["--synthetic"], "none"); 2049]);
    assert!(!valid_metadata(&serde_json::from_value(excessive).unwrap()));
}

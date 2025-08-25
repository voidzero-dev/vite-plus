use std::{
    collections::{btree_map::Entry, BTreeMap, HashMap},
    env::{self, args},
    fs::{read, File},
    io::{stderr, BufWriter, Write as _},
    path::PathBuf,
    process::{self, Stdio},
};

use fspy::{AccessMode, PathAccess};
use futures_util::future::try_join;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Config {
    cases: HashMap<String, Case>,
}

#[derive(Serialize, Deserialize)]
struct Case {
    dir: String,
    cmd: Vec<String>,
}

struct AccessCollector {
    dir: PathBuf,
    accesses: BTreeMap<String, AccessMode>,
}

impl AccessCollector {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            accesses: BTreeMap::new(),
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = (&str, AccessMode)> {
        self.accesses.iter().map(|(k, v)| (k.as_str(), *v))
    }
    pub fn add(&mut self, access: PathAccess) {
        let path = PathBuf::from(access.path.to_cow_os_str().to_os_string());
        if let Ok(relative_path) = path.strip_prefix(&self.dir) {
            let relative_path = relative_path
                .to_str()
                .expect("relative path should be valid UTF-8")
                .to_owned();
            match self.accesses.entry(relative_path) {
                Entry::Vacant(vacant) => {
                    vacant.insert(access.mode);
                }
                Entry::Occupied(mut occupied) => {
                    let occupied_mode = occupied.get_mut();
                    match (*occupied_mode, access.mode) {
                        (_, AccessMode::ReadWrite) => {
                            *occupied_mode = AccessMode::ReadWrite;
                        }
                        (AccessMode::Read, AccessMode::ReadDir) => {
                            *occupied_mode = AccessMode::ReadDir;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let mut args = args();
    args.next(); // skip the first argument (the program name)
    let filter = args.next();
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config = read(manifest_dir.join("e2e_config.toml")).unwrap();
    let config: Config = toml::from_slice(&config).unwrap();
    let spy = fspy::Spy::global().unwrap();
    for (name, case) in config.cases {
        if let Some(filter) = &filter {
            if !name.contains(filter) {
                continue;
            }
        }
        println!("Running case `{}` in dir `{}`", name, case.dir);
        let mut cmd = spy.new_command(case.cmd[0].clone());
        let dir = manifest_dir.join(&case.dir);
        cmd.args(&case.cmd[1..])
            .envs(env::vars_os())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(&dir);

        let tracked_child = cmd.spawn().await.unwrap();

        let (accesses, output) = try_join(
            tracked_child.accesses_future,
            tracked_child.tokio_child.wait_with_output(),
        )
        .await
        .unwrap();
        if !output.status.success() {
            eprintln!("----- stdout begin -----");
            stderr().write_all(&output.stdout).unwrap();
            eprintln!("----- stdout end -----");
            eprintln!("----- stderr begin-----");
            stderr().write_all(&output.stderr).unwrap();
            eprintln!("----- stderr end -----");

            eprintln!("Case `{}` failed with status: {}", name, output.status);
            process::exit(1);
        }

        let mut collector = AccessCollector::new(dir);
        for access in accesses.iter() {
            collector.add(access);
        }
        let snap_file = File::create(manifest_dir.join(format!("snaps/{}.txt", name))).unwrap();
        let mut snap_writer = BufWriter::new(snap_file);
        for (path, mode) in collector.iter() {
            writeln!(snap_writer, "{}: {:?}", path, mode).unwrap();
        }
    }
}

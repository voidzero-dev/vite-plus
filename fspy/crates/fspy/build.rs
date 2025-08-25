use std::{
    env::{self, current_dir},
    ffi::{OsStr, OsString},
    fs,
    io::Read,
    path::Path,
};

use anyhow::{Context, bail};
use xxhash_rust::xxh3::xxh3_128;

fn download(url: &str) -> anyhow::Result<impl Read + use<>> {
    let resp = attohttpc::get(url).send().unwrap();
    if resp.status() != attohttpc::StatusCode::OK {
        bail!("non-ok response: {:?}", resp.status())
    }
    Ok(resp)
}

fn unpack_tar_gz(content: impl Read, path: &str) -> anyhow::Result<Vec<u8>> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    // let path = path.as_ref();
    let tar = GzDecoder::new(content);
    let mut archive = Archive::new(tar);
    for entry in archive.entries()? {
        let mut entry = entry?;
        if entry.path_bytes().as_ref() == path.as_bytes() {
            let mut data = Vec::<u8>::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut data)?;
            return Ok(data);
        }
    }
    bail!("Path {} not found in tar gz", path)
}

fn download_and_unpack_tar_gz(url: &str, path: &str) -> anyhow::Result<Vec<u8>> {
    let resp = download(url).context(format!("Failed to get ok response from {}", url))?;
    let data = unpack_tar_gz(resp, path).context(format!(
        "Failed to download or unpack {} out of {}",
        path, url
    ))?;
    Ok(data)
}

const MACOS_BINARY_DOWNLOADS: &[(&str, &[(&str, &str, u128)])] = &[
    (
        "aarch64",
        &[
            (
                "https://github.com/branchseer/oils-for-unix-binaries/releases/download/0.29.0-manual/oils-for-unix-0.29.0-aarch64-apple-darwin.tar.gz",
                "oils-for-unix",
                149945237112824769531360595981178091193,
            ),
            (
                "https://github.com/uutils/coreutils/releases/download/0.1.0/coreutils-0.1.0-aarch64-apple-darwin.tar.gz",
                "coreutils-0.1.0-aarch64-apple-darwin/coreutils",
                255656813290649147736009964224176006890,
            ),
        ],
    ),
    (
        "x86_64",
        &[
            (
                "https://github.com/branchseer/oils-for-unix-binaries/releases/download/0.29.0-manual/oils-for-unix-0.29.0-x86_64-apple-darwin.tar.gz",
                "oils-for-unix",
                286203014616009968685843701528129413859,
            ),
            (
                "https://github.com/uutils/coreutils/releases/download/0.1.0/coreutils-0.1.0-x86_64-apple-darwin.tar.gz",
                "coreutils-0.1.0-x86_64-apple-darwin/coreutils",
                75344743234387926348628744659874018387,
            ),
        ],
    ),
];

fn fetch_macos_binaries() -> anyhow::Result<()> {
    if env::var("CARGO_CFG_TARGET_OS").unwrap() != "macos" {
        return Ok(());
    };
    let out_dir = current_dir()
        .unwrap()
        .join(Path::new(&std::env::var_os("OUT_DIR").unwrap()));

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let downloads = MACOS_BINARY_DOWNLOADS
        .iter()
        .find(|(arch, _)| *arch == target_arch)
        .context(format!("Unsupported macOS arch: {}", target_arch))?
        .1;
    // let downloads = [(zsh_url.as_str(), "bin/zsh", zsh_hash)];
    for (url, path_in_targz, expected_hash) in downloads.iter().copied() {
        let filename = path_in_targz.split('/').rev().next().unwrap();
        let download_path = out_dir.join(filename);
        let hash_path = out_dir.join(format!("{}.hash", filename));

        let file_exists = matches!(fs::read(&download_path), Ok(existing_file_data) if xxh3_128(&existing_file_data) == expected_hash);
        if !file_exists {
            let data = download_and_unpack_tar_gz(url, path_in_targz)?;
            fs::write(&download_path, &data).context(format!(
                "Saving {path_in_targz} in {url} to {}",
                download_path.display()
            ))?;
            let actual_hash = xxh3_128(&data);
            assert_eq!(
                actual_hash, expected_hash,
                "expected_hash of {} in {} needs to be updated",
                path_in_targz, url
            );
        }
        fs::write(&hash_path, format!("{:x}", expected_hash))?;
    }
    Ok(())
    // let zsh_path = ensure_downloaded(&zsh_url);
}

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    fetch_macos_binaries().context("Failed to fetch macOS binaries")?;
    Ok(())
}

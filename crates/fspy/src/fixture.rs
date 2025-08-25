use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

pub struct Fixture {
    pub name: &'static str,
    pub content: &'static [u8],
    pub hash: &'static str,
}

#[doc(hidden)]
#[macro_export]
macro_rules! fixture  {
    ($name: literal) => {
        $crate::fixture::Fixture::new(
            $name,
            ::core::include_bytes!(::core::concat!(::core::env!("OUT_DIR"), "/", $name)),
            ::core::include_str!(::core::concat!(::core::env!("OUT_DIR"), "/", $name, ".hash")),
        )
    };
}

pub use fixture;

impl Fixture {
    pub const fn new(name: &'static str, content: &'static [u8], hash: &'static str) -> Self {
        Self {
            name,
            content,
            hash
        }
    }
    pub fn write_to(&self, dir: impl AsRef<Path>, suffix: &str) -> io::Result<PathBuf> {

        let dir = dir.as_ref();
        let path = dir.join(format!("{}_{}{}", self.name, self.hash, suffix));

        if fs::exists(&path)? {
            return Ok(path);
        }
        let tmp_path = dir.join(format!("{:x}", rand::random::<u128>()));
        let mut tmp_file_open_options = OpenOptions::new();
        tmp_file_open_options.write(true)
            .create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut tmp_file_open_options, 0o755);// executable
        let mut tmp_file = tmp_file_open_options.open(&tmp_path)?;
        tmp_file.write_all(self.content)?;
        drop(tmp_file);

        if let Err(err) = fs::rename(&tmp_path, &path) {
            if !fs::exists(&path)? {
                return Err(err);
            }
            fs::remove_file(&tmp_path)?;
        }
        Ok(path)
    }
}

use std::path::Path;

// pub trait FileSystem {
//     fn peek_executable(&self, path: &Path, buf: &mut [u8]) -> nix::Result<usize>;
// }

// pub struct RealFileSystem;

// impl FileSystem for RealFileSystem {
//     fn peek_executable(&self, path: &Path, buf: &mut [u8]) -> nix::Result<usize> {
//         std::fs::File::open(path)?.read(buf)
//     }
// }

use std::{
    ffi::{CStr, OsStr},
    os::unix::ffi::OsStrExt as _,
    path::Path,
};

use bstr::BStr;
use elf::{ElfBytes, abi::PT_INTERP, endian::AnyEndian};

pub fn is_dynamically_linked_to_libc(executable: impl AsRef<[u8]>) -> nix::Result<bool> {
    let executable = executable.as_ref();
    let Some(interp) = get_interp(executable)? else {
        return Ok(false);
    };
    let Some(interp_filename) = Path::new(OsStr::from_bytes(interp)).file_name() else {
        return Ok(false);
    };
    let interp_filename = interp_filename.as_bytes();
    Ok(interp_filename.starts_with(b"ld-") || interp_filename.starts_with(b"ld."))
}

fn get_interp(executable: &[u8]) -> nix::Result<Option<&BStr>> {
    let elf = ElfBytes::<'_, AnyEndian>::minimal_parse(executable.as_ref())
        .map_err(|_| nix::Error::ENOEXEC)?;
    let Some(headers) = elf.segments() else {
        return Ok(None);
    };

    let Some(interp_header) = headers
        .into_iter()
        .find(|header| header.p_type == PT_INTERP)
    else {
        return Ok(None);
    };
    let Ok(interp) = elf.segment_data(&interp_header) else {
        return Err(nix::Error::ENOEXEC);
    };

    let interp = CStr::from_bytes_until_nul(interp)
        .map(CStr::to_bytes)
        .unwrap_or(interp);
    Ok(Some(BStr::new(interp)))
}

#[cfg(test)]
mod tests {
    use std::fs::read;

    use super::*;
    #[test]
    fn dynamic_executable() {
        assert_eq!(
            is_dynamically_linked_to_libc(read("/bin/sh").unwrap()).unwrap(),
            true
        );
    }
    #[test]
    fn static_executable() {
        let cat = read("/bin/cat").unwrap();
        let ld_so_path = get_interp(&cat).unwrap().unwrap();

        assert_eq!(
            is_dynamically_linked_to_libc(read(OsStr::from_bytes(ld_so_path)).unwrap()).unwrap(),
            false
        );
    }
}

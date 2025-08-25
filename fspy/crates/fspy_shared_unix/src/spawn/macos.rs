use crate::{
    exec::{Exec, ensure_env},
    payload::{EncodedPayload, PAYLOAD_ENV_NAME},
};
use phf::{Set, phf_set};
use std::{
    convert::Infallible,
    ffi::OsStr,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, absolute},
};

pub struct PreExec(Infallible);
impl PreExec {
    pub fn run(&mut self) -> nix::Result<()> {
        match self.0 {}
    }
}

pub fn handle_exec(
    command: &mut Exec,
    encoded_payload: &EncodedPayload,
) -> nix::Result<Option<PreExec>> {
    if command.program.first() != Some(&b'/') {
        let program =
            absolute(OsStr::from_bytes(&command.program)).expect("Failed to get absolute path");
        command.program = program.into_os_string().into_vec().into();
    }

    let program_path = Path::new(OsStr::from_bytes(&command.program));

    let injectable = if let (Some(parent), Some(file_name)) =
        (program_path.parent(), program_path.file_name())
    {
        if matches!(parent.as_os_str().as_bytes(), b"/bin" | b"/usr/bin") {
            let fixtures = &encoded_payload.payload.fixtures;
            if matches!(file_name.as_bytes(), b"sh" | b"bash") {
                command.program = fixtures.bash_path.as_bytes().into();
                true
            } else if COREUTILS_FUNCTIONS.contains(file_name.as_bytes()) {
                command.program = fixtures.coreutils_path.as_bytes().into();
                true
            } else {
                false
            }
        } else {
            true
        }
    } else {
        true
    };

    const DYLD_INSERT_LIBRARIES: &[u8] = b"DYLD_INSERT_LIBRARIES";
    if injectable {
        ensure_env(
            &mut command.envs,
            DYLD_INSERT_LIBRARIES,
            &encoded_payload
                .payload
                .preload_path
                .as_bytes(),
        )?;
        ensure_env(
            &mut command.envs,
            PAYLOAD_ENV_NAME,
            &encoded_payload.encoded_string,
        )?;
    } else {
        command.envs.retain(|(name, _)| {
            name != DYLD_INSERT_LIBRARIES && name != PAYLOAD_ENV_NAME.as_bytes()
        });
    }
    Ok(None)
}

pub static COREUTILS_FUNCTIONS: Set<&'static [u8]> = phf_set! {
    b"[", b"arch", b"b2sum", b"b3sum", b"base32", b"base64", b"basename", b"basenc",
    b"cat", b"chgrp", b"chmod", b"chown", b"chroot", b"cksum", b"comm", b"cp", b"csplit",
    b"cut", b"date", b"dd", b"df", b"dir", b"dircolors", b"dirname", b"du", b"echo", b"env",
    b"expand", b"expr", b"factor", b"false", b"fmt", b"fold", b"groups", b"hashsum", b"head",
    b"hostid", b"hostname", b"id", b"install", b"join", b"kill", b"link", b"ln", b"logname",
    b"ls", b"md5sum", b"mkdir", b"mkfifo", b"mknod", b"mktemp", b"more", b"mv", b"nice", b"nl",
    b"nohup", b"nproc", b"numfmt", b"od", b"paste", b"pathchk", b"pinky", b"pr", b"printenv",
    b"printf", b"ptx", b"pwd", b"readlink", b"realpath", b"rm", b"rmdir", b"seq", b"sha1sum",
    b"sha224sum", b"sha256sum", b"sha3-224sum", b"sha3-256sum", b"sha3-384sum", b"sha3-512sum",
    b"sha384sum", b"sha3sum", b"sha512sum", b"shake128sum", b"shake256sum", b"shred", b"shuf",
    b"sleep", b"sort", b"split", b"stat", b"stdbuf", b"stty", b"sum", b"sync", b"tac", b"tail",
    b"tee", b"test", b"timeout", b"touch", b"tr", b"true", b"truncate", b"tsort", b"tty", b"uname",
    b"unexpand", b"uniq", b"unlink", b"uptime", b"users", b"vdir", b"wc", b"who", b"whoami", b"yes",
};

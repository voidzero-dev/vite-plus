use base64::{Engine as _, prelude::BASE64_STANDARD_NO_PAD};
use bincode::{Decode, Encode, config::standard};
use bstr::BString;
use fspy_shared::ipc::NativeString;

use std::{
    os::{
        fd::RawFd,
        unix::ffi::OsStringExt,
    },
};

#[derive(Debug, Encode, Decode)]
pub struct Payload {
    pub ipc_fd: RawFd,
    pub preload_path: NativeString,

    #[cfg(target_os = "macos")]
    pub fixtures: Fixtures,

    #[cfg(target_os = "linux")]
    pub seccomp_payload: seccomp_unotify::payload::SeccompPayload,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Encode, Decode, Clone)]
pub struct Fixtures {
    pub bash_path: NativeString,
    pub coreutils_path: NativeString,
    // pub interpose_cdylib_path: NativeString,
}


pub(crate) const PAYLOAD_ENV_NAME: &str = "FSPY_PAYLOAD";

pub struct EncodedPayload {
    pub payload: Payload,
    pub encoded_string: BString,
}

pub fn encode_payload(payload: Payload) -> EncodedPayload {
    let bincode_bytes = bincode::encode_to_vec(&payload, standard()).unwrap();
    let encoded_string = BASE64_STANDARD_NO_PAD.encode(&bincode_bytes);
    EncodedPayload {
        payload,
        encoded_string: encoded_string.into(),
    }
}

pub fn decode_payload_from_env() -> anyhow::Result<EncodedPayload> {
    let Some(encoded_string) = std::env::var_os(PAYLOAD_ENV_NAME) else {
        anyhow::bail!("Environemnt variable '{}' not found", PAYLOAD_ENV_NAME);
    };
    decode_payload(encoded_string.into_vec().into())
}

fn decode_payload(encoded_string: BString) -> anyhow::Result<EncodedPayload> {
    let bincode_bytes = BASE64_STANDARD_NO_PAD.decode(&encoded_string)?;
    let (payload, n) = bincode::decode_from_slice::<Payload, _>(&bincode_bytes, standard())?;
    assert_eq!(bincode_bytes.len(), n);
    Ok(EncodedPayload {
        payload,
        encoded_string,
    })
}

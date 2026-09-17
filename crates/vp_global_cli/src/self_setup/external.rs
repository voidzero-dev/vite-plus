//! Per-user setup receipts for binaries owned by an external installer.

use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    time::SystemTime,
};

use serde::{Deserialize, Serialize};
use vp_setup::SELF_SETUP_MARKER;
use vp_shared::EnvConfig;
use vt_path::{AbsolutePath, AbsolutePathBuf};

use crate::error::Error;

#[derive(Deserialize, Serialize, PartialEq, Eq)]
struct Source {
    path: PathBuf,
    version: String,
    modified: SystemTime,
    len: u64,
}

#[derive(Deserialize, Serialize)]
struct Receipt {
    source: Source,
    binary: PathBuf,
}

pub(super) struct SetupState {
    source: Source,
    path: AbsolutePathBuf,
}

impl SetupState {
    pub(super) fn new(binary: &Path, local_version: Option<&str>) -> Result<Self, Error> {
        let metadata = std::fs::metadata(binary)?;
        let mut hash = rustc_hash::FxHasher::default();
        binary.hash(&mut hash);
        Ok(Self {
            source: Source {
                path: binary.to_path_buf(),
                version: local_version.unwrap_or(env!("CARGO_PKG_VERSION")).to_string(),
                modified: metadata.modified()?,
                len: metadata.len(),
            },
            path: EnvConfig::get()
                .dirs
                .state
                .join("self-setup")
                .join(format!("{:016x}.json", hash.finish())),
        })
    }

    pub(super) async fn completed_binary(
        &self,
        bundled: bool,
    ) -> Result<Option<AbsolutePathBuf>, Error> {
        let data = match tokio::fs::read(&self.path).await {
            Ok(data) => data,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        // An interrupted receipt write must allow setup to retry.
        let Ok(receipt) = serde_json::from_slice::<Receipt>(&data) else { return Ok(None) };
        if receipt.source != self.source || !receipt.binary.is_file() {
            return Ok(None);
        }
        let complete = if bundled {
            receipt.binary == self.source.path
        } else {
            receipt.binary != self.source.path
                && receipt.binary.parent().is_some_and(|bin| bin.join(SELF_SETUP_MARKER).is_file())
        };
        if !complete {
            return Ok(None);
        }
        Ok(AbsolutePathBuf::new(receipt.binary))
    }

    pub(super) async fn save(self, binary: &AbsolutePath) -> Result<(), Error> {
        tokio::fs::create_dir_all(self.path.parent().ok_or(Error::CliBinaryNotFound)?).await?;
        let receipt = Receipt { source: self.source, binary: binary.as_path().to_path_buf() };
        tokio::fs::write(&self.path, serde_json::to_vec(&receipt)?).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bundled_receipt_is_per_user_and_invalidated_when_source_changes() {
        EnvConfig::scoped_async(|env| async move {
            let binary = env.dirs.data.join("external/bin/vp");
            tokio::fs::create_dir_all(binary.parent().unwrap()).await.unwrap();
            tokio::fs::write(&binary, b"vp").await.unwrap();
            let state = SetupState::new(binary.as_path(), None).unwrap();
            assert!(state.path.as_path().starts_with(env.dirs.state.as_path()));
            assert!(state.completed_binary(true).await.unwrap().is_none());
            state.save(&binary).await.unwrap();

            let state = SetupState::new(binary.as_path(), None).unwrap();
            assert_eq!(state.completed_binary(true).await.unwrap(), Some(binary.clone()));
            assert!(state.completed_binary(false).await.unwrap().is_none());
            assert!(!binary.parent().unwrap().join(SELF_SETUP_MARKER).as_path().exists());

            tokio::fs::write(&binary, b"new vp build").await.unwrap();
            let state = SetupState::new(binary.as_path(), None).unwrap();
            assert!(state.completed_binary(true).await.unwrap().is_none());
        })
        .await;
    }

    #[tokio::test]
    async fn standalone_receipt_requires_a_marked_deployed_binary() {
        EnvConfig::scoped_async(|env| async move {
            let source = env.dirs.data.join("vp");
            let binary = env.dirs.data.join("version/bin/vp");
            tokio::fs::write(&source, b"vp").await.unwrap();
            tokio::fs::create_dir_all(binary.parent().unwrap()).await.unwrap();
            tokio::fs::write(&binary, b"vp").await.unwrap();
            SetupState::new(source.as_path(), None).unwrap().save(&binary).await.unwrap();
            let state = SetupState::new(source.as_path(), None).unwrap();
            assert!(state.completed_binary(false).await.unwrap().is_none());

            let marker = binary.parent().unwrap().join(SELF_SETUP_MARKER);
            tokio::fs::write(&marker, b"").await.unwrap();
            assert_eq!(state.completed_binary(false).await.unwrap(), Some(binary.clone()));
            // A package manager can add a bundled CLI after a bare-binary install.
            assert!(state.completed_binary(true).await.unwrap().is_none());

            // Upgrade clears the marker; cleanup or implode can remove the target entirely.
            tokio::fs::remove_file(&marker).await.unwrap();
            assert!(state.completed_binary(false).await.unwrap().is_none());
            tokio::fs::write(&marker, b"").await.unwrap();
            tokio::fs::remove_file(&binary).await.unwrap();
            assert!(state.completed_binary(false).await.unwrap().is_none());
        })
        .await;
    }

    #[tokio::test]
    async fn changed_version_or_incomplete_receipt_retries_setup() {
        EnvConfig::scoped_async(|env| async move {
            let binary = env.dirs.data.join("vp");
            tokio::fs::write(&binary, b"vp").await.unwrap();
            SetupState::new(binary.as_path(), Some("first")).unwrap().save(&binary).await.unwrap();
            let state = SetupState::new(binary.as_path(), Some("second")).unwrap();
            assert!(state.completed_binary(true).await.unwrap().is_none());

            tokio::fs::write(&state.path, b"{\"source\":").await.unwrap();
            let state = SetupState::new(binary.as_path(), Some("first")).unwrap();
            assert!(state.completed_binary(true).await.unwrap().is_none());
        })
        .await;
    }
}

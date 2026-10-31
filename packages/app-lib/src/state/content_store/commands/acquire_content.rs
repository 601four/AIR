use crate::ErrorKind;
use crate::state::content_store::adapters::downloads;
use crate::state::content_store::adapters::filesystem;
use crate::state::content_store::adapters::sqlite as catalog;
use crate::state::content_store::{
    ContentStore, GetFileResult, StoredFileHandle, StoredFileMetadata,
    StoredFileStatus, validate_digest,
};
use crate::util::content_hash::{hash_file, temporary_file};
use crate::util::fetch::{self, DownloadMeta, FetchProgressFn, FetchSemaphore};
use anyhow::{Context, bail};
use sha2::{Digest, Sha512};
use std::path::Path;
use tokio::fs::File;

impl ContentStore {
    pub(crate) async fn get_or_download_file(
        &self,
        mirrors: &[&str],
        sha512: Option<&str>,
        sha1: Option<&str>,
        size: Option<u64>,
        download_meta: Option<&DownloadMeta>,
        semaphore: &FetchSemaphore,
        progress: Option<&mut FetchProgressFn<'_>>,
    ) -> crate::Result<GetFileResult> {
        let Some(sha512) = sha512 else {
            return self
                .get_or_download_file_with_sha1(
                    mirrors,
                    sha1,
                    size,
                    download_meta,
                    semaphore,
                    progress,
                )
                .await;
        };
        validate_digest(sha512, 128)?;
        let _acquisition = self.download_lock(sha512).lock().await;
        if let Some(stored_file) = self.lookup(Some(sha512), size).await? {
            return Ok(GetFileResult {
                stored_file,
                reused: true,
            });
        }
        let download = downloads::download(
            &self.staging,
            &self.pool,
            mirrors,
            download_meta,
            semaphore,
            progress,
        )
        .await?;
        if sha512 != download.sha512 {
            bail!(ErrorKind::ContentStoreError(format!(
                "downloaded content SHA-512 mismatch: expected {sha512}, got {}",
                download.sha512,
            )));
        }
        if let Some(expected_size) = size
            && expected_size != download.size
        {
            tracing::warn!(
                expected_size,
                actual_size = download.size,
                sha512 = %download.sha512,
                "Downloaded content matches SHA-512 but differs from the declared size; using its actual size"
            );
        }
        let sources = mirrors
            .iter()
            .filter_map(|source| {
                let url = url::Url::parse(source).ok()?;
                (url.scheme() == "https"
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none())
                .then(|| source.to_string())
            })
            .collect::<Vec<_>>();
        let stored_file = self
            .save_staged_file(download.into_staged()?, &sources)
            .await?;
        Ok(GetFileResult {
            stored_file,
            reused: false,
        })
    }

    /// 无 SHA-512 时的回退：先下载，再用 SHA-1 校验（CurseForge 只提供 SHA-1）
    async fn get_or_download_file_with_sha1(
        &self,
        mirrors: &[&str],
        sha1: Option<&str>,
        size: Option<u64>,
        download_meta: Option<&DownloadMeta>,
        semaphore: &FetchSemaphore,
        progress: Option<&mut FetchProgressFn<'_>>,
    ) -> crate::Result<GetFileResult> {
        let sha1 = sha1.ok_or_else(|| {
            ErrorKind::ContentStoreError(
                "content download is missing a SHA-1 or SHA-512 hash".into(),
            )
        })?;
        validate_digest(sha1, 40)?;
        let download = downloads::download(
            &self.staging,
            &self.pool,
            mirrors,
            download_meta,
            semaphore,
            progress,
        )
        .await?;
        let staged = download.into_staged()?;
        let staged_path: std::path::PathBuf = staged.path.to_path_buf();
        let actual_sha1 = tokio::task::spawn_blocking(move || -> std::io::Result<String> {
            use std::io::Read;
            let mut file = std::fs::File::open(&staged_path)?;
            let mut hasher = sha1_smol::Sha1::new();
            let mut buffer = vec![0u8; 262144];
            loop {
                let read = file.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
            }
            Ok(hasher.digest().to_string())
        })
        .await
        .map_err(|error| {
            ErrorKind::ContentStoreError(format!("failed to hash download: {error}"))
        })??;
        if !actual_sha1.eq_ignore_ascii_case(sha1) {
            bail!(ErrorKind::ContentStoreError(format!(
                "downloaded content SHA-1 mismatch: expected {sha1}, got {actual_sha1}",
            )));
        }
        if let Some(expected_size) = size
            && expected_size != staged.size
        {
            tracing::warn!(
                expected_size,
                actual_size = staged.size,
                "Downloaded content matches SHA-1 but differs from the declared size; using its actual size"
            );
        }
        let sources = mirrors
            .iter()
            .filter_map(|source| {
                let url = url::Url::parse(source).ok()?;
                (url.scheme() == "https"
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none())
                .then(|| source.to_string())
            })
            .collect::<Vec<_>>();
        let stored_file = self.save_staged_file(staged, &sources).await?;
        Ok(GetFileResult {
            stored_file,
            reused: false,
        })
    }

    pub(crate) async fn store_file(
        &self,
        source: &Path,
    ) -> crate::Result<StoredFileHandle> {
        if let Some(stored_file) = self.find_file_by_path(source).await? {
            return Ok(stored_file);
        }
        let hashes = hash_file(source).await?;
        if let Some(stored_file) =
            self.lookup(Some(&hashes.sha512), Some(hashes.size)).await?
        {
            return Ok(stored_file);
        }
        let staged = self.stage_file(source).await?;
        self.save_staged_file(staged, &[]).await
    }

    pub(in crate::state::content_store) async fn save_staged_file(
        &self,
        staged: fetch::StagedDownload,
        sources: &[String],
    ) -> crate::Result<StoredFileHandle> {
        if staged.path.parent() != Some(self.staging.as_path()) {
            bail!(ErrorKind::ContentStoreError(
                "content was not staged in the shared store".into()
            ));
        }
        let fetch::StagedDownload {
            path: temporary,
            size,
            sha512: hash,
        } = staged;
        let guard = self.lease().await;
        let _publication = self.publish_lock(&hash).lock().await;
        if let Some(existing) = self
            .lookup_with_guard(Some(&hash), Some(size), guard.clone())
            .await?
        {
            return Ok(existing);
        }
        let destination = self
            .root
            .join(crate::state::content_store::object_relative_path(&hash)?);
        filesystem::publish_staged_file(
            &self.root,
            &destination,
            temporary,
            &hash,
        )
        .await?;
        let stored_file = StoredFileMetadata {
            sha512: hash,
            size: size.try_into().context(ErrorKind::ContentStoreError(
                "content file is too large".into(),
            ))?,
            status: StoredFileStatus::Ready,
            modified_as: filesystem::modified_at_ns(&destination).await?,
            last_used_at: chrono::Utc::now().timestamp(),
            sources: catalog::encode_sources(sources)?,
        };
        catalog::save_file(&self.pool, &stored_file).await?;
        Ok(StoredFileHandle {
            metadata: stored_file,
            path: destination,
            _guard: guard,
        })
    }

    pub(crate) async fn store_bytes(
        &self,
        bytes: &[u8],
    ) -> crate::Result<StoredFileHandle> {
        let (mut file, temporary) = self.temporary().await?;
        filesystem::write_staged_bytes(&mut file, bytes).await?;
        drop(file);
        self.save_staged_file(
            fetch::StagedDownload {
                path: temporary,
                size: bytes.len() as u64,
                sha512: crate::util::content_hash::encode_hex(
                    Sha512::digest(bytes).as_slice(),
                ),
            },
            &[],
        )
        .await
    }

    pub(crate) async fn temporary(
        &self,
    ) -> crate::Result<(File, tempfile::TempPath)> {
        temporary_file(Some(&self.staging)).await
    }
    pub(crate) fn require_staging_space(&self, size: u64) -> crate::Result<()> {
        filesystem::require_staging_space(&self.staging, size)
    }

    pub(in crate::state::content_store) async fn stage_file(
        &self,
        source: &Path,
    ) -> crate::Result<fetch::StagedDownload> {
        filesystem::stage_file(&self.staging, source).await
    }
}

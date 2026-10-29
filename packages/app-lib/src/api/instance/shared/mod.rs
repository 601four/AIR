use crate::SharedInstanceUnavailableReason;
use crate::event::InstancePayloadType;
use crate::event::emit::emit_instance;
use crate::install::{
    InstallJobSnapshot, SharedInstanceExternalFileData,
    SharedInstanceInstallData,
};
use crate::state::instances::{InstanceLink, SharedInstanceAttachment};
use crate::state::{
    AppliedContentSetPatch, CacheBehaviour, CachedEntry, ContentSetSyncStatus,
    ContentSourceKind, EditInstance, ModLoader, ModrinthCredentials,
    ProjectType, SharedInstanceRole, State,
};
use crate::util::fetch::{
    INSECURE_NO_TIMEOUT_REQWEST_CLIENT, INSECURE_REQWEST_CLIENT,
    NO_TIMEOUT_REQWEST_CLIENT, REQWEST_CLIENT,
};
use anyhow::{Context, bail};
use chrono::{DateTime, Utc};
use modrinth_content_management::{
    Change, CommonExternalFilePolicy, ConfigurationDiff,
    ContentSetConfiguration, ContentSetDiffEntry, ContentSetDiffKind,
    ContentSetDiffOptions, ContentSetSnapshot, ExternalFileKey,
    LoaderReference, diff_configuration, diff_content_sets,
};
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeSet;
use std::collections::{HashMap, HashSet};
use std::io::Read;

pub(crate) const CONFIG_BUNDLE_FILE_TYPE: &str = "configs";
/// Wire type for an instance file the owner selected to share. The file name
/// is its instance-relative path.
pub(crate) const CONFIG_FILE_TYPE: &str = "config";
pub(crate) const CONFIG_SYNC_ENABLED: bool = true;
pub(crate) const CONFIG_DIRECTORY: &str = "config";
pub(crate) const MAX_SHARED_FILES: usize = 4096;
pub(crate) const MAX_SHARED_FILES_TOTAL_SIZE: u64 = 500 * 1024 * 1024;
pub(crate) const MAX_CONFIG_BUNDLE_ENTRIES: usize = 4096;
pub(crate) const MAX_CONFIG_BUNDLE_FILE_SIZE: u64 = 512 * 1024 * 1024;
pub(crate) const MAX_CONFIG_BUNDLE_TOTAL_SIZE: u64 = 512 * 1024 * 1024;
pub(crate) const CONFIG_FILE_EXTENSIONS: [&str; 14] = [
    "json",
    "json5",
    "jsonc",
    "yml",
    "yaml",
    "css",
    "toml",
    "txt",
    "ini",
    "cfg",
    "conf",
    "properties",
    "xml",
    "nbt",
];

/// Extensions of executables, scripts and installers. These are hidden from
/// the publish file selection and never installed; the shared instances
/// backend also rejects them by content.
const EXECUTABLE_FILE_EXTENSIONS: &[&str] = &[
    "exe", "dll", "msi", "msix", "com", "scr", "cpl", "sys", "bat", "cmd",
    "ps1", "psm1", "vbs", "vbe", "jse", "wsf", "wsh", "hta", "lnk", "reg",
    "sh", "bash", "zsh", "command", "app", "dylib", "so", "appimage", "apk",
    "deb", "rpm", "dmg", "pkg", "class",
];

pub(crate) fn is_supported_config_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            CONFIG_FILE_EXTENSIONS
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate))
        })
}

/// Whether members can install a shared file at this instance-relative path.
/// Content folders are shared through the content set instead.
pub(crate) fn is_shareable_file_path(path: &str) -> bool {
    crate::state::content_store::validate_relative(path).is_ok()
        && is_shareable_path(path)
        && !is_executable_file(path)
        && path
            .split_once('/')
            .is_none_or(|(root, _)| !is_content_folder(root))
}

fn is_executable_file(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            EXECUTABLE_FILE_EXTENSIONS
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate))
        })
}

/// Leading bytes of each file passed to [`is_executable_content`].
pub(super) const FILE_SCAN_HEAD_SIZE: usize = 8192;

/// Mirrors the upload scan of the shared instances backend, so files it would
/// reject fail before a version is created.
pub(super) fn is_executable_content(head: &[u8]) -> bool {
    infer::get(head).is_some_and(|kind| match kind.matcher_type() {
        infer::MatcherType::App => {
            !matches!(kind.extension(), "der" | "pem" | "obj" | "bc")
        }
        _ => matches!(
            kind.extension(),
            "sh" | "msi" | "cab" | "deb" | "rpm" | "crx"
        ),
    })
}

/// Whether a directory can hold shareable files, so the rest can be skipped
/// while walking the instance.
pub(super) fn is_shareable_directory(path: &str) -> bool {
    is_shareable_path(path) && !is_content_folder(path)
}

fn is_shareable_path(path: &str) -> bool {
    !path.split('/').any(|part| part.starts_with('.'))
        && super::export_mrpack::is_exportable_path(path)
}

fn is_content_folder(path: &str) -> bool {
    ProjectType::iterator()
        .any(|project_type| project_type.get_folder() == path)
}

pub(crate) fn read_bounded_config_bundle_entry(
    reader: impl Read,
    declared_size: u64,
    total_size: &mut u64,
) -> crate::Result<Vec<u8>> {
    let remaining = MAX_CONFIG_BUNDLE_TOTAL_SIZE.saturating_sub(*total_size);
    let entry_limit = MAX_CONFIG_BUNDLE_FILE_SIZE.min(remaining);
    if declared_size > entry_limit {
        bail!(crate::ErrorKind::InputError(
            "shared instance config bundle exceeds the uncompressed size limit"
                .to_string(),
        ));
    }

    let capacity = usize::try_from(declared_size).context(
        crate::ErrorKind::InputError(
            "shared instance config bundle entry is too large".to_string(),
        ),
    )?;
    let mut bytes = Vec::with_capacity(capacity);
    reader
        .take(entry_limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .context("failed to read shared instance config bundle")?;
    let actual_size = bytes.len() as u64;
    if actual_size > entry_limit {
        bail!(crate::ErrorKind::InputError(
            "shared instance config bundle exceeds the uncompressed size limit"
                .to_string(),
        ));
    }
    *total_size = total_size.checked_add(actual_size).ok_or(
        crate::ErrorKind::InputError(
            "shared instance config bundle size overflowed".to_string(),
        ),
    )?;

    Ok(bytes)
}


mod types;

pub use self::types::{
    SharedInstanceExternalFilePreview, SharedInstanceInstallPreview,
    SharedInstanceInvite, SharedInstanceInviteInstallPreview,
    SharedInstanceInviteLink, SharedInstanceJoinType,
    SharedInstancePublishFile, SharedInstancePublishPreview,
    SharedInstanceUpdateDiff, SharedInstanceUpdateDiffType,
    SharedInstanceUpdatePreview, SharedInstanceUser, SharedInstanceUsers,
};

// 共享实例功能已移除：下面这些函数保留签名，但一律"什么都不做"。
// 上层调用不会报错，也不会再产生任何网络请求或数据库操作。

fn disabled() -> crate::Error {
    crate::ErrorKind::InputError("共享实例功能已移除".to_string()).into()
}

pub async fn can_active_user_use_shared_instances() -> crate::Result<bool> {
    Ok(false)
}

pub(crate) async fn check_shared_instance_availability_before_launch(
    _instance_id: &str,
    _state: &crate::State,
) -> crate::Result<()> {
    Ok(())
}

pub(crate) async fn sync_shared_instance_icon(
    _instance_id: &str,
    _icon_path: Option<&str>,
    _state: &crate::State,
) -> crate::Result<()> {
    Ok(())
}

pub async fn get_shared_instance_users(
    _instance_id: &str,
) -> crate::Result<SharedInstanceUsers> {
    Ok(SharedInstanceUsers::empty())
}

pub async fn invite_shared_instance_users(
    _instance_id: &str,
    _user_ids: Vec<String>,
) -> crate::Result<SharedInstanceUsers> {
    Err(disabled())
}

pub async fn create_shared_instance_invite_link(
    _instance_id: &str,
    _max_age_seconds: Option<i32>,
    _max_uses: Option<i32>,
    _replace_invite_id: Option<String>,
) -> crate::Result<SharedInstanceInviteLink> {
    Err(disabled())
}

pub async fn get_shared_instance_invites(
    _instance_id: &str,
) -> crate::Result<Vec<SharedInstanceInvite>> {
    Ok(Vec::new())
}

pub async fn revoke_shared_instance_invite(
    _instance_id: &str,
    _invite_id: String,
) -> crate::Result<()> {
    Ok(())
}

pub async fn remove_shared_instance_users(
    _instance_id: &str,
    _user_ids: Vec<String>,
    _has_pending_recipients: bool,
) -> crate::Result<SharedInstanceUsers> {
    Err(disabled())
}

pub async fn accept_pending_shared_instance_invite(
    _shared_instance_id: &str,
) -> crate::Result<()> {
    Ok(())
}

pub async fn decline_pending_shared_instance_invite(
    _shared_instance_id: &str,
) -> crate::Result<()> {
    Ok(())
}

pub async fn install_shared_instance(
    _shared_instance_id: &str,
    _name: String,
    _manager_id: Option<String>,
    _server_manager_name: Option<String>,
    _server_manager_icon_url: Option<String>,
    _instance_icon_url: Option<String>,
) -> crate::Result<crate::install::InstallJobSnapshot> {
    Err(disabled())
}

pub async fn get_shared_instance_install_preview(
    _shared_instance_id: &str,
    _name: String,
) -> crate::Result<SharedInstanceInstallPreview> {
    Err(disabled())
}

pub async fn accept_shared_instance_invite_for_install(
    _invite_id: &str,
) -> crate::Result<SharedInstanceInviteInstallPreview> {
    Err(disabled())
}

pub async fn get_shared_instance_update_preview(
    _instance_id: &str,
) -> crate::Result<Option<SharedInstanceUpdatePreview>> {
    Ok(None)
}

pub async fn update_shared_instance(
    _instance_id: &str,
) -> crate::Result<crate::install::InstallJobSnapshot> {
    Err(disabled())
}

pub async fn unpublish_shared_instance(_instance_id: &str) -> crate::Result<()> {
    Ok(())
}

pub async fn unlink_shared_instance(_instance_id: &str) -> crate::Result<()> {
    Ok(())
}

pub async fn publish_shared_instance(
    _instance_id: &str,
    _file_paths: Vec<String>,
    _on_progress: &(dyn Fn(u64, u64) + Send + Sync),
) -> crate::Result<crate::state::instances::SharedInstanceAttachment> {
    Err(disabled())
}

pub async fn get_shared_instance_publish_preview(
    _instance_id: &str,
) -> crate::Result<Option<SharedInstancePublishPreview>> {
    Ok(None)
}

pub async fn get_shared_instance_publish_support_details(
    _instance_id: &str,
) -> crate::Result<Option<String>> {
    Ok(None)
}

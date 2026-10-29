//! 账号数据目录（可迁移）
//!
//! 默认：<data_dir>/<app_identifier>   例如 %APPDATA%\\com.aircore.launcher
//! 可选：在默认目录下放一个 data-location.json：
//!       { "path": "D:\\AirCoreData" }
//! 下次启动时：先把默认目录的数据复制到新目录（仅当新目录还没有 app.db），
//! 然后整个启动器都使用新目录（app.db、插件、日志、窗口状态）。

use std::path::{Path, PathBuf};

use serde_json::json;

pub const DATA_LOCATION_FILE: &str = "data-location.json";

/// 默认数据目录（固定，受系统账号目录影响）
pub fn default_dir(app_identifier: &str) -> Option<PathBuf> {
    Some(dirs::data_dir()?.join(app_identifier))
}

/// 用户设置的数据目录（未设置或无效时返回 None）
pub fn configured_dir(app_identifier: &str) -> Option<PathBuf> {
    let default = default_dir(app_identifier)?;
    let pointer = default.join(DATA_LOCATION_FILE);
    let text = std::fs::read_to_string(pointer).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let path = value.get("path")?.as_str()?.trim();
    if path.is_empty() {
        return None;
    }
    let dir = PathBuf::from(path);
    if dir == default {
        return None;
    }
    Some(dir)
}

/// 当前生效的数据目录
pub fn effective_dir(app_identifier: &str) -> Option<PathBuf> {
    configured_dir(app_identifier).or_else(|| default_dir(app_identifier))
}

/// 设置（或清除）数据目录；None 表示恢复默认
pub fn set_configured_dir(
    app_identifier: &str,
    path: Option<String>,
) -> crate::Result<()> {
    let default = default_dir(app_identifier).ok_or_else(|| {
        crate::ErrorKind::FSError("could not find valid data dir".to_string())
    })?;
    std::fs::create_dir_all(&default)?;
    let pointer = default.join(DATA_LOCATION_FILE);

    match path {
        Some(path) => {
            let path = path.trim().to_string();
            if path.is_empty() {
                let _ = std::fs::remove_file(&pointer);
                return Ok(());
            }
            let dir = PathBuf::from(&path);
            if dir == default {
                let _ = std::fs::remove_file(&pointer);
                return Ok(());
            }
            std::fs::create_dir_all(&dir)?;
            std::fs::write(&pointer, serde_json::to_vec_pretty(&json!({ "path": path }))?)?;
            tracing::info!("Configured data directory: {}", dir.display());
        }
        None => {
            let _ = std::fs::remove_file(&pointer);
            tracing::info!("Reset data directory to default");
        }
    }
    Ok(())
}

/// 启动时调用：如果设置了新数据目录且里面还没数据，先复制一份过去
pub async fn migrate_if_needed(app_identifier: &str) -> crate::Result<()> {
    let Some(configured) = configured_dir(app_identifier) else {
        return Ok(());
    };
    let Some(default) = default_dir(app_identifier) else {
        return Ok(());
    };
    if configured.join("app.db").exists() {
        return Ok(());
    }
    if !default.join("app.db").exists() {
        // 默认目录也没数据（首次启动），不需要复制
        return Ok(());
    }

    tracing::info!(
        "Migrating launcher data from {} to {}",
        default.display(),
        configured.display()
    );
    let default_clone = default.clone();
    let configured_clone = configured.clone();
    tokio::task::spawn_blocking(move || copy_dir_contents(&default_clone, &configured_clone))
        .await??;
    Ok(())
}

fn copy_dir_contents(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        // 不复制位置指向文件和运行锁
        if name_str == DATA_LOCATION_FILE || name_str == "store.lock" {
            continue;
        }
        let source = entry.path();
        let target = to.join(&name);
        if source.is_dir() {
            copy_dir_contents(&source, &target)?;
        } else if !target.exists() {
            std::fs::copy(&source, &target)?;
        }
    }
    Ok(())
}

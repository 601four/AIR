use serde::{Deserialize, Serialize};
use tauri::{Manager, Runtime};
use tauri_plugin_opener::OpenerExt;
use theseus::{
    emit_warning, handler,
    prelude::{CommandPayload, DirectoryInfo, app_db_backup_dir},
};

use crate::api::Result;
use crate::api::oauth_utils::auth_code_reply::{
    normalize_deep_link_command, parse_auth_deeplink, submit_deeplink,
};
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use theseus::prelude::canonicalize;
use tokio_util::sync::CancellationToken;
use url::Url;

#[derive(Default)]
pub struct DebugInfoExports(DashMap<(String, String), CancellationToken>);

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("utils")
        .setup(|app, _| {
            app.manage(DebugInfoExports::default());
            Ok(())
        })
        .on_event(|app, event| {
            if let tauri::RunEvent::WindowEvent {
                label,
                event: tauri::WindowEvent::Destroyed,
                ..
            } = event
            {
                for export in &app.state::<DebugInfoExports>().0 {
                    if &export.key().0 == label {
                        export.value().cancel();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_os,
            is_network_metered,
            should_disable_mouseover,
            highlight_in_folder,
            open_path,
            show_launcher_logs_folder,
            show_app_db_backups_folder,
            export_debug_info,
            cancel_debug_info_export,
            progress_bars_list,
            get_opening_command,
            super::thumbnails::get_image_thumbnail,
            optimize_launcher_memory,
            set_download_source,
            set_loader_source,
            get_loader_source,
            prefs_get_all,
            debug_log_append,
            get_data_dir,
            set_data_dir,
            prefs_set,
            translate_text,
])
        .build()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::enum_variant_names)]
pub enum OS {
    Windows,
    Linux,
    MacOS,
}

/// Gets OS
#[tauri::command]
pub fn get_os() -> OS {
    #[cfg(target_os = "windows")]
    let os = OS::Windows;
    #[cfg(target_os = "linux")]
    let os = OS::Linux;
    #[cfg(target_os = "macos")]
    let os = OS::MacOS;
    os
}

#[tauri::command]
pub async fn is_network_metered() -> Result<bool> {
    Ok(theseus::prelude::is_network_metered().await?)
}

// Lists active progress bars
// Create a new HashMap with the same keys
// Values provided should not be used directly, as they are not guaranteed to be up-to-date
#[tauri::command]
pub async fn progress_bars_list()
-> Result<DashMap<uuid::Uuid, theseus::LoadingBar>> {
    let res = theseus::EventState::list_progress_bars().await?;
    Ok(res)
}

// disables mouseover and fixes a random crash error only fixed by recent versions of macos
#[tauri::command]
pub async fn should_disable_mouseover() -> bool {
    if cfg!(target_os = "macos") {
        // We try to match version to 12.2 or higher. If unrecognizable to pattern or lower, we default to the css with disabled mouseover for safety
        if let tauri_plugin_os::Version::Semantic(major, minor, _) =
            tauri_plugin_os::version()
            && major >= 12
            && minor >= 3
        {
            // Mac os version is 12.3 or higher, we allow mouseover
            return false;
        }
        true
    } else {
        // Not macos, we allow mouseover
        false
    }
}

#[tauri::command]
pub async fn highlight_in_folder<R: Runtime>(
    app: tauri::AppHandle<R>,
    path: PathBuf,
) {
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(e) = app.opener().reveal_item_in_dir(path) {
            tracing::error!("Failed to highlight file in folder: {}", e);
        }
    })
    .await
    .ok();
}

#[tauri::command]
pub async fn open_path<R: Runtime>(app: tauri::AppHandle<R>, path: PathBuf) {
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(e) =
            app.opener().open_path(path.to_string_lossy(), None::<&str>)
        {
            tracing::error!("Failed to open path: {}", e);
        }
    })
    .await
    .ok();
}

#[tauri::command]
pub async fn show_launcher_logs_folder<R: Runtime>(app: tauri::AppHandle<R>) {
    if let Some(d) = DirectoryInfo::global_handle_if_ready() {
        let path = d.launcher_logs_dir().unwrap_or_default();
        // failure to get folder just opens filesystem
        // (ie: if in debug mode only and launcher_logs never created)
        open_path(app, path).await;
    }
}

#[tauri::command]
pub async fn show_app_db_backups_folder<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<()> {
    let path = app_db_backup_dir()?;
    tokio::fs::create_dir_all(&path).await?;
    open_path(app, path).await;
    Ok(())
}

#[tauri::command]
pub async fn export_debug_info<R: Runtime>(
    window: tauri::Window<R>,
    state: tauri::State<'_, DebugInfoExports>,
    id: String,
    path: PathBuf,
    on_progress: tauri::ipc::Channel<
        theseus::debug_info::DebugInfoExportProgress,
    >,
) -> Result<bool> {
    let key = (window.label().to_string(), id);
    let cancel = CancellationToken::new();
    match state.0.entry(key.clone()) {
        dashmap::mapref::entry::Entry::Vacant(entry) => {
            entry.insert(cancel.clone());
        }
        dashmap::mapref::entry::Entry::Occupied(_) => {
            return Err(std::io::Error::other(
                "Debug info export is already running",
            )
            .into());
        }
    }
    let result =
        theseus::debug_info::export_debug_info(path, cancel, move |progress| {
            let _ = on_progress.send(progress);
        })
        .await;
    state.0.remove(&key);
    Ok(result?)
}

#[tauri::command]
pub fn cancel_debug_info_export<R: Runtime>(
    window: tauri::Window<R>,
    state: tauri::State<'_, DebugInfoExports>,
    id: String,
) {
    if let Some(cancel) = state.0.get(&(window.label().to_string(), id)) {
        cancel.cancel();
    }
}

// Get opening command
// For example, if a user clicks on an .mrpack to open the app.
// This should be called once and only when the app is done booting up and ready to receive a command
// Returns a Command struct- see events.js
#[tauri::command]
#[cfg(target_os = "macos")]
pub async fn get_opening_command(
    state: tauri::State<'_, crate::macos::deep_link::InitialPayload>,
) -> Result<Option<CommandPayload>> {
    let payload = state.payload.lock().await;
    let cmd_arg = std::env::args_os()
        .nth(1)
        .map(|path| path.to_string_lossy().to_string());

    return if let Some(payload) = payload.as_ref() {
        opening_command(payload).await
    } else if let Some(cmd_arg) = cmd_arg {
        opening_command(&cmd_arg).await
    } else {
        Ok(None)
    };
}

#[tauri::command]
#[cfg(not(target_os = "macos"))]
pub async fn get_opening_command() -> Result<Option<CommandPayload>> {
    // Tauri is not CLI, we use arguments as path to file to call
    let cmd_arg = std::env::args_os().nth(1);

    let cmd_arg = cmd_arg.map(|path| path.to_string_lossy().to_string());
    if let Some(cmd) = cmd_arg {
        return opening_command(&cmd).await;
    }
    Ok(None)
}

// helper function called when redirected by a weblink (ie: modrith://do-something) or when redirected by a .mrpack file (in which case its a filepath)
// We hijack the deep link library (which also contains functionality for instance-checking)
pub async fn handle_command(command: String) -> Result<()> {
    let command = normalize_deep_link_command(&command);
    if accept_auth_deeplink(&command).await {
        return Ok(());
    }

    tracing::info!("handle command: {command}");
    Ok(theseus::handler::parse_and_emit_command(&command).await?)
}

async fn opening_command(command: &str) -> Result<Option<CommandPayload>> {
    let command = normalize_deep_link_command(command);
    if accept_auth_deeplink(&command).await {
        return Ok(None);
    }

    tracing::info!("opening command {command}");
    Ok(Some(handler::parse_command(&command).await?))
}

async fn accept_auth_deeplink(command: &str) -> bool {
    let Some((code, nonce)) = parse_auth_deeplink(command) else {
        return false;
    };

    tracing::info!("Handling Modrinth auth deep link");
    if !submit_deeplink(code, &nonce) {
        const MESSAGE: &str = "Couldn't finish signing in. Please try again.";
        if let Err(error) = emit_warning(MESSAGE).await {
            tracing::error!("{MESSAGE} ({error})");
        }
    }
    true
}

// Remove when (and if) https://github.com/tauri-apps/tauri/issues/12022 is implemented
pub(crate) fn tauri_convert_file_src(path: &Path) -> Result<Url> {
    #[cfg(any(windows, target_os = "android"))]
    const BASE: &str = "http://asset.localhost/";
    #[cfg(not(any(windows, target_os = "android")))]
    const BASE: &str = "asset://localhost/";

    let path = canonicalize(path)?;
    let path = path.to_string_lossy();
    let encoded = urlencoding::encode(&path);

    Ok(Url::parse(&format!("{BASE}{encoded}"))?)
}

/// 尝试释放启动器自身占用的工作集（Windows 上调用 SetProcessWorkingSetSize）。
/// 这不会动其他进程，只让系统回收启动器的冷页，为游戏让出内存。
#[tauri::command]
pub fn optimize_launcher_memory() -> serde_json::Value {
    let result = crate::api::memory_optimize::optimize_system_memory();
    serde_json::to_value(result).unwrap_or_else(|_| serde_json::json!({}))
}
/// 读取 AirCore 自定义偏好（强调色 / 内存开关 / 下载源）
#[tauri::command]
pub async fn prefs_get_all() -> std::result::Result<std::collections::HashMap<String, String>, String> {
    theseus::prefs::prefs_get_all()
        .await
        .map_err(|e| e.to_string())
}

/// 写入一个 AirCore 偏好
#[tauri::command]
pub async fn prefs_set(key: String, value: String) -> std::result::Result<(), String> {
    theseus::prefs::prefs_set(key, value)
        .await
        .map_err(|e| e.to_string())
}

/// 把前端控制台信息追加到文件，便于排查白屏 / 报错
#[tauri::command]
pub fn debug_log_append<R: tauri::Runtime>(app: tauri::AppHandle<R>, lines: String) -> bool {
    use std::io::Write;
    use tauri::Manager;
    let identifier = app.config().identifier.clone();
    let Some(base) = theseus::data_dir::effective_dir(&identifier)
        .or_else(|| app.path().app_data_dir().ok())
    else {
        return false;
    };
    let log_dir = base.join("logs");
    if std::fs::create_dir_all(&log_dir).is_err() {
        return false;
    }
    let path = log_dir.join("webview.log");
    match std::fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut file) => file.write_all(lines.as_bytes()).is_ok(),
        Err(_) => false,
    }
}

/// 读取当前账号数据目录（app.db / 插件 / 日志 所在位置）
#[tauri::command]
pub fn get_data_dir<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> std::result::Result<serde_json::Value, String> {
    use tauri::Manager;
    let identifier = app.config().identifier.clone();
    let default_dir = theseus::data_dir::default_dir(&identifier)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let configured = theseus::data_dir::configured_dir(&identifier)
        .map(|p| p.to_string_lossy().into_owned());
    let effective = theseus::data_dir::effective_dir(&identifier)
        .or_else(|| app.path().app_data_dir().ok())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(serde_json::json!({
        "default": default_dir,
        "configured": configured,
        "effective": effective,
    }))
}

/// 设置账号数据目录：path = None 表示恢复默认；重启后生效
#[tauri::command]
pub fn set_data_dir<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    path: Option<String>,
) -> std::result::Result<(), String> {
    let identifier = app.config().identifier.clone();
    theseus::data_dir::set_configured_dir(&identifier, path).map_err(|e| e.to_string())
}

/// 设置加载器下载源：mode = official | cn | custom
#[tauri::command]
pub fn set_loader_source(mode: Option<String>, prefix: Option<String>) -> bool {
    let mode = match mode.as_deref() {
        Some("official") => theseus::util::mirror::LoaderSource::Official,
        Some("custom") => theseus::util::mirror::LoaderSource::Custom,
        _ => theseus::util::mirror::LoaderSource::Cn,
    };
    theseus::util::mirror::set_loader_source(mode, prefix);
    true
}

/// 读取当前加载器下载源
#[tauri::command]
pub fn get_loader_source() -> serde_json::Value {
    let mode = match theseus::util::mirror::current_loader_mode() {
        theseus::util::mirror::LoaderSource::Official => "official",
        theseus::util::mirror::LoaderSource::Cn => "cn",
        theseus::util::mirror::LoaderSource::Custom => "custom",
    };
    serde_json::json!({
        "mode": mode,
        "prefix": theseus::util::mirror::current_loader_prefix(),
    })
}

/// 切换 Minecraft 下载源："official" | "bmclapi"
#[tauri::command]
pub fn set_download_source(source: String) -> bool {
	use theseus::util::mirror::{set_source, DownloadSource};
	let parsed = match source.as_str() {
		"bmclapi" => DownloadSource::Bmclapi,
		_ => DownloadSource::Official,
	};
	set_source(parsed);
	true
}


/// 从字符串中取出 marker 之后、下一个引号之前的内容
fn extract_between(haystack: &str, marker: &str) -> Option<String> {
    let start = haystack.find(marker)? + marker.len();
    let rest = &haystack[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// 使用必应（国际版翻译器的免费页面接口）翻译
async fn try_bing(client: &reqwest::Client, text: &str) -> Option<String> {
    let page = client
        .get("https://cn.bing.com/translator")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;

    let ig = extract_between(&page, "IG:\"")?;
    let iid = extract_between(&page, "data-iid=\"translator.")?;
    let token = extract_between(&page, "params_AbusePreventionHelper = [")?;
    // 上面拿到的是时间戳，再取紧随的 token 字符串
    let after_ts = page.find("params_AbusePreventionHelper = [")?;
    let slice = &page[after_ts..];
    let comma = slice.find(",\"")? + 2;
    let token_rest = &slice[comma..];
    let token_end = token_rest.find('"')?;
    let abuse_token = &token_rest[..token_end];
    let timestamp = token.trim_end_matches(',').trim_matches('"').to_string();

    let url = format!(
        "https://cn.bing.com/ttranslatev3?isVertical=1&&IG={ig}&IID=translator.{iid}.1"
    );

    let params = [
        ("fromLang", "auto-detect".to_string()),
        ("text", text.to_string()),
        ("to", "zh-Hans".to_string()),
        ("token", abuse_token.to_string()),
        ("key", timestamp),
    ];

    let response = client.post(&url).form(&params).send().await.ok()?;
    let value: serde_json::Value = response.json().await.ok()?;
    let translated = value
        .get(0)?
        .get("translations")?
        .get(0)?
        .get("text")?
        .as_str()?
        .to_string();
    if translated.trim().is_empty() {
        None
    } else {
        Some(translated)
    }
}


/// Lingva（开源 Google 代理，国内可访问）
async fn try_lingva(client: &reqwest::Client, text: &str) -> Option<String> {
    let encoded: String = urlencoding_encode(text);
    for host in ["https://lingva.ml", "https://lingva.garudalinux.org"] {
        let url = format!("{host}/api/v1/auto/zh/{encoded}");
        if let Ok(response) = client.get(&url).send().await {
            if response.status().is_success() {
                if let Ok(value) = response.json::<serde_json::Value>().await {
                    if let Some(translation) = value.get("translation").and_then(|v| v.as_str()) {
                        if !translation.trim().is_empty() {
                            return Some(translation.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

/// 简易 URL 编码（避免引入额外依赖）
fn urlencoding_encode(input: &str) -> String {
    let mut out = String::new();
    for byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// 翻译文本：必应 → Lingva → Google → MyMemory → 失败。
#[tauri::command]
pub async fn translate_text(
	text: String,
	target: Option<String>,
) -> std::result::Result<String, String> {
	translate_text_inner(text, target)
		.await
		.map_err(|e| e.to_string())
}

async fn translate_text_inner(text: String, target: Option<String>) -> anyhow::Result<String> {
    let target = target.unwrap_or_else(|| "zh-CN".to_string());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| anyhow::anyhow!(e))?;

    // 0) 必应免费页面接口
    if let Some(translated) = try_bing(&client, &text).await {
        return Ok(translated);
    }

    // 1) Lingva（开源代理，多个镜像）
    if let Some(translated) = try_lingva(&client, &text).await {
        return Ok(translated);
    }

    // 2) Google 非官方 gtx 接口
    if let Ok(response) = client
        .get("https://translate.googleapis.com/translate_a/single")
        .query(&[
            ("client", "gtx"),
            ("sl", "auto"),
            ("tl", target.as_str()),
            ("dt", "t"),
            ("q", text.as_str()),
        ])
        .send()
        .await
    {
        if response.status().is_success() {
            if let Ok(value) = response.json::<serde_json::Value>().await {
                if let Some(chunks) = value.get(0).and_then(|v| v.as_array()) {
                    let mut translated = String::new();
                    for chunk in chunks {
                        if let Some(part) = chunk.get(0).and_then(|v| v.as_str()) {
                            translated.push_str(part);
                        }
                    }
                    if !translated.trim().is_empty() {
                        return Ok(translated);
                    }
                }
            }
        }
    }

    // 3) MyMemory 备选
    let response = client
        .get("https://api.mymemory.translated.net/get")
        .query(&[("q", text.as_str()), ("langpair", "en|zh-CN")])
        .send()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    let value: serde_json::Value = response.json().await.map_err(|e| anyhow::anyhow!(e))?;
    value
        .get("responseData")
        .and_then(|v| v.get("translatedText"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("翻译失败"))
}

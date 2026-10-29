use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::api::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionManifest {
	/// 必填：且必须与 zip 文件名一致
	pub id: String,
	pub name: String,
	#[serde(default)]
	pub description: Option<String>,
	#[serde(default = "default_entry")]
	pub entry: String,
	#[serde(default)]
	pub permissions: Vec<String>,
	#[serde(default)]
	pub icon: Option<String>,
	#[serde(default = "default_api_version", alias = "apiVersion")]
	pub api_version: u32,
	#[serde(default = "default_true")]
	pub enabled: bool,
}

fn default_api_version() -> u32 {
	1
}

fn default_true() -> bool {
	true
}

fn default_entry() -> String {
	"index.html".to_string()
}

/// 扩展目录：<app_data>/extensions/
pub fn extensions_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Option<PathBuf> {
	use tauri::Manager;
	app.path().app_data_dir().ok().map(|dir| dir.join("extensions"))
}

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
	tauri::plugin::Builder::new("extensions")
		.invoke_handler(tauri::generate_handler![
			extensions_list,
			extensions_diagnostics,
			plugin_net_fetch,
			extensions_read_binary,


			extensions_read,
			extensions_remove,
			extensions_set_enabled,
			extensions_versions,
			extensions_storage_get,
			extensions_storage_set,
		])
		.build()
}

/// 插件来源：文件夹或 zip 包
/// 插件来源：只支持 zip 包
#[derive(Debug, Clone)]
struct PluginSource {
	zip: PathBuf,
}

impl PluginSource {
	fn disabled_marker(&self) -> PathBuf {
		self.zip.with_extension("disabled")
	}

	fn data_file(&self) -> PathBuf {
		self.zip.with_extension("data.json")
	}
}

/// 根据插件 id 找到它的来源（文件夹优先，其次 <id>.zip）
fn resolve_source(dir: &Path, id: &str) -> Option<PluginSource> {
	if id.contains('/') || id.contains('\\') || id.contains("..") {
		return None;
	}
	let as_zip = dir.join(format!("{id}.zip"));
	if as_zip.is_file() {
		return Some(PluginSource { zip: as_zip });
	}
	None
}

fn read_zip_text(zip: &Path, file: &str) -> Option<String> {
	let file_handle = std::fs::File::open(zip).ok()?;
	let mut archive = zip::ZipArchive::new(file_handle).ok()?;
	let target = file.trim_start_matches('/');
	for index in 0..archive.len() {
		let Ok(mut entry) = archive.by_index(index) else {
			continue;
		};
		let Ok(name) = entry.name().map(|n| n.to_string()) else {
			continue;
		};
		let matches = name == target || name.ends_with(&format!("/{target}"));
		if !matches {
			continue;
		}
		let mut buffer = String::new();
		if entry.read_to_string(&mut buffer).is_ok() {
			return Some(buffer);
		}
	}
	None
}


/// 列出所有已安装的插件（文件夹 + zip 包）
#[tauri::command]
pub async fn extensions_list<R: tauri::Runtime>(
	app: tauri::AppHandle<R>,
) -> Result<Vec<ExtensionManifest>> {
	let Some(dir) = extensions_dir(&app) else {
		return Ok(Vec::new());
	};
	let Ok(entries) = std::fs::read_dir(&dir) else {
		return Ok(Vec::new());
	};

	let mut result = Vec::new();
	for entry in entries.flatten() {
		let path = entry.path();

		// 仅支持 zip 插件
		if path.extension().and_then(|e| e.to_str()) == Some("zip") {
			let Some(content) = read_zip_text(&path, "manifest.json") else {
				continue;
			};
			let content = content.trim_start_matches('\u{feff}');
			let Ok(mut manifest) = serde_json::from_str::<ExtensionManifest>(content) else {
				continue;
			};
			// id 必填，且必须与 zip 文件名一致
			let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
				continue;
			};
			if manifest.id.is_empty() || manifest.id != stem {
				continue;
			}
			if path.with_extension("disabled").exists() {
				manifest.enabled = false;
			}
			result.push(manifest);
		}
	}
	result.sort_by(|a, b| a.name.cmp(&b.name));
	Ok(result)
}

/// ext:// 协议读取（文件夹插件与 zip 插件都支持）
pub fn serve_extension<R: tauri::Runtime>(
	app: &tauri::AppHandle<R>,
	id: &str,
	path: &str,
) -> Option<(Vec<u8>, String)> {
	let dir = extensions_dir(app)?;
	let clean = path.trim_start_matches('/');
	if clean.split('/').any(|part| part == "..") {
		return None;
	}
	let file = if clean.is_empty() { "index.html" } else { clean };

	let source = resolve_source(&dir, id)?;
	let bytes = {
		let zip_path = source.zip;
		{
			let file_handle = std::fs::File::open(&zip_path).ok()?;
			let mut archive = zip::ZipArchive::new(file_handle).ok()?;
			let mut found: Option<Vec<u8>> = None;
			for index in 0..archive.len() {
				let Ok(mut entry) = archive.by_index(index) else {
					continue;
				};
				let Ok(name) = entry.name().map(|n| n.to_string()) else {
			continue;
		};
				if name == file || name.ends_with(&format!("/{file}")) {
					let mut buffer = Vec::new();
					if entry.read_to_end(&mut buffer).is_ok() {
						found = Some(buffer);
						break;
					}
				}
			}
			found?
		}
	};

	let mime = match Path::new(file).extension().and_then(|e| e.to_str()).unwrap_or("") {
		"html" => "text/html; charset=utf-8",
		"js" | "mjs" => "text/javascript; charset=utf-8",
		"css" => "text/css; charset=utf-8",
		"json" => "application/json; charset=utf-8",
		"svg" => "image/svg+xml",
		"png" => "image/png",
		"jpg" | "jpeg" => "image/jpeg",
		"webp" => "image/webp",
		"woff2" => "font/woff2",
		"wasm" => "application/wasm",
		_ => "application/octet-stream",
	};
	Some((bytes, mime.to_string()))
}

/// 读取插件文件文本
#[tauri::command]
pub async fn extensions_read<R: tauri::Runtime>(
	app: tauri::AppHandle<R>,
	id: String,
	file: String,
) -> Result<String> {
	let Some(dir) = extensions_dir(&app) else {
		return Ok(String::new());
	};
	let Some(source) = resolve_source(&dir, &id) else {
		return Ok(String::new());
	};
	Ok(read_zip_text(&source.zip, &file).unwrap_or_default())
}

/// 启用 / 停用插件
#[tauri::command]
pub async fn extensions_set_enabled<R: tauri::Runtime>(
	app: tauri::AppHandle<R>,
	id: String,
	enabled: bool,
) -> Result<bool> {
	let Some(dir) = extensions_dir(&app) else {
		return Ok(false);
	};
	let Some(source) = resolve_source(&dir, &id) else {
		return Ok(false);
	};
	let marker = source.disabled_marker();
	if enabled {
		let _ = std::fs::remove_file(marker);
	} else {
		let _ = std::fs::write(marker, b"");
	}
	Ok(true)
}

/// 卸载插件（删文件夹或 zip）
#[tauri::command]
pub async fn extensions_remove<R: tauri::Runtime>(
	app: tauri::AppHandle<R>,
	id: String,
) -> Result<bool> {
	let Some(dir) = extensions_dir(&app) else {
		return Ok(false);
	};
	let Some(source) = resolve_source(&dir, &id) else {
		return Ok(false);
	};
	let _ = std::fs::remove_file(source.disabled_marker());
	let _ = std::fs::remove_file(source.data_file());
	std::fs::remove_file(&source.zip).map_err(|e| anyhow::anyhow!(e))?;
	Ok(true)
}

/// 返回每个插件的修改时间（毫秒），用于热重载
#[tauri::command]
pub async fn extensions_versions<R: tauri::Runtime>(
	app: tauri::AppHandle<R>,
) -> Result<Vec<(String, u64)>> {
	let Some(dir) = extensions_dir(&app) else {
		return Ok(Vec::new());
	};
	let Ok(entries) = std::fs::read_dir(&dir) else {
		return Ok(Vec::new());
	};

	let mut result = Vec::new();
	for entry in entries.flatten() {
		let path = entry.path();
		let id = if path.is_dir() {
			entry.file_name().to_string_lossy().to_string()
		} else if path.extension().and_then(|e| e.to_str()) == Some("zip") {
			path.file_stem()
				.map(|s| s.to_string_lossy().to_string())
				.unwrap_or_default()
		} else {
			continue;
		};
		if id.is_empty() {
			continue;
		}

		let mut latest = 0_u64;
		let mut consider = |metadata: std::fs::Metadata| {
			if let Ok(modified) = metadata.modified() {
				if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
					latest = latest.max(duration.as_millis() as u64);
				}
			}
		};

		if path.is_dir() {
			if let Ok(files) = std::fs::read_dir(&path) {
				for file in files.flatten() {
					if let Ok(metadata) = file.metadata() {
						consider(metadata);
					}
				}
			}
		} else if let Ok(metadata) = path.metadata() {
			consider(metadata);
		}

		result.push((id, latest));
	}
	Ok(result)
}

fn resolve_data_file<R: tauri::Runtime>(
	app: &tauri::AppHandle<R>,
	id: &str,
) -> Option<PathBuf> {
	let dir = extensions_dir(app)?;
	let source = resolve_source(&dir, id)?;
	Some(source.data_file())
}

/// 读取插件自己的配置
#[tauri::command]
pub async fn extensions_storage_get<R: tauri::Runtime>(
	app: tauri::AppHandle<R>,
	id: String,
) -> Result<serde_json::Value> {
	let Some(path) = resolve_data_file(&app, &id) else {
		return Ok(serde_json::json!({}));
	};
	let Ok(content) = std::fs::read_to_string(&path) else {
		return Ok(serde_json::json!({}));
	};
	let trimmed = content.trim_start_matches('\u{feff}');
	Ok(serde_json::from_str(trimmed).unwrap_or_else(|_| serde_json::json!({})))
}

/// 写入插件自己的配置
#[tauri::command]
pub async fn extensions_storage_set<R: tauri::Runtime>(
	app: tauri::AppHandle<R>,
	id: String,
	value: serde_json::Value,
) -> Result<bool> {
	let Some(path) = resolve_data_file(&app, &id) else {
		return Ok(false);
	};
	let text = serde_json::to_string_pretty(&value).map_err(|e| anyhow::anyhow!(e))?;
	std::fs::write(path, text).map_err(|e| anyhow::anyhow!(e))?;
	Ok(true)
}

/// 诊断：返回被跳过的 zip 及原因，避免静默失败
#[tauri::command]
pub async fn extensions_diagnostics<R: tauri::Runtime>(
	app: tauri::AppHandle<R>,
) -> Result<Vec<String>> {
	let Some(dir) = extensions_dir(&app) else {
		return Ok(Vec::new());
	};
	let Ok(entries) = std::fs::read_dir(&dir) else {
		return Ok(Vec::new());
	};

	let mut problems = Vec::new();
	for entry in entries.flatten() {
		let path = entry.path();
		if path.extension().and_then(|e| e.to_str()) != Some("zip") {
			continue;
		}
		let file_name = entry.file_name().to_string_lossy().to_string();
		let Some(content) = read_zip_text(&path, "manifest.json") else {
			problems.push(format!("{file_name}: 缺少 manifest.json"));
			continue;
		};
		let content = content.trim_start_matches('\u{feff}');
		let manifest = match serde_json::from_str::<ExtensionManifest>(content) {
			Ok(manifest) => manifest,
			Err(error) => {
				problems.push(format!("{file_name}: manifest 格式错误（{error}）"));
				continue;
			}
		};
		let stem = path
			.file_stem()
			.and_then(|s| s.to_str())
			.unwrap_or_default();
		if manifest.id != stem {
			problems.push(format!(
				"{file_name}: id 与文件名不一致（id={}）",
				manifest.id
			));
		}
	}
	Ok(problems)
}

/// 插件网络请求（需 manifest permissions 包含 "network"）
/// 只允许 http/https，限制响应大小，防止插件拿来做本地文件读取
#[tauri::command]
pub async fn plugin_net_fetch(
	url: String,
	method: Option<String>,
	headers: Option<std::collections::HashMap<String, String>>,
	body: Option<String>,
	timeout_ms: Option<u64>,
) -> std::result::Result<serde_json::Value, String> {
	if !(url.starts_with("http://") || url.starts_with("https://")) {
		return Err("仅支持 http/https".to_string());
	}

	let client = reqwest::Client::builder()
		.timeout(std::time::Duration::from_millis(
			timeout_ms.unwrap_or(15000).clamp(1000, 60000),
		))
		.build()
		.map_err(|e| e.to_string())?;

	let method = method.unwrap_or_else(|| "GET".to_string()).to_uppercase();
	let mut request = match method.as_str() {
		"POST" => client.post(&url),
		"PUT" => client.put(&url),
		"DELETE" => client.delete(&url),
		_ => client.get(&url),
	};

	if let Some(headers) = headers {
		for (key, value) in headers {
			if let (Ok(name), Ok(value)) = (
				reqwest::header::HeaderName::from_bytes(key.as_bytes()),
				reqwest::header::HeaderValue::from_str(&value),
			) {
				request = request.header(name, value);
			}
		}
	}
	if let Some(body) = body {
		request = request.body(body);
	}

	let response = request.send().await.map_err(|e| e.to_string())?;
	let status = response.status().as_u16();
	let text = response
		.text()
		.await
		.map_err(|e| e.to_string())?;
	let trimmed = if text.len() > 4 * 1024 * 1024 {
		text[..4 * 1024 * 1024].to_string()
	} else {
		text
	};
	Ok(serde_json::json!({ "status": status, "body": trimmed }))
}

/// 读取 zip 内的任意文件（二进制），返回 base64
/// 插件用它加载 wasm / 图片 / 字体等资源
#[tauri::command]
pub async fn extensions_read_binary<R: tauri::Runtime>(
	app: tauri::AppHandle<R>,
	id: String,
	file: String,
) -> Result<String> {
	use base64::Engine as _;
	let Some(dir) = extensions_dir(&app) else {
		return Ok(String::new());
	};
	let Some(source) = resolve_source(&dir, &id) else {
		return Ok(String::new());
	};
	let key = file.trim_start_matches('/');
	let zip_path = source.zip;
	let bytes = (|| -> Option<Vec<u8>> {
		let handle = std::fs::File::open(&zip_path).ok()?;
		let mut archive = zip::ZipArchive::new(handle).ok()?;
		for index in 0..archive.len() {
			let mut entry = archive.by_index(index).ok()?;
			let Ok(name) = entry.name().map(|n| n.to_string()) else {
				continue;
			};
			if name == key || name.ends_with(&format!("/{key}")) {
				let mut buffer = Vec::new();
				if entry.read_to_end(&mut buffer).is_ok() {
					return Some(buffer);
				}
			}
		}
		None
	})()
	.unwrap_or_default();
	Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

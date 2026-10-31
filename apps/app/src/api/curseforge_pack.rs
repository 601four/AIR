//! CurseForge 整合包 zip 导入：转换成 .mrpack，交给上游安装任务
//!
//! 上游安装任务负责：建实例、装游戏、装加载器、下载模组（带进度/取消/暂停/回滚）、
//! 解压 overrides。这里只负责把 CF 的 manifest 翻译成 .mrpack。
//!
//! 依赖 MCIM 公益镜像（无需 CurseForge API key）。

use std::io::{Cursor, Read, Write};

use serde::{Deserialize, Serialize};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

const MCIM_BASE: &str = "https://mod.mcimirror.top";
const RESOLVE_CHUNK: usize = 500;

#[derive(Debug, Deserialize)]
struct CfManifest {
	#[serde(default)]
	name: Option<String>,
	#[serde(default)]
	version: Option<String>,
	minecraft: CfMinecraft,
	#[serde(default)]
	files: Vec<CfFile>,
	#[serde(default)]
	overrides: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CfMinecraft {
	version: String,
	#[serde(default, rename = "modLoaders")]
	mod_loaders: Vec<CfModLoader>,
}

#[derive(Debug, Deserialize, Clone)]
struct CfModLoader {
	id: String,
	#[serde(default)]
	primary: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct CfFile {
	#[serde(rename = "projectID")]
	project_id: i64,
	#[serde(rename = "fileID")]
	file_id: i64,
	#[serde(default = "default_true")]
	required: bool,
}

fn default_true() -> bool {
	true
}

#[derive(Debug, Serialize, Clone)]
pub struct CfManualFile {
	pub file_name: String,
	/// 模组显示名（可用于搜索）
	pub display_name: String,
	pub project_id: i64,
	pub file_id: i64,
	pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct CfPackInfo {
	pub title: String,
	pub version: Option<String>,
	pub game_version: String,
	pub loader: String,
	pub loader_version: Option<String>,
	pub required_files: usize,
	pub optional_files: usize,
}

#[derive(Debug, Serialize)]
pub struct CfPrepareResult {
	pub mrpack_path: String,
	pub title: String,
	pub manual_files: Vec<CfManualFile>,
	pub optional_skipped: usize,
}

fn parse_loader(id: &str) -> (String, String) {
	let lower = id.to_ascii_lowercase();
	for (prefix, loader, dep_key) in [
		("neoforge-", "neoforge", "neoforge"),
		("forge-", "forge", "forge"),
		("fabric-", "fabric", "fabric-loader"),
		("quilt-", "quilt", "quilt-loader"),
	] {
		if let Some(version) = lower.strip_prefix(prefix) {
			return (loader.to_string(), version.to_string());
		}
	}
	("vanilla".to_string(), String::new())
}

/// 启动器名 → mrpack dependencies 里的 key
fn mrpack_loader_key(loader: &str) -> &'static str {
	match loader {
		"fabric" => "fabric-loader",
		"quilt" => "quilt-loader",
		"neoforge" => "neoforge",
		"forge" => "forge",
		_ => "minecraft",
	}
}

fn read_zip_entry(archive: &mut ZipArchive<Cursor<Vec<u8>>>, name: &str) -> Option<Vec<u8>> {
	for index in 0..archive.len() {
		let mut entry = archive.by_index(index).ok()?;
		let entry_name = entry.name().ok()?.to_string();
		if entry_name.eq_ignore_ascii_case(name) {
			let mut bytes = Vec::new();
			entry.read_to_end(&mut bytes).ok()?;
			return Some(bytes);
		}
	}
	None
}

fn parse_manifest(zip_path: &str) -> Result<(CfManifest, Vec<u8>), String> {
	let bytes =
		std::fs::read(zip_path).map_err(|error| format!("无法读取整合包: {error}"))?;
	let mut archive = ZipArchive::new(Cursor::new(bytes.clone()))
		.map_err(|error| format!("不是有效的 zip: {error}"))?;
	let manifest_bytes = read_zip_entry(&mut archive, "manifest.json")
		.ok_or_else(|| "整合包里没有 manifest.json，可能不是 CurseForge 整合包".to_string())?;
	let manifest: CfManifest =
		serde_json::from_slice(&manifest_bytes).map_err(|error| format!("manifest 解析失败: {error}"))?;
	Ok((manifest, bytes))
}

fn primary_loader(manifest: &CfManifest) -> Option<&CfModLoader> {
	manifest
		.minecraft
		.mod_loaders
		.iter()
		.find(|loader| loader.primary)
		.or_else(|| manifest.minecraft.mod_loaders.first())
}

fn rewrite_download_url(url: &str) -> String {
	if let Some(rest) = url.strip_prefix("https://edge.forgecdn.net") {
		return format!("{MCIM_BASE}{rest}");
	}
	if let Some(rest) = url.strip_prefix("https://api.curseforge.com") {
		return format!("{MCIM_BASE}/curseforge{rest}");
	}
	url.to_string()
}

#[tauri::command]
pub fn cf_pack_info_inner(zip_path: String) -> Result<CfPackInfo, String> {
	let (manifest, _bytes) = parse_manifest(&zip_path)?;
	let (loader, loader_version) = primary_loader(&manifest)
		.map(|loader| parse_loader(&loader.id))
		.unwrap_or_else(|| ("vanilla".to_string(), String::new()));
	Ok(CfPackInfo {
		title: manifest
			.name
			.clone()
			.unwrap_or_else(|| "CurseForge 整合包".to_string()),
		version: manifest.version.clone(),
		game_version: manifest.minecraft.version.clone(),
		loader,
		loader_version: (!loader_version.is_empty()).then_some(loader_version),
		required_files: manifest.files.iter().filter(|file| file.required).count(),
		optional_files: manifest.files.iter().filter(|file| !file.required).count(),
	})
}

#[tauri::command]
pub async fn cf_pack_to_mrpack_inner(
	zip_path: String,
	include_optional: bool,
) -> Result<CfPrepareResult, String> {
	let (manifest, zip_bytes) = parse_manifest(&zip_path)?;
	let title = manifest
		.name
		.clone()
		.unwrap_or_else(|| "CurseForge 整合包".to_string());
	let (loader_name, loader_version) = primary_loader(&manifest)
		.map(|loader| parse_loader(&loader.id))
		.unwrap_or_else(|| ("vanilla".to_string(), String::new()));
	let dep_key = mrpack_loader_key(&loader_name).to_string();

	let mut selected: Vec<CfFile> = Vec::new();
	let mut optional_skipped = 0usize;
	for file in &manifest.files {
		if file.required || include_optional {
			selected.push(file.clone());
		} else {
			optional_skipped += 1;
		}
	}

	let client = tauri_plugin_http::reqwest::Client::builder()
		.user_agent(theseus::launcher_user_agent())
		.timeout(std::time::Duration::from_secs(120))
		.build()
		.map_err(|error| error.to_string())?;

	let mut file_infos: Vec<serde_json::Value> = Vec::new();
	for chunk in selected.chunks(RESOLVE_CHUNK) {
		let ids: Vec<i64> = chunk.iter().map(|file| file.file_id).collect();
		let body = serde_json::json!({ "fileIds": ids });
		let body_bytes =
			serde_json::to_vec(&body).map_err(|error| format!("请求构造失败: {error}"))?;
		let response = client
			.post(format!("{MCIM_BASE}/curseforge/v1/mods/files"))
			.header("Content-Type", "application/json")
			.body(body_bytes)
			.send()
			.await
			.map_err(|error| format!("连接镜像失败: {error}"))?;
		if !response.status().is_success() {
			return Err(format!("镜像返回错误: HTTP {}", response.status()));
		}
		let text = response
			.text()
			.await
			.map_err(|error| format!("镜像响应读取失败: {error}"))?;
		let payload: serde_json::Value = serde_json::from_str(&text)
			.map_err(|error| format!("镜像响应解析失败: {error}"))?;
		if let Some(data) = payload.get("data").and_then(|value| value.as_array()) {
			file_infos.extend(data.iter().cloned());
		}
	}

	let mut mrpack_files: Vec<serde_json::Value> = Vec::new();
	let mut manual_files: Vec<CfManualFile> = Vec::new();
	for item in &file_infos {
		let file_id = item.get("id").and_then(|value| value.as_i64()).unwrap_or_default();
		let project_id = item.get("modId").and_then(|value| value.as_i64()).unwrap_or_default();
		let file_name = item
			.get("fileName")
			.and_then(|value| value.as_str())
			.unwrap_or("unknown.jar")
			.to_string();
		let download_url = item.get("downloadUrl").and_then(|value| value.as_str());
		let sha1 = item
			.get("hashes")
			.and_then(|value| value.as_array())
			.and_then(|hashes| {
				hashes.iter().find_map(|hash| {
					let algo = hash.get("algo").and_then(|value| value.as_i64()).unwrap_or(0);
					if algo == 1 {
						hash.get("value").and_then(|value| value.as_str()).map(str::to_string)
					} else {
						None
					}
				})
			});
		let file_size = item.get("fileLength").and_then(|value| value.as_u64()).unwrap_or(0);
		let manifest_project_id = selected
			.iter()
			.find(|file| file.file_id == file_id)
			.map(|file| file.project_id)
			.unwrap_or(project_id);
		let display_name = item
			.get("displayName")
			.and_then(|value| value.as_str())
			.map(str::to_string)
			.unwrap_or_else(|| file_name.clone());

		match (download_url, sha1) {
			(Some(url), Some(sha1)) => {
				// 镜像优先，但保留官方地址作为回退（上游下载器会依次尝试）
				let mirrored = rewrite_download_url(url);
				let downloads = if mirrored == url {
					serde_json::json!([url])
				} else {
					serde_json::json!([mirrored, url])
				};
				mrpack_files.push(serde_json::json!({
					"path": format!("mods/{file_name}"),
					"hashes": { "sha1": sha1 },
					"downloads": downloads,
					"fileSize": file_size
				}));
			}
			_ => {
				manual_files.push(CfManualFile {
					file_name,
					display_name,
					project_id: manifest_project_id,
					file_id,
					reason: if download_url.is_none() {
						"作者禁止第三方分发，请到网页手动下载".to_string()
					} else {
						"镜像缺少 SHA-1 校验信息".to_string()
					},
				});
			}
		}
	}

	if mrpack_files.is_empty() {
		return Err("没有可自动下载的文件".to_string());
	}

	let mut dependencies = serde_json::Map::new();
	dependencies.insert("minecraft".to_string(), serde_json::json!(manifest.minecraft.version));
	dependencies.insert(dep_key, serde_json::json!(loader_version));

	let index = serde_json::json!({
		"formatVersion": 1,
		"game": "minecraft",
		"versionId": manifest.version.clone().unwrap_or_else(|| "1.0.0".to_string()),
		"name": title.clone(),
		"files": mrpack_files,
		"dependencies": dependencies
	});

	let output_path = std::env::temp_dir().join(format!(
		"aircore-cf-{}.mrpack",
		uuid::Uuid::new_v4()
	));
	{
		let file = std::fs::File::create(&output_path)
			.map_err(|error| format!("无法创建临时文件: {error}"))?;
		let mut writer = ZipWriter::new(file);
		let options = SimpleFileOptions::default();
		writer
			.start_file("modrinth.index.json", options)
			.map_err(|error| error.to_string())?;
		writer
			.write_all(
				serde_json::to_vec_pretty(&index)
					.map_err(|error| error.to_string())?
					.as_slice(),
			)
			.map_err(|error| error.to_string())?;

		let overrides_prefix = manifest
			.overrides
			.clone()
			.unwrap_or_else(|| "overrides".to_string());
		let prefix = format!("{overrides_prefix}/");
		let mut archive = ZipArchive::new(Cursor::new(zip_bytes))
			.map_err(|error| format!("不是有效的 zip: {error}"))?;
		for index_entry in 0..archive.len() {
			let mut entry = match archive.by_index(index_entry) {
				Ok(entry) => entry,
				Err(_) => continue,
			};
			let name = match entry.name() {
				Ok(name) => name.to_string(),
				Err(_) => continue,
			};
			let Some(relative) = name.strip_prefix(&prefix) else {
				continue;
			};
			if relative.is_empty() || name.ends_with('/') {
				continue;
			}
			let mut buffer = Vec::new();
			if entry.read_to_end(&mut buffer).is_err() {
				continue;
			}
			writer
				.start_file(format!("overrides/{relative}"), options)
				.map_err(|error| error.to_string())?;
			writer.write_all(&buffer).map_err(|error| error.to_string())?;
		}
		writer.finish().map_err(|error| error.to_string())?;
	}

	Ok(CfPrepareResult {
		mrpack_path: output_path.to_string_lossy().into_owned(),
		title,
		manual_files,
		optional_skipped,
	})
}


/// 删除一个临时 .mrpack
pub fn cf_cleanup_temp_pack_inner(path: String) -> bool {
	let target = std::path::PathBuf::from(&path);
	let is_temp_pack = target
		.file_name()
		.and_then(|name| name.to_str())
		.is_some_and(|name| name.starts_with("aircore-cf-") && name.ends_with(".mrpack"));
	if !is_temp_pack {
		return false;
	}
	std::fs::remove_file(&target).is_ok()
}

/// 清理上次崩溃/强退遗留的临时 .mrpack
pub fn cleanup_stale_temp_packs() {
	let temp_dir = std::env::temp_dir();
	let Ok(entries) = std::fs::read_dir(&temp_dir) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
			continue;
		};
		if name.starts_with("aircore-cf-") && name.ends_with(".mrpack") {
			let _ = std::fs::remove_file(&path);
		}
	}
}

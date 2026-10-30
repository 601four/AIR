//! 下载源（镜像）支持。
//! 默认走官方；开启国内镜像时，把 Mojang 相关域名重写到 BMCLAPI。

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadSource {
	Official,
	Bmclapi,
}

static SOURCE: AtomicU8 = AtomicU8::new(0);

pub fn set_source(source: DownloadSource) {
	SOURCE.store(
		match source {
			DownloadSource::Official => 0,
			DownloadSource::Bmclapi => 1,
		},
		Ordering::Relaxed,
	);
}

pub fn current_source() -> DownloadSource {
	if SOURCE.load(Ordering::Relaxed) == 1 {
		DownloadSource::Bmclapi
	} else {
		DownloadSource::Official
	}
}

const BMCLAPI: &str = "https://bmclapi2.bangbang93.com";

/// 加载器下载源模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoaderSource {
	/// 官方直连
	Official,
	/// 国内加速：Maven Central 走阿里云，其他保持官方
	Cn,
	/// 自定义镜像前缀（要求路径透传）
	Custom,
}

static LOADER_MODE: AtomicU8 = AtomicU8::new(1);
static LOADER_PREFIX: std::sync::OnceLock<std::sync::Mutex<Option<String>>> =
	std::sync::OnceLock::new();

fn loader_prefix() -> &'static std::sync::Mutex<Option<String>> {
	LOADER_PREFIX.get_or_init(|| std::sync::Mutex::new(None))
}

/// 设置加载器下载源
pub fn set_loader_source(mode: LoaderSource, prefix: Option<String>) {
	LOADER_MODE.store(
		match mode {
			LoaderSource::Official => 0,
			LoaderSource::Cn => 1,
			LoaderSource::Custom => 2,
		},
		Ordering::Relaxed,
	);
	let cleaned = prefix.and_then(|value| {
		let trimmed = value.trim().trim_end_matches('/').to_string();
		if trimmed.is_empty() {
			None
		} else {
			Some(trimmed)
		}
	});
	if let Ok(mut guard) = loader_prefix().lock() {
		*guard = cleaned;
	}
}

pub fn current_loader_mode() -> LoaderSource {
	match LOADER_MODE.load(Ordering::Relaxed) {
		0 => LoaderSource::Official,
		2 => LoaderSource::Custom,
		_ => LoaderSource::Cn,
	}
}

pub fn current_loader_prefix() -> Option<String> {
	loader_prefix().lock().ok().and_then(|guard| guard.clone())
}

const ALIYUN_MAVEN: &str = "https://maven.aliyun.com/repository/public";

/// 把 Mojang 官方地址重写为镜像地址（仅开启镜像时）
pub fn rewrite_url(url: &str) -> String {
	if current_source() == DownloadSource::Official {
		return url.to_string();
	}

	let rules: [(&str, &str); 5] = [
		("https://piston-meta.mojang.com", BMCLAPI),
		("https://launchermeta.mojang.com", BMCLAPI),
		("https://piston-data.mojang.com", BMCLAPI),
		("https://libraries.minecraft.net", "https://bmclapi2.bangbang93.com/maven"),
		("https://resources.download.minecraft.net", "https://bmclapi2.bangbang93.com/assets"),
	];

	for (from, to) in rules {
		if let Some(rest) = url.strip_prefix(from) {
			return format!("{to}{rest}");
		}
	}

	// 加载器 maven
	match current_loader_mode() {
		LoaderSource::Official => {}
		LoaderSource::Cn => {
			// Maven Central 走阿里云镜像；Fabric/Forge 仓库暂无可靠国内镜像，保持官方
			if let Some(rest) = url.strip_prefix("https://repo1.maven.org/maven2") {
				return format!("{ALIYUN_MAVEN}{rest}");
			}
		}
		LoaderSource::Custom => {
			if let Some(prefix) = current_loader_prefix() {
				const LOADER_HOSTS: [&str; 6] = [
					"https://maven.fabricmc.net",
					"https://maven.quiltmc.org",
					"https://maven.minecraftforge.net",
					"https://files.minecraftforge.net",
					"https://maven.neoforged.net/releases",
					"https://maven.neoforged.net",
				];
				for host in LOADER_HOSTS {
					if let Some(rest) = url.strip_prefix(host) {
						return format!("{prefix}{rest}");
					}
				}
			}
		}
	}

	url.to_string()
}

/// 回退计数：按资源类别记录镜像失败次数
static FAILURES: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<&'static str, u32>>> =
	std::sync::OnceLock::new();

fn failures() -> &'static std::sync::Mutex<std::collections::HashMap<&'static str, u32>> {
	FAILURES.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// 把 URL 归类，便于判断哪一类资源的镜像不可用
pub fn category(url: &str) -> &'static str {
	if url.contains("resources.download.minecraft.net") {
		"assets"
	} else if url.contains("libraries.minecraft.net") {
		"maven"
	} else {
		"meta"
	}
}

/// 记录一次回退；同一类资源连续失败多次后本次运行不再尝试镜像
pub fn record_failure(category: &'static str) {
	let mut guard = match failures().lock() {
		Ok(guard) => guard,
		Err(_) => return,
	};
	let count = guard.entry(category).or_insert(0);
	*count += 1;
}

/// 该类资源的镜像是否已判定不可用
pub fn is_broken(category: &'static str) -> bool {
	let guard = match failures().lock() {
		Ok(guard) => guard,
		Err(_) => return false,
	};
	guard.get(category).copied().unwrap_or(0) >= 5
}

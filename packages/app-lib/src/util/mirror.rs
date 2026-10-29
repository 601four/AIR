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

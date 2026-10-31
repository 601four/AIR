/*!
# Theseus

Theseus is a library which provides utilities for launching minecraft, creating Modrinth mod packs,
and launching Modrinth mod packs
*/
#![warn(unused_import_braces)]
#![deny(unused_must_use)]

#[macro_use]
pub mod util;

mod api;
mod error;
mod event;
pub mod install;
mod launcher;
mod logger;
mod state;

pub use api::*;
pub use error::*;
#[cfg(feature = "export-ts")]
pub use event::export_app_event_bindings;
pub use event::{
    AppEvent, EventState, LoadingBar, LoadingBarType, emit::emit_loading,
    emit::emit_warning, emit::init_loading,
};
pub use logger::start_logger;
pub use state::State;
pub use util::fetch::DownloadReason;

/// 运行时注册的启动器版本（由 app 启动时设置）
static APP_VERSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// 由 app 层在启动时调用，把真实版本号告知 theseus
pub fn set_app_version(version: impl Into<String>) {
    let _ = APP_VERSION.set(version.into());
}

fn app_version() -> String {
    APP_VERSION
        .get()
        .cloned()
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string())
}

/// 全局 User-Agent：名称固定，版本号跟随实际发布版本
pub fn launcher_user_agent() -> String {
    format!(
        "AirCore/{} ({}; https://github.com/601four/AIR)",
        app_version(),
        std::env::consts::OS
    )
}

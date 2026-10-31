//! 窗口位置 / 大小记忆（存到用户设置的数据目录，而不是固定 C 盘）

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedWindowState {
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub maximized: bool,
}

pub fn state_path<R: Runtime>(app: &AppHandle<R>) -> Option<std::path::PathBuf> {
    let identifier = app.config().identifier.clone();
    let base = theseus::data_dir::effective_dir(&identifier)
        .or_else(|| app.path().app_data_dir().ok())?;
    Some(base.join("window-state.json"))
}

/// 启动时恢复窗口状态；位置不在任何显示器内时跳过位置
pub fn restore<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Some(path) = state_path(app) else {
        return;
    };

    let Ok(text) = std::fs::read_to_string(&path) else {
        // 首次使用新数据目录：尝试从旧位置迁移一次
        migrate_old_state(app, &path);
        return;
    };
    let Ok(state) = serde_json::from_str::<SavedWindowState>(&text) else {
        return;
    };

    if state.width >= 200 && state.height >= 200 {
        let _ = window.set_size(tauri::PhysicalSize::new(state.width, state.height));
    }

    let position_visible = app
        .available_monitors()
        .map(|monitors| {
            monitors.iter().any(|monitor| {
                let pos = monitor.position();
                let size = monitor.size();
                state.x >= pos.x
                    && state.y >= pos.y
                    && state.x < pos.x + size.width as i32
                    && state.y < pos.y + size.height as i32
            })
        })
        .unwrap_or(false);
    if position_visible && !state.maximized {
        let _ = window.set_position(tauri::PhysicalPosition::new(state.x, state.y));
    }

    if state.maximized {
        let _ = window.maximize();
    } else {
        let _ = window.unmaximize();
    }
}

/// 保存当前窗口状态（关闭前调用）
pub fn save<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Some(path) = state_path(app) else {
        return;
    };

    let maximized = window.is_maximized().unwrap_or(false);
    let size = window.inner_size().ok();
    let position = window.outer_position().ok();

    // 最大化时位置不可靠：位置留空（-1），恢复时会忽略
    let state = SavedWindowState {
        width: size.map(|s| s.width).unwrap_or(1280),
        height: size.map(|s| s.height).unwrap_or(800),
        x: if maximized { -1 } else { position.map(|p| p.x).unwrap_or(-1) },
        y: if maximized { -1 } else { position.map(|p| p.y).unwrap_or(-1) },
        maximized,
    };

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&state) {
        let _ = std::fs::write(path, json);
    }
}

/// 旧版本的 app-window-state.json（Tauri 插件格式）转换一次
fn migrate_old_state<R: Runtime>(app: &AppHandle<R>, new_path: &std::path::Path) {
    let Ok(base) = app.path().app_data_dir() else {
        return;
    };
    let old = base.join("app-window-state.json");
    let Ok(text) = std::fs::read_to_string(old) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return;
    };
    let Some(main) = value.get("main") else {
        return;
    };
    let read_u32 = |key: &str, fallback: u32| {
        main.get(key).and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(fallback)
    };
    let read_i32 = |key: &str| main.get(key).and_then(|v| v.as_i64()).map(|v| v as i32);
    let state = SavedWindowState {
        width: read_u32("width", 1280),
        height: read_u32("height", 800),
        x: read_i32("x").unwrap_or(-1),
        y: read_i32("y").unwrap_or(-1),
        maximized: main.get("maximized").and_then(|v| v.as_bool()).unwrap_or(false),
    };
    if let Some(parent) = new_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&state) {
        let _ = std::fs::write(new_path, json);
    }
}

//! 系统托盘：最小化到托盘、托盘菜单（显示 / 退出）

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

pub const TRAY_ID: &str = "aircore-tray";

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
	if let Some(window) = app.get_webview_window("main") {
		let _ = window.show();
		let _ = window.unminimize();
		let _ = window.set_focus();
	}
}

/// 创建托盘图标：左键显示主窗口，右键菜单
pub fn setup_tray<R: Runtime>(app: &tauri::App<R>) -> tauri::Result<()> {
	let menu = build_menu(app.handle(), "Show", "Quit")?;
	let mut builder = TrayIconBuilder::with_id(TRAY_ID)
		.tooltip("AirCore")
		.menu(&menu)
		.show_menu_on_left_click(false)
		.on_menu_event(|app, event| match event.id.as_ref() {
			"tray-show" => show_main_window(app),
			"tray-quit" => {
				crate::window_state::save(app);
				app.exit(0);
			}
			_ => {}
		})
		.on_tray_icon_event(|tray, event| {
			if let TrayIconEvent::Click {
				button: MouseButton::Left,
				button_state: MouseButtonState::Up,
				..
			} = event
			{
				show_main_window(tray.app_handle());
			}
		});
	if let Some(icon) = app.default_window_icon().cloned() {
		builder = builder.icon(icon);
	}
	builder.build(app)?;
	Ok(())
}

fn build_menu<R: Runtime>(
	app: &AppHandle<R>,
	show_label: &str,
	quit_label: &str,
) -> tauri::Result<Menu<R>> {
	let show = MenuItem::with_id(app, "tray-show", show_label, true, None::<&str>)?;
	let quit = MenuItem::with_id(app, "tray-quit", quit_label, true, None::<&str>)?;
	Menu::with_items(app, &[&show, &quit])
}

/// 前端安装完语言包后，把菜单文字换成当前语言
pub fn update_labels<R: Runtime>(
	app: &AppHandle<R>,
	show_label: &str,
	quit_label: &str,
) -> tauri::Result<()> {
	if let Some(tray) = app.tray_by_id(TRAY_ID) {
		let menu = build_menu(app, show_label, quit_label)?;
		tray.set_menu(Some(menu))?;
	}
	Ok(())
}

/// 最小化到托盘：隐藏主窗口
pub fn hide_to_tray<R: Runtime>(app: &AppHandle<R>) {
	if let Some(window) = app.get_webview_window("main") {
		let _ = window.hide();
	}
}

/// 从托盘恢复主窗口
pub fn show_from_tray<R: Runtime>(app: &AppHandle<R>) {
	show_main_window(app);
}

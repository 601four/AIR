// 去商业化：广告相关逻辑已全部移除。
// 这里只保留空的函数名，避免其它文件 import 报错。
// 原来的实现是调用 Tauri 的 plugin:ads 命令。

export async function init_ads_window(_overrideShown = false) {}

export async function take_ads_window_hold() {}

export async function release_ads_window_hold() {}

export async function hide_ads_window(_reset) {}

export async function should_show_ads_consent_popup() {
	return false
}

export async function perform_ads_consent_action(_action) {}

export async function open_ads_consent_preferences() {}

export async function record_ads_click() {}

export async function open_ads_link(_path, _origin) {}

export async function get_ads_debug_state() {
	return null
}

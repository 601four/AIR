import { invoke } from '@tauri-apps/api/core'

/**
 * AirCore 自定义偏好的存储层。
 *
 * 策略：
 *  - localStorage 作为同步缓存（页面启动时立即能读）
 *  - 数据库表 aircore_prefs 作为真正存储（重装/换电脑不丢）
 */

export const PREF_KEYS = {
	accentColor: 'aircore-accent-color',
	autoMemory: 'aircore-auto-memory',
	prelaunchOptimize: 'aircore-prelaunch-memory-opt',
	downloadSource: 'aircore-download-source',
} as const

/** 启动时把数据库里的偏好回填到 localStorage */
export async function hydratePrefs(): Promise<void> {
	try {
		const stored = await invoke<Record<string, string>>('plugin:utils|prefs_get_all')
		const databasePrefs = stored ?? {}
		for (const [key, value] of Object.entries(databasePrefs)) {
			if (typeof value === 'string') localStorage.setItem(key, value)
		}
		// 数据库里还没有、但本地已有的偏好，上传一次
		for (const key of Object.values(PREF_KEYS)) {
			const localValue = localStorage.getItem(key)
			if (localValue !== null && databasePrefs[key] === undefined) {
				void invoke('plugin:utils|prefs_set', { key, value: localValue }).catch(() => {})
			}
		}
	} catch (error) {
		console.warn('[prefs] hydrate failed, using localStorage only', error)
	}
}

/** 写入一个偏好：先写本地缓存，再异步写入数据库 */
export function setPref(key: string, value: string | null): void {
	try {
		if (value === null) localStorage.removeItem(key)
		else localStorage.setItem(key, value)
	} catch {
		// ignore
	}
	void invoke('plugin:utils|prefs_set', { key, value: value ?? '' }).catch((error) => {
		console.warn('[prefs] persist failed', key, error)
	})
}

/** 同步读取（从本地缓存） */
export function getPref(key: string): string | null {
	try {
		return localStorage.getItem(key)
	} catch {
		return null
	}
}

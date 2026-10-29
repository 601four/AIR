import { invoke } from '@tauri-apps/api/core'

import { setPref } from './use-prefs'
import { computed, ref } from 'vue'

// 启动器的内存策略设置（存在本地，不走账号同步）
const AUTO_KEY = 'aircore-auto-memory'
const OPT_KEY = 'aircore-prelaunch-memory-opt'

function readFlag(key: string): boolean {
	try {
		return window.localStorage.getItem(key) === '1'
	} catch {
		return false
	}
}

const autoMemory = ref<boolean>(readFlag(AUTO_KEY))
const prelaunchOptimize = ref<boolean>(readFlag(OPT_KEY))

/**
 * 根据物理内存推荐一个分配值（MB）。
 * 规则：总内存的 50%，下限 2GB，上限 8GB，并按 512MB 取整。
 */
export function recommendMemoryMb(totalMb: number): number {
	if (!Number.isFinite(totalMb) || totalMb <= 0) return 2048
	const half = Math.floor(totalMb * 0.5)
	const clamped = Math.min(Math.max(half, 2048), 8192)
	return Math.round(clamped / 512) * 512
}

export function useMemorySettings() {
	const setAutoMemory = (value: boolean) => {
		autoMemory.value = value
		setPref(AUTO_KEY, value ? '1' : '0')
	}

	const setPrelaunchOptimize = (value: boolean) => {
		prelaunchOptimize.value = value
		setPref(OPT_KEY, value ? '1' : '0')
	}

	return {
		autoMemory: computed(() => autoMemory.value),
		prelaunchOptimize: computed(() => prelaunchOptimize.value),
		setAutoMemory,
		setPrelaunchOptimize,
	}
}

export function isPrelaunchOptimizeEnabled(): boolean {
	return prelaunchOptimize.value
}

export interface MemoryOptimizeResult {
	available_before_mb: number
	available_after_mb: number
	freed_mb: number
	trimmed_processes: number
}

const lastOptimizeResult = ref<MemoryOptimizeResult | null>(null)

/** 立即执行一次系统内存优化（全局能力） */
export async function runMemoryOptimize(): Promise<MemoryOptimizeResult | null> {
	try {
		const result = await invoke<MemoryOptimizeResult>('plugin:utils|optimize_launcher_memory')
		lastOptimizeResult.value = result
		console.info(
			'[memory] released ' + result.freed_mb + ' MB, trimmed ' + result.trimmed_processes + ' processes',
		)
		return result
	} catch (error) {
		console.warn('[memory] optimize failed', error)
		return null
	}
}

export function useMemoryOptimizeResult() {
	return computed(() => lastOptimizeResult.value)
}

import { computed, ref } from 'vue'

import { setPref } from './use-prefs'

// 主题强调色：直接改写 CSS 变量，不动底层主题文件
export interface AccentOption {
	id: string
	name: string
	color: string
}

// 新玩家的默认强调色
export const DEFAULT_ACCENT = '#ff5050'

export const ACCENT_OPTIONS: AccentOption[] = [
	{ id: 'red', name: '朱红', color: '#ff5050' },
	{ id: 'green', name: '经典绿', color: '#1bd96a' },
	{ id: 'blue', name: '天空蓝', color: '#3b82f6' },
	{ id: 'pink', name: '樱花粉', color: '#ec4899' },
]

const STORAGE_KEY = 'aircore-accent-color'

function readStored(): string {
	try {
		return window.localStorage.getItem(STORAGE_KEY) ?? DEFAULT_ACCENT
	} catch {
		return DEFAULT_ACCENT
	}
}


// 清理旧版本留下的面板染色内联样式（只清一次）
const LEGACY_SURFACE_VARS = [
	'--surface-1', '--surface-1-5', '--surface-2', '--surface-2-5',
	'--surface-3', '--surface-4', '--surface-5',
	'--color-bg', '--color-raised-bg', '--color-super-raised-bg',
	'--color-button-bg', '--color-divider', '--color-divider-dark', '--color-tooltip-bg',
]
for (const name of LEGACY_SURFACE_VARS) {
	document.documentElement.style.removeProperty(name)
}

const accentColor = ref<string>(readStored())

function hexToRgb(hex: string): { r: number; g: number; b: number } {
	const value = hex.replace('#', '')
	const full = value.length === 3 ? value.split('').map((c) => c + c).join('') : value
	const int = Number.parseInt(full, 16)
	return { r: (int >> 16) & 255, g: (int >> 8) & 255, b: int & 255 }
}

function rgba(hex: string, alpha: number): string {
	const { r, g, b } = hexToRgb(hex)
	return `rgba(${r}, ${g}, ${b}, ${alpha})`
}

function rgbAlpha(rgbValue: string, alpha: number): string {
	const rgb = parseRgbLike(rgbValue)
	if (!rgb) return rgbValue
	return `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${alpha})`
}

function parseRgbLike(value: string): { r: number; g: number; b: number } | null {
	const match = value.match(/rgba?\((\d+)[,\s]+(\d+)[,\s]+(\d+)/)
	return match ? { r: Number(match[1]), g: Number(match[2]), b: Number(match[3]) } : null
}

function mix(hex: string, target: string, amount: number): string {
	const a = hexToRgb(hex)
	const b = hexToRgb(target)
	const to = (x: number, y: number) => Math.round(x + (y - x) * amount)
	return `rgb(${to(a.r, b.r)}, ${to(a.g, b.g)}, ${to(a.b, b.b)})`
}



export function applyAccentColor(color: string) {
	const root = document.documentElement
	const isDark = root.classList.contains('dark-mode') || root.classList.contains('oled-mode')

	root.style.setProperty('--color-brand', color)
	root.style.setProperty('--color-brand-highlight', rgba(color, 0.25))
	root.style.setProperty('--color-brand-shadow', rgba(color, 0.7))
	root.style.setProperty('--loading-bar-gradient', `linear-gradient(to right, ${color} 0%, ${mix(color, '#ffffff', 0.35)} 100%)`)

	// 同时覆盖 green 系列变量，让组件里的 text-green / bg-green 也跟着变
	root.style.setProperty('--color-green', color)
	root.style.setProperty('--color-green-highlight', isDark ? rgba(color, 0.25) : mix(color, '#ffffff', 0.8))

	// 侧边栏 / 顶栏用的是 --brand-gradient-*，原本硬编码为绿色渐变，这里一并换成主题色
	// 和上游一样是“很深的主题色”渐变，而不是直接拿亮色堆上去
	const deepTop = mix(color, '#000000', 0.85)
	const deepBottom = mix(color, '#000000', 0.55)
	root.style.setProperty(
		'--brand-gradient-bg',
		`linear-gradient(0deg, ${rgbAlpha(deepTop, 0.2)} 0%, ${rgbAlpha(deepBottom, 0.1)} 100%)`,
	)
	root.style.setProperty(
		'--brand-gradient-strong-bg',
		`linear-gradient(270deg, ${mix(color, '#000000', 0.88)} 10%, ${mix(color, '#000000', 0.78)} 100%)`,
	)
	root.style.setProperty('--brand-gradient-border', rgba(color, 0.12))
	root.style.setProperty(
		'--brand-gradient-fade-out-color',
		`linear-gradient(to bottom, ${rgba(color, 0)} 0%, ${mix(color, '#000000', 0.85)} 80%)`,
	)

}

export function useAccentColor() {

	const setAccent = (color: string) => {
		accentColor.value = color
		applyAccentColor(color)
		setPref(STORAGE_KEY, color)
	}

	const reset = () => setAccent(DEFAULT_ACCENT)


	return {
		accentColor,
		current: computed(() => accentColor.value),
		isCustom: computed(() => !ACCENT_OPTIONS.some((option) => option.color === accentColor.value)),
		setAccent,
		reset,
	}
}

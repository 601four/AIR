import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

const CJK = /[\u4e00-\u9fff]/
const cache = new Map<string, string>()

function firstClassId(html: string): string | null {
	const match = /\/class\/(\d+)\.html/.exec(html)
	return match ? match[1] : null
}

function anchorEnglish(html: string): string | null {
	const re = /\/class\/(\d+)\.html[^>]*>([\s\S]*?)<\/a>/g
	let match: RegExpExecArray | null
	while ((match = re.exec(html))) {
		const text = match[2].replace(/<[^>]+>/g, '')
		const paren = /\(([^)]+)\)\s*$/.exec(text)
		if (paren && /[A-Za-z]/.test(paren[1])) {
			return paren[1].trim()
		}
	}
	return null
}

async function getText(url: string): Promise<string> {
	const response = await tauriFetch(url, { method: 'GET' })
	if (!response.ok) throw new Error(`HTTP ${response.status}`)
	return await response.text()
}

/** 中文搜索兜底：用 MC 百科把中文名换成英文名 */
export async function translateChineseName(name: string): Promise<string> {
	if (!CJK.test(name)) return name
	const cached = cache.get(name)
	if (cached) return cached
	try {
		const html = await getText(
			`https://search.mcmod.cn/s?key=${encodeURIComponent(name)}`,
		)
		const english = anchorEnglish(html)
		if (english) {
			cache.set(name, english)
			return english
		}
	} catch {
		// 网络不可用时保持原名
	}
	return name
}

/** 直接解析 MC 百科详情页（class）地址 */
export async function resolveMcmodUrl(name: string): Promise<string> {
	const searchUrl = `https://search.mcmod.cn/s?key=${encodeURIComponent(name)}`
	try {
		const html = await getText(searchUrl)
		const id = firstClassId(html)
		if (id) return `https://www.mcmod.cn/class/${id}.html`
	} catch {
		// fallback
	}
	return searchUrl
}

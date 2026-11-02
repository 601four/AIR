import { invoke } from '@tauri-apps/api/core'

const CJK = /[\u4e00-\u9fff]/
const cache = new Map<string, string>()

/**
 * 将搜索参数里的中文 query 换成英文关键词。
 * 先查内置别名表（已在 search.ts 做过），
 * 剩下的交给 MC 百科搜索结果提取英文名。
 */
export async function translateSearchParams(params: string): Promise<string> {
	const match = /(?:^|&)query=([^&]*)/.exec(params)
	if (!match) return params
	let raw = match[1]
	try {
		raw = decodeURIComponent(raw)
	} catch {
		return params
	}
	if (!CJK.test(raw)) return params

	const cached = cache.get(raw)
	if (cached) {
		return params.replace(match[0], `query=${encodeURIComponent(cached)}`)
	}

	try {
		const english = await invoke<string | null>('plugin:utils|mcmod_translate', { name: raw })
		if (english) {
			cache.set(raw, english)
			return params.replace(match[0], `query=${encodeURIComponent(english)}`)
		}
	} catch {
		// 网络不可用时保持原样
	}
	return params
}

import { translateChineseName } from './mcmod'

const CJK = /[\u4e00-\u9fff]/

/**
 * 将搜索参数里的中文 query 换成英文关键词。
 * 内置别名表在 search.ts 里已经处理，
 * 这里负责内置表没命中时走 MC 百科解析英文名。
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

	const english = await translateChineseName(raw)
	if (!english || english === raw) return params
	return params.replace(match[0], `query=${encodeURIComponent(english)}`)
}

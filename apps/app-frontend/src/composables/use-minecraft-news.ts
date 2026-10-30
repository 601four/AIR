import { fetch as tauriFetch } from '@tauri-apps/plugin-http'
import { ref } from 'vue'

/** Minecraft 官方启动器新闻源 v2（带图片） */
const NEWS_URL = 'https://launchercontent.mojang.com/v2/news.json'
const IMAGE_BASE = 'https://launchercontent.mojang.com'
const CACHE_KEY = 'aircore-mc-news-cache-v2'
const CACHE_TTL_MS = 24 * 60 * 60 * 1000

export interface MinecraftNewsItem {
	id: string
	title: string
	category: string
	date: string
	link: string
	image: string
}

interface RawImage {
	url?: string
}

interface RawEntry {
	id?: string
	title?: string
	category?: string
	date?: string
	readMoreLink?: string
	newsPageImage?: RawImage
	playPageImage?: RawImage
}

const items = ref<MinecraftNewsItem[]>([])
const loaded = ref(false)
let pending: Promise<void> | null = null

function readCache(): MinecraftNewsItem[] | null {
	try {
		const raw = window.localStorage.getItem(CACHE_KEY)
		if (!raw) return null
		const parsed = JSON.parse(raw) as { at: number; items: MinecraftNewsItem[] }
		if (!parsed?.items || typeof parsed.at !== 'number') return null
		if (Date.now() - parsed.at > CACHE_TTL_MS) return null
		return parsed.items
	} catch {
		return null
	}
}

function writeCache(value: MinecraftNewsItem[]) {
	try {
		window.localStorage.setItem(CACHE_KEY, JSON.stringify({ at: Date.now(), items: value }))
	} catch {
		// ignore
	}
}

function imageUrl(entry: RawEntry): string {
	const relative = entry.newsPageImage?.url ?? entry.playPageImage?.url ?? ''
	if (!relative) return ''
	if (relative.startsWith('http')) return relative
	return IMAGE_BASE + (relative.startsWith('/') ? relative : '/' + relative)
}

function normalize(entries: RawEntry[]): MinecraftNewsItem[] {
	return entries
		.filter((entry) => entry?.title && entry?.readMoreLink)
		.slice(0, 6)
		.map((entry, index) => ({
			id: entry.id ?? String(index),
			title: String(entry.title),
			category: entry.category ? String(entry.category) : '',
			date: entry.date ? String(entry.date) : '',
			link: String(entry.readMoreLink),
			image: imageUrl(entry),
		}))
}

export async function loadMinecraftNews(force = false): Promise<void> {
	if (pending) return pending
	if (!force && items.value.length > 0) return
	const cached = force ? null : readCache()
	if (cached && cached.length > 0) {
		items.value = cached
		loaded.value = true
		return
	}

	pending = (async () => {
		try {
			// 只走 Tauri 的 HTTP 层，不受页面 CSP 影响
			const response = await tauriFetch(NEWS_URL, { headers: { Accept: 'application/json' } })
			if (!response.ok) throw new Error('HTTP ' + response.status)
			const data = (await response.json()) as { entries?: RawEntry[] }
			const next = normalize(data.entries ?? [])
			if (next.length > 0) {
				items.value = next
				writeCache(next)
			}
		} catch (error) {
			console.warn('[minecraft-news] load failed', error)
		} finally {
			loaded.value = true
			pending = null
		}
	})()
	return pending
}

export function useMinecraftNews() {
	return { items, loaded, loadMinecraftNews }
}

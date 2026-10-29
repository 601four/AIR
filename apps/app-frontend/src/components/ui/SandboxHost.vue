<template>
	<div class="hidden">
		<iframe
			v-for="plugin in sandboxPlugins"
			:key="`${plugin.id}:${sandboxReloadToken.get(plugin.id) ?? 0}`"
			:ref="(element) => registerFrame(plugin.id, element)"
			sandbox="allow-scripts"
			:srcdoc="documents[plugin.id] ?? ''"
		/>
	</div>
</template>

<script setup lang="ts">
import { clearCanvas, setCanvasOps, type CanvasOp } from '@/plugins/canvas-store'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue'

import { renderUiTree, type UiNode } from '@/plugins/ui-renderer'
import { SANDBOX_SDK } from '@/plugins/sandbox-sdk'
import {
	logPlugin,
	pluginState,
	pushPluginToast,
	sandboxFrames,
	sandboxReloadToken,
	setSandboxSlot,
} from '@/plugin-host'

const documents = reactive<Record<string, string>>({})

const sandboxPlugins = computed(() =>
	pluginState.extensions.filter(
		(extension) => extension.enabled !== false && (extension.entry ?? '').endsWith('.html'),
	),
)

function registerFrame(pluginId: string, element: unknown) {
	if (element instanceof HTMLIFrameElement) {
		sandboxFrames.set(pluginId, element)
	}
}


/** 把 zip 里的外部脚本 / 样式直接内联进 HTML，
 *  避免依赖自定义协议（Windows 下路径很容易出问题）*/
async function inlineAssets(pluginId: string, html: string): Promise<string> {
	let result = html

	// 把 <script src="..."> 膏成内联脚本
	const scriptPattern = /<script\b([^>]*?)\bsrc\s*=\s*["']([^"']+)["']([^>]*)>\s*<\/script>/gi
	const scriptMatches = [...result.matchAll(scriptPattern)]
	for (const match of scriptMatches) {
		const [, before, src, after] = match
		const isModule = /type\s*=\s*["']module["']/i.test(before + after)
		try {
			const code = await invoke<string>('plugin:extensions|extensions_read', {
				id: pluginId,
				file: src,
			})
			const typeAttr = isModule ? ' type="module"' : ''
			const closeScript = '<' + '/script>'
			result = result.replace(match[0], `<script${typeAttr}>\n${code}\n${closeScript}`)
		} catch (error) {
			logPlugin(`内联脚本失败: ${src}`, 'error', pluginId)
		}
	}

	// <link rel="stylesheet" href="..."> → <style>
	const stylePattern = /<link\b([^>]*?)>/g
	for (const match of [...result.matchAll(stylePattern)]) {
		const tag = match[0]
		if (!/rel\s*=\s*["']stylesheet["']/i.test(tag)) continue
		const href = tag.match(/href\s*=\s*["']([^"']+)["']/i)?.[1]
		if (!href) continue
		if (/^https?:/i.test(href)) continue
		try {
			const css = await invoke<string>('plugin:extensions|extensions_read', {
				id: pluginId,
				file: href,
			})
			result = result.replace(tag, `<style>\n${css}\n</style>`)
		} catch (error) {
			logPlugin(`内联样式失败: ${href}`, 'error', pluginId)
		}
	}

	return result
}

async function loadDocument(pluginId: string, entry: string) {
	try {
		logPlugin(`开始加载页面: ${entry}`, 'info', pluginId)
		console.log('[sandbox] loadDocument', pluginId, entry)
		const html = await invoke<string>('plugin:extensions|extensions_read', {
			id: pluginId,
			file: entry,
		})
		const openTag = '<' + 'script>'
		const closeTag = '<' + '/script>'
		console.log('[sandbox] html loaded', pluginId, 'length=', html.length)
		const withAssets = await inlineAssets(pluginId, html)
		const injected = `${openTag}${SANDBOX_SDK}${closeTag}`
		// <base> 让相对路径指向插件 zip 内的资源
		// Windows/Android：自定义协议实际走 http://ext.localhost/<id>/
		const isWindowsLike = /Windows|Android/i.test(navigator.userAgent)
		const baseOrigin = isWindowsLike
			? `http://ext.localhost/${pluginId}/`
			: `ext://${pluginId}/`
		const baseTag = `<base href="${baseOrigin}">`
		logPlugin(`页面已内联资源，长度 ${withAssets.length}`, 'info', pluginId)
		console.log('[sandbox] document ready', pluginId, 'length=', withAssets.length)
		documents[pluginId] = /<head[^>]*>/i.test(withAssets)
			? withAssets.replace(/<head[^>]*>/i, (match) => match + baseTag + injected)
			: baseTag + injected + withAssets
	} catch (error) {
		console.error('[sandbox] loadDocument failed', pluginId, error)
		logPlugin(`读取页面失败: ${String(error)}`, 'error', pluginId)
		console.warn('[sandbox] 读取插件页面失败', pluginId, error)
	}
}

async function callHost(pluginId: string, method: string, params: Record<string, unknown>) {
	const extension = pluginState.extensions.find((item) => item.id === pluginId)
	const permissions = extension?.permissions ?? []

	if (method === 'instances.launch' && !permissions.includes('instances.launch')) {
		throw new Error('插件权限不足: instances.launch')
	}
	if (method === 'net.fetch' && !permissions.includes('network')) {
		throw new Error('插件权限不足: network')
	}

	switch (method) {
		case 'instances.list':
			return invoke('plugin:instance|instance_list')
		case 'instances.launch':
			return invoke('plugin:instance|instance_run', {
				instanceId: params.instanceId,
				serverAddress: null,
				overrides: null,
			})
		case 'storage.get':
			return invoke('plugin:extensions|extensions_storage_get', { id: pluginId })
		case 'storage.set':
			return invoke('plugin:extensions|extensions_storage_set', {
				id: pluginId,
				value: params.value ?? {},
			})
		case 'net.fetch':
			return invoke('plugin:extensions|plugin_net_fetch', {
				url: String(params.url ?? ''),
				method: (params.method as string | undefined) ?? null,
				headers: (params.headers as Record<string, string> | undefined) ?? null,
				body: (params.body as string | undefined) ?? null,
				timeoutMs: (params.timeoutMs as number | undefined) ?? null,
			})
		case 'asset.read':
			return invoke<string>('plugin:extensions|extensions_read_binary', {
				id: pluginId,
				file: String(params.path ?? ''),
			})
		case 'asset.image': {
			const path = String(params.path ?? '')
			const base64 = await invoke<string>('plugin:extensions|extensions_read_binary', {
				id: pluginId,
				file: path,
			})
			const ext = (path.split('.').pop() ?? '').toLowerCase()
			const mime =
				ext === 'jpg' || ext === 'jpeg' ? 'image/jpeg'
				: ext === 'webp' ? 'image/webp'
				: ext === 'svg' ? 'image/svg+xml'
				: ext === 'gif' ? 'image/gif'
				: 'image/png'
			return `data:${mime};base64,${base64}`
		}
		case 'canvas.draw': {
			const canvasId = String(params.id ?? 'canvas')
			if (params.clear) clearCanvas(canvasId)
			const ops = Array.isArray(params.ops) ? (params.ops as CanvasOp[]) : []
			setCanvasOps(canvasId, ops)
			return true
		}
		case 'clipboard.write':
			await navigator.clipboard.writeText(String(params.text ?? ''))
			return true
		case 'open.url': {
			const url = String(params.url ?? '')
			if (!(url.startsWith('https://') || url.startsWith('http://'))) {
				throw new Error('仅允许 http/https')
			}
			await invoke('plugin:opener|open_url', { url })
			return true
		}
		case 'notify':
			pushPluginToast(String(params.message ?? ''), (params.type as 'info') ?? 'info')
			return null
		default:
			throw new Error(`不支持的方法: ${method}`)
	}
}

function findPluginIdBySource(source: MessageEventSource | null) {
	for (const [pluginId, frame] of sandboxFrames) {
		if (frame.contentWindow === source) return pluginId
	}
	return null
}

function onMessage(event: MessageEvent) {
	const data = event.data
	if (!data || data.__aircore !== true) return
	const pluginId = findPluginIdBySource(event.source)
	if (!pluginId) return

	if (data.kind === 'plugin.error') {
		logPlugin(`插件脚本错误: ${data.message}`, 'error', pluginId)
		if (data.stack) logPlugin(String(data.stack).split('\n').slice(0, 4).join(' | '), 'error', pluginId)
		return
	}

	if (data.kind === 'ui') {
		console.log('[sandbox] ui message', pluginId, 'slot=', data.slot ?? 'page', 'nodes=', (data.tree ?? []).length)
		logPlugin(`收到界面描述: ${data.slot ?? 'page'}（${(data.tree ?? []).length} 节点）`, 'info', pluginId)
		const slot = data.slot === 'sidebar' || data.slot === 'settings' ? data.slot : 'page'
		setSandboxSlot(pluginId, slot, data.tree ?? [], data.title)
		return
	}

	if (data.kind === 'api') {
		callHost(pluginId, data.method, data.params ?? {})
			.then((result) =>
				sandboxFrames.get(pluginId)?.contentWindow?.postMessage(
					{ __aircore: true, kind: 'api.response', reqId: data.reqId, result },
					'*',
				),
			)
			.catch((error) =>
				sandboxFrames.get(pluginId)?.contentWindow?.postMessage(
					{ __aircore: true, kind: 'api.response', reqId: data.reqId, error: String(error) },
					'*',
				),
			)
	}
}

// 新增 / 删除插件时加载对应文档
// 插件页面延迟加载：启动时先把主界面显示出来，
// 空闲后再读取插件 HTML，避免拖慢启动
const documentsReady = ref(false)

function loadAllDocuments() {
	for (const extension of sandboxPlugins.value) {
		if (!documents[extension.id]) void loadDocument(extension.id, extension.entry ?? 'index.html')
	}
}

watch(
	() => pluginState.extensions.map((extension) => `${extension.id}:${extension.entry}:${extension.enabled}`).join('|'),
	() => {
		if (documentsReady.value) loadAllDocuments()
	},
	{ immediate: true },
)


onMounted(() => {
	const start = () => {
		if (documentsReady.value) return
		documentsReady.value = true
		// 当前页面就是插件页时，立即加载，不等空闲
		loadAllDocuments()
	}
	// 路由已在插件页则立即加载
	if (window.location.hash.startsWith('#/ext/') || window.location.pathname.startsWith('/ext/')) {
		start()
		return
	}
	if ('requestIdleCallback' in window) {
		requestIdleCallback(() => start(), { timeout: 2500 })
	} else {
		setTimeout(start, 1200)
	}
})

// 热重载：重新读取 HTML
watch(
	() => Array.from(sandboxReloadToken.entries()).map(([id, token]) => `${id}:${token}`).join('|'),
	() => {
		for (const extension of sandboxPlugins.value) {
			void loadDocument(extension.id, extension.entry ?? 'index.html')
		}
	},
)

onMounted(() => {
	window.addEventListener('message', onMessage)
	console.log(
		'[sandbox] host mounted, plugins =',
		pluginState.extensions.map((extension) => `${extension.id}(enabled=${extension.enabled !== false})`),
	)
})

// 插件卸载 / 停用后，清掉它的 HTML 与 iframe 引用，避免残留
watch(
	() => sandboxPlugins.value.map((extension) => extension.id),
	(ids) => {
		console.log('[sandbox] enabled plugin ids changed =', ids)
		const alive = new Set(ids)
		for (const id of Object.keys(documents)) {
			if (!alive.has(id)) {
				delete documents[id]
				sandboxFrames.delete(id)
			}
		}
	},
)

onUnmounted(() => {
	window.removeEventListener('message', onMessage)
})
</script>

<template>
	<div class="flex flex-col gap-3">
		<Admonition v-if="error" type="critical" header="插件运行出错" :body="error" />
		<template v-else>
			<div v-for="(node, index) in rendered" :key="index" class="contents">
				<component :is="asComponent(node)" />
			</div>
		</template>
		<div v-if="reloadStatus" class="text-xs text-secondary">{{ reloadStatus }}</div>
		<iframe :key="frameKey" ref="frame" class="hidden" sandbox="allow-scripts" :srcdoc="srcdoc" />
	</div>
</template>

<script setup lang="ts">
import { Admonition } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { onMounted, onUnmounted, ref, watch } from 'vue'

import { renderUiTree, type UiNode } from '@/plugins/ui-renderer'
import { SANDBOX_SDK } from '@/plugins/sandbox-sdk'
import { pushPluginToast, sandboxReloadToken } from '@/plugin-host'

const props = defineProps<{ pluginId: string; entry: string; permissions?: string[] }>()

const frame = ref<HTMLIFrameElement | null>(null)
const srcdoc = ref('')
const frameKey = ref(0)
const reloadStatus = ref('')
const error = ref<string | null>(null)
const tree = ref<UiNode[]>([])

const rendered = ref<unknown[]>([])
watch(tree, () => {
	rendered.value = renderUiTree(tree.value, sendEvent)
})

function asComponent(node: unknown) {
	return node
}

function sendEvent(id: string, payload?: unknown) {
	if (!id) return
	frame.value?.contentWindow?.postMessage({ __aircore: true, kind: 'event', id, payload }, '*')
}

const METHOD_PERMISSIONS: Record<string, string[]> = {
	'instances.launch': ['instances.launch'],
}

async function callHost(method: string, params: Record<string, unknown> = {}) {
	const required = METHOD_PERMISSIONS[method]
	if (required && !required.every((permission) => (props.permissions ?? []).includes(permission))) {
		throw new Error(`插件权限不足: ${method}（需要 ${required.join(', ')}）`)
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
		case 'notify': {
			pushPluginToast(String(params.message ?? ''), (params.type as 'info') ?? 'info')
			return null
		}
		default:
			throw new Error(`不支持的方法: ${method}`)
	}
}

function onMessage(event: MessageEvent) {
	const data = event.data
	if (!data || data.__aircore !== true) return
	if (event.source !== frame.value?.contentWindow) return

	if (data.kind === 'ui') {
		tree.value = data.tree ?? []
		return
	}

	if (data.kind === 'api') {
		callHost(data.method, data.params)
			.then((result) => {
				frame.value?.contentWindow?.postMessage(
					{ __aircore: true, kind: 'api.response', reqId: data.reqId, result },
					'*',
				)
			})
			.catch((err: unknown) => {
				frame.value?.contentWindow?.postMessage(
					{ __aircore: true, kind: 'api.response', reqId: data.reqId, error: String(err) },
					'*',
				)
			})
		return
	}
}

watch(
	() => sandboxReloadToken.get(props.pluginId) ?? 0,
	(value) => {
		reloadStatus.value = `已收到重载信号 #${value}，页面重建中`
		frameKey.value += 1
		void load()
	},
)

async function load() {
	console.info('[sandbox] 重新加载插件页面:', props.pluginId)
	reloadStatus.value = `已重新读取页面 ${new Date().toLocaleTimeString()}`
	try {
		const html = await invoke<string>('plugin:extensions|extensions_read', {
			id: props.pluginId,
			file: props.entry,
		})
		// 把 SDK 注入到页面里，插件只需要调 window.AirCore
		const openTag = '<' + 'script>'
		const closeTag = '<' + '/script>'
		const injected = `${openTag}${SANDBOX_SDK}${closeTag}`
		if (/<head[^>]*>/i.test(html)) {
			srcdoc.value = html.replace(/<head[^>]*>/i, (match) => match + injected)
		} else {
			srcdoc.value = injected + html
		}
	} catch (err) {
		error.value = String(err)
	}
}

onMounted(() => {
	window.addEventListener('message', onMessage)
	void load()
})

onUnmounted(() => {
})

onUnmounted(() => {
	window.removeEventListener('message', onMessage)
})

defineExpose({ sendEvent })
</script>

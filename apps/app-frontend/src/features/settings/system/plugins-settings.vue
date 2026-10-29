<template>
	<div class="flex flex-col gap-6">
		<section class="border-0 border-b border-solid border-divider pb-6">
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.title) }}</h2>
			<p class="m-0 mt-1 text-secondary">
				{{ formatMessage(messages.description, { count: total }) }}
			</p>
			<div class="mt-3">
				<Button type="outlined" @click="openFolder">{{ formatMessage(messages.openFolder) }}</Button>
			</div>
		</section>

		<Admonition
			v-if="diagnostics.length > 0"
			type="warning"
			:header="formatMessage(messages.diagnosticsTitle)"
			:body="diagnostics.join('\\n')"
		/>

		<Admonition
			v-if="pluginError"
			type="critical"
			:header="formatMessage(messages.errorTitle)"
			:body="pluginError"
		/>

		<div v-if="total === 0" class="text-secondary">{{ formatMessage(messages.empty) }}</div>

		<div v-else class="flex flex-col gap-2">
			<template v-for="extension in extensions" :key="extension.id">
				<div
					class="flex items-center justify-between gap-4 rounded-xl border border-solid border-surface-4 bg-surface-3 p-4"
				>
					<div class="min-w-0 cursor-pointer" @click="toggleExpanded(extension.id)">
						<div class="flex items-center gap-2 font-semibold text-contrast">
							<span class="text-secondary">{{ isExpanded(extension.id) ? '▾' : '▸' }}</span>
							{{ extension.name || extension.id }}
						</div>
						<div class="truncate text-secondary">{{ describe(extension) }}</div>
					</div>
					<div class="flex shrink-0 items-center gap-3" @click.stop>
						<Toggle
							:model-value="extension.enabled !== false"
							@update:model-value="(value) => setEnabled(extension, value)"
						/>
						<Button type="outlined" color="red" @click="remove(extension)">{{ formatMessage(messages.uninstall) }}</Button>
					</div>
				</div>
				<div
					v-if="isExpanded(extension.id)"
					class="ml-4 flex flex-col gap-3 border-0 border-l-2 border-solid border-surface-4 pl-4 pb-2"
				>
					<component
						:is="node"
						v-for="(node, index) in settingsNodes(extension.id)"
						:key="index"
					/>
					<div v-if="settingsNodes(extension.id).length === 0" class="text-secondary">
						{{ formatMessage(messages.noConfig) }}
					</div>
				</div>
			</template>
		</div>
	</div>
</template>

<script setup lang="ts">
import {
	Admonition,
	Button,
	Toggle,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { appDataDir } from '@tauri-apps/api/path'
import { computed, onMounted, onUnmounted, reactive } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { injectAppSettingsModal } from '@/features/settings/app-settings-modal/context'

import {
	bumpSandboxToken,
	dispatchSandboxEvent,
	pluginState,
	registerSandboxNav,
	sandboxTrees,
	unloadPlugin,
} from '@/plugin-host'
import { renderUiTree, type UiNode } from '@/plugins/ui-renderer'

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()

const messages = defineMessages({
	title: { id: 'app.plugins.title', defaultMessage: 'Plugins' },
	description: {
		id: 'app.plugins.description',
		defaultMessage:
			'Plugins are zip files placed in the extensions folder (zip name = plugin id). Total: {count}.',
	},
	openFolder: { id: 'app.plugins.open-folder', defaultMessage: 'Open plugins folder' },
	errorTitle: { id: 'app.plugins.error-title', defaultMessage: 'Plugin error' },
	empty: { id: 'app.plugins.empty', defaultMessage: 'No plugins installed yet.' },
	uninstall: { id: 'app.plugins.uninstall', defaultMessage: 'Uninstall' },
	noConfig: {
		id: 'app.plugins.no-config',
		defaultMessage: 'This plugin has no configurable options.',
	},
	meta: { id: 'app.plugins.meta', defaultMessage: '{id} \u00b7 entry {entry} \u00b7 API v{version}' },
	openFolderFailed: {
		id: 'app.plugins.open-folder-failed',
		defaultMessage: 'Failed to open folder',
	},
	uninstalled: { id: 'app.plugins.uninstalled', defaultMessage: 'Uninstalled' },
	saved: { id: 'app.plugins.saved', defaultMessage: 'Plugin settings saved' },
	saveFailed: {
		id: 'app.plugins.save-failed',
		defaultMessage: 'Failed to save plugin settings',
	},
	diagnosticsTitle: {
		id: 'app.plugins.diagnostics-title',
		defaultMessage: 'Skipped plugin files',
	},
	listFailed: {
		id: 'app.plugins.list-failed',
		defaultMessage: 'Failed to load plugin list',
	},
})
const settingsModal = injectAppSettingsModal()
const router = useRouter()
const route = useRoute()

import { ref } from 'vue'

const extensions = ref<{ id: string; name?: string; entry?: string; apiVersion?: number; enabled?: boolean }[]>([])
const pluginError = computed(() => pluginState?.error ?? '')
const diagnostics = ref<string[]>([])
const total = computed(() => extensions.value.length)

const expanded = reactive(new Set<string>())
const savedConfigs = reactive(new Map<string, Record<string, string>>())

function isExpanded(pluginId: string) {
	return expanded.has(pluginId)
}

function toggleExpanded(pluginId: string) {
	if (expanded.has(pluginId)) expanded.delete(pluginId)
	else expanded.add(pluginId)
}

function describe(extension: { id: string; entry?: string; apiVersion?: number; enabled?: boolean }) {
	const entry = extension.entry || 'index.html'
	const version = extension.apiVersion ?? 1
	return formatMessage(messages.meta, { id: extension.id, entry, version })
}

function draftKey(pluginId: string, field: string) {
	return `${pluginId}::${field}`
}

function modelOf(pluginId: string) {
	const model: Record<string, string> = {}
	const values = configValues as Record<string, string>
	for (const key of Object.keys(values)) {
		const [pid, field] = key.split('::')
		if (pid === pluginId && field) model[field] = values[key]
	}
	return model
}

function settingsNodes(pluginId: string): unknown[] {
	try {
		const trees = sandboxTrees?.get(pluginId)
		if (!trees?.settings?.tree?.length) return []
		return renderUiTree(
			trees.settings.tree as UiNode[],
			(eventId: string, payload?: unknown) => {
				if (typeof payload === 'string' && eventId) {
					const key = draftKey(pluginId, eventId)
					configValues[key] = payload
				}
				dispatchSandboxEvent(pluginId, eventId, payload)
			},
			modelOf(pluginId),
		)
	} catch (error) {
		console.warn('[plugins] 渲染插件配置失败', pluginId, error)
		return []
	}
}

async function openFolder() {
	try {
		const dataDir = await appDataDir()
		await invoke('plugin:utils|open_path', { path: `${dataDir}/extensions` })
	} catch (error) {
		addNotification({ type: 'error', title: formatMessage(messages.openFolderFailed), text: String(error) })
	}
}

function leavePluginPage(pluginId: string) {
	if (route.path === `/ext/${pluginId}`) {
		void router.replace('/')
	}
}

async function setEnabled(extension: { id: string; name?: string; entry?: string }, enabled: boolean) {
	await invoke('plugin:extensions|extensions_set_enabled', { id: extension.id, enabled })
	const record = extensions.value.find((item) => item.id === extension.id)
	if (record) record.enabled = enabled

	// 同步给插件宿主，否则 SandboxHost 不会创建/重建它的 iframe
	const hostRecord = pluginState.extensions.find((item) => item.id === extension.id)
	if (hostRecord) {
		hostRecord.enabled = enabled
	} else if (enabled) {
		pluginState.extensions = [
			...pluginState.extensions,
			{ ...extension, enabled, apiVersion: 1 },
		]
	}

	await loadSavedConfigs()

	if (enabled) {
		registerSandboxNav(extension)
		bumpSandboxToken(extension.id)
	} else {
		leavePluginPage(extension.id)
		await unloadPlugin(extension.id)
	}
}

async function remove(extension: { id: string; name?: string }) {
	leavePluginPage(extension.id)
	await unloadPlugin(extension.id)
	await invoke('plugin:extensions|extensions_remove', { id: extension.id })

	// 清理本地列表与配置缓存（配置文件已在 Rust 侧随插件一起删除）
	extensions.value = extensions.value.filter((item) => item.id !== extension.id)
	const prefix = `${extension.id}::`
	for (const key of Object.keys(configValues)) {
		if (key.startsWith(prefix)) delete configValues[key]
	}
	for (const key of Object.keys(savedConfigValues)) {
		if (key.startsWith(prefix)) delete savedConfigValues[key]
	}
	savedConfigs.delete(extension.id)
	if (pluginState?.extensions) pluginState.extensions = pluginState.extensions.filter((item) => item.id !== extension.id)
	addNotification({ type: 'info', title: formatMessage(messages.uninstalled), text: extension.name || extension.id })
}

async function loadSavedConfigs() {
	for (const extension of extensions.value) {
		try {
			const data = await invoke('plugin:extensions|extensions_storage_get', { id: extension.id })
			const flat: Record<string, string> = {}
			if (data && typeof data === 'object') {
				for (const [key, value] of Object.entries(data as Record<string, unknown>)) {
					flat[key] = typeof value === 'string' ? value : JSON.stringify(value)
				}
			}
			savedConfigs.set(extension.id, flat)
			for (const [key, value] of Object.entries(flat)) {
				configValues[draftKey(extension.id, key)] = value
			savedConfigValues[draftKey(extension.id, key)] = value
			}
		} catch (error) {
			console.warn('[plugins] 读取插件配置失败', extension.id, error)
		}
	}
}

// 插件配置：与设置弹窗的全局保存条接入
const configValues = reactive<Record<string, string>>({})
const savedConfigValues = reactive<Record<string, string>>({})
const savingConfig = ref(false)

function configChanges(): Record<string, string> {
	const changed: Record<string, string> = {}
	for (const key of Object.keys(configValues)) {
		if (configValues[key] !== savedConfigValues[key]) changed[key] = configValues[key]
	}
	return changed
}

function configHasChanges() {
	// 读取响应式对象，保证弹窗能追踪到变化
	return Object.keys(configChanges()).length > 0
}

function resetConfigChanges() {
	for (const key of Object.keys(configValues)) {
		configValues[key] = savedConfigValues[key] ?? ''
	}
}

async function saveConfigChanges() {
	const changes = configChanges()
	if (Object.keys(changes).length === 0) return
	const byPlugin = new Map<string, Record<string, string>>()
	for (const key of Object.keys(changes)) {
		const [pluginId, field] = key.split('::')
		if (!pluginId || !field) continue
		if (!byPlugin.has(pluginId)) byPlugin.set(pluginId, {})
		byPlugin.get(pluginId)![field] = changes[key]
	}
	savingConfig.value = true
	try {
		for (const [pluginId, values] of byPlugin) {
			// 合并已保存的配置，避免只写变更字段导致其他键丢失
			const fieldPrefix = `${pluginId}::`
			const existing: Record<string, string> = {}
			for (const key of Object.keys(savedConfigValues)) {
				if (key.startsWith(fieldPrefix)) existing[key.slice(fieldPrefix.length)] = savedConfigValues[key]
			}
			const merged = { ...existing, ...values }
			const ok = await invoke<boolean>('plugin:extensions|extensions_storage_set', {
				id: pluginId,
				value: merged,
			})
			if (ok === false) {
				throw new Error(`write failed: ${pluginId}`)
			}
			savedConfigs.set(pluginId, values)
			for (const [field, value] of Object.entries(values)) {
				savedConfigValues[`${pluginId}::${field}`] = value
				configValues[`${pluginId}::${field}`] = value
			}
		}
		addNotification({ type: 'success', title: formatMessage(messages.saved), text: String(byPlugin.size) })
	} catch (error) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.saveFailed),
			text: String(error),
		})
		console.error('[plugins] 保存插件配置失败', error)
	} finally {
		savingConfig.value = false
	}
}

async function refreshExtensions() {
	try {
		const list = await invoke<{ id: string; name?: string; entry?: string; apiVersion?: number; enabled?: boolean }[]>(
			'plugin:extensions|extensions_list',
		)
		extensions.value = list
		diagnostics.value = await invoke<string[]>('plugin:extensions|extensions_diagnostics').catch(
			() => [],
		) ?? []
	} catch (error) {
		addNotification({ type: 'error', title: formatMessage(messages.listFailed), text: String(error) })
	}
}

onMounted(() => {
	void refreshExtensions().then(() => {
		void loadSavedConfigs()
		registerController()
	})
})

function registerController() {
	settingsModal.registerUnsavedChangesController({
		hasChanges: configHasChanges,
		getOriginal: () => ({ ...savedConfigValues }),
		getModified: () => ({ ...configValues }),
		isSaving: () => savingConfig.value,
		reset: resetConfigChanges,
		save: saveConfigChanges,
	})
}

onUnmounted(() => {
	settingsModal.registerUnsavedChangesController(null)
})
</script>

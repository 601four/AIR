/**
 * 插件宿主：把启动器的 Vue 与 UI 组件库暴露给外部插件，
 * 插件只需要一个普通 JS 文件，就能渲染和主界面完全一致的 UI。
 */
import * as Vue from 'vue'
import { invoke } from '@tauri-apps/api/core'
import * as UI from '@modrinth/ui'
import * as ICONS from '@modrinth/assets'
import { markRaw, reactive, shallowReactive } from 'vue'

export interface PluginPanel {
	id: string
	title: string
	icon?: unknown
	component: unknown
	/** 所属插件 id，用于错误隔离与权限 */
	pluginId?: string
}

/** API 契约版本：插件 manifest 里声明的 apiVersion 必须相容 */
export const PLUGIN_API_VERSION = 1

/** 敏感能力 → 需要的权限名 */
const PERMISSION_BY_COMMAND: Record<string, string[]> = {
	'plugin:instance|instance_run': ['instances.launch'],
	'plugin:instance|instance_kill': ['instances.launch'],
}

const panels = shallowReactive<PluginPanel[]>([])
const loadedPlugins = new Set<string>()

export interface PluginLogEntry {
	at: string
	level: 'info' | 'warn' | 'error'
	pluginId?: string
	message: string
}

export interface PluginAccountProfile {
	name: string
	uuid: string
	accessToken: string
	type: 'offline' | 'custom'
	pluginId?: string
}

/** 插件注册的账号提供者 */
export const accountProviders: {
	id: string
	name: string
	login: (username: string, password?: string) => Promise<PluginAccountProfile>
}[] = reactive([])

/** 插件提供的账号（保存在本地） */
export const pluginAccounts = reactive<PluginAccountProfile[]>(
	JSON.parse(localStorage.getItem('aircore-plugin-accounts') ?? '[]'),
)

export function persistPluginAccounts() {
	localStorage.setItem('aircore-plugin-accounts', JSON.stringify(pluginAccounts))
}

export interface PluginToast {
	id: number
	message: string
	type: 'info' | 'success' | 'warning' | 'error'
}

/** 插件弹出的全局通知 */
export const pluginToasts = reactive<PluginToast[]>([])

/** 沙箱插件投影到各个插槽的 UI 树 */
export interface SandboxTrees {
	page?: unknown[]
	sidebar?: unknown[]
	settings?: { title: string; tree: unknown[] }
	meta?: { title?: string }
}

export const sandboxTrees = reactive(new Map<string, SandboxTrees>())

/** 每个沙箱插件的 iframe（由 SandboxHost 注册） */
export const sandboxFrames = new Map<string, HTMLIFrameElement>()

/** 插件页面的表单值（保持重渲染不丢） */
export const sandboxModels = reactive(new Map<string, Record<string, string>>())

export function getSandboxModel(pluginId: string): Record<string, string> {
	return sandboxModels.get(pluginId) ?? {}
}

export function setSandboxValue(pluginId: string, key: string, value: string) {
	const current = sandboxModels.get(pluginId) ?? {}
	sandboxModels.set(pluginId, { ...current, [key]: value })
}

/** 向某个沙箱插件的 iframe 回传界面事件 */
export function dispatchSandboxEvent(pluginId: string, eventId: string, payload?: unknown) {
	if (!eventId) return
	sandboxFrames
		.get(pluginId)
		?.contentWindow?.postMessage({ __aircore: true, kind: 'event', id: eventId, payload }, '*')
}

export function setSandboxSlot(
	pluginId: string,
	slot: 'page' | 'sidebar' | 'settings',
	tree: unknown[],
	title?: string,
) {
	const current = sandboxTrees.get(pluginId) ?? {}
	if (slot === 'settings') {
		current.settings = { title: title || '插件设置', tree }
	} else {
		current[slot] = tree
	}
	sandboxTrees.set(pluginId, { ...current })
}

/** 清理某个沙箱插件的所有投影与 iframe 引用 */
export function clearSandbox(pluginId: string) {
	sandboxTrees.delete(pluginId)
	sandboxFrames.delete(pluginId)
	sandboxModels.delete(pluginId)
}

/** 沙箱插件的重载信号（变化时 iframe 重建） */
export const sandboxReloadToken = reactive(new Map<string, number>())

export function bumpSandboxToken(pluginId: string) {
	sandboxReloadToken.set(pluginId, (sandboxReloadToken.get(pluginId) ?? 0) + 1)
}
let toastSeq = 0

const PLUGIN_AUTOLOAD_KEY = 'aircore-plugins-autoload'

/** 启动时是否自动加载插件（默认开） */
export function pluginsAutoLoad(): boolean {
	try {
		return localStorage.getItem(PLUGIN_AUTOLOAD_KEY) !== '0'
	} catch {
		return true
	}
}

export function setPluginsAutoLoad(enabled: boolean) {
	localStorage.setItem(PLUGIN_AUTOLOAD_KEY, enabled ? '1' : '0')
}

type HostNotifier = (message: string, type: 'info' | 'success' | 'warning' | 'error') => void
let hostNotifier: HostNotifier | null = null

/** 启动器自己的通知系统（由 App.vue 注入），插件优先用它 */
export function setHostNotifier(notifier: HostNotifier) {
	hostNotifier = notifier
}

let shuttingDown = false
window.addEventListener('beforeunload', () => {
	shuttingDown = true
})

const MAX_TOASTS = 4

export function pushPluginToast(message: string, type: PluginToast['type'] = 'info') {
	// 应用正在关闭：丢弃所有插件通知
	if (shuttingDown) return
	// 优先用启动器自己的通知系统，避免两套通知重叠
	if (hostNotifier) {
		hostNotifier(message, type)
		return
	}
	// 同一条消息不重复堆叠
	const existing = pluginToasts.find((toast) => toast.message === message)
	if (existing) return
	const id = ++toastSeq
	pluginToasts.push({ id, message, type })
	while (pluginToasts.length > MAX_TOASTS) pluginToasts.shift()
	setTimeout(() => {
		const index = pluginToasts.findIndex((toast) => toast.id === id)
		if (index !== -1) pluginToasts.splice(index, 1)
	}, 4000)
}

/** 被插件隐藏的内置导航项（home / browse / skins / screenshots / gallery） */
export const hiddenNavIds = reactive(new Set<string>())
const hiddenByPlugin = new Map<string, Set<string>>()

export function isNavHidden(id: string): boolean {
	return hiddenNavIds.has(id)
}

/** 为沙箱插件注册（或刷新）其导航项 */
export function registerSandboxNav(extension: { id: string; name?: string; icon?: string }) {
	const existing = pluginRegistry.navItems.find((item) => item.id === extension.id)
	if (existing) {
		existing.title = extension.name ?? existing.title
		existing.icon = resolvePluginIcon(extension.icon)
		return
	}
	pluginRegistry.navItems.push({
		id: extension.id,
		title: extension.name ?? extension.id,
		pluginId: extension.id,
		icon: resolvePluginIcon(extension.icon),
		// 沙箱插件的页面由 ExtensionPanel 渲染，这里给个空组件避免 auto-unwrap 警告
		component: markRaw({}),
	})
}

/** 把 manifest 里的图标名（如 "SparklesIcon"）解析成宿主图标组件 */
export function resolvePluginIcon(name: string | undefined) {
	if (!name) return undefined
	return (ICONS as Record<string, unknown>)[name]
}

export function hideNavItem(id: string) {
	hiddenNavIds.add(id)
	logPlugin(`隐藏导航项: ${id}`, 'info')
}

export function showNavItem(id: string) {
	hiddenNavIds.delete(id)
	logPlugin(`恢复导航项: ${id}`, 'info')
}

export const pluginLogs: PluginLogEntry[] = reactive([])

export function logPlugin(
	message: string,
	level: PluginLogEntry['level'] = 'info',
	pluginId?: string,
) {
	pluginLogs.push({
		at: new Date().toLocaleTimeString(),
		level,
		pluginId,
		message,
	})
	if (pluginLogs.length > 500) pluginLogs.shift()
}

export const pluginState = reactive({
	extensions: [] as { id: string; entry?: string }[],
	error: null as string | null,
})

export const pluginRegistry = reactive({
	panels,
	navItems: [] as PluginPanel[],
	sidebarSections: [] as PluginPanel[],
	settingsSections: [] as PluginPanel[],
})

const ALLOWED_COMMANDS = [
	'plugin:instance|instance_list',
	'plugin:instance|instance_get',
	'plugin:instance|instance_run',
	'plugin:instance|instance_kill',
	'plugin:utils|get_os',
	'plugin:settings|settings_get',
	'plugin:extensions|extensions_list',
]

function safeInvoke(command: string, args?: Record<string, unknown>) {
	if (!ALLOWED_COMMANDS.includes(command)) {
		return Promise.reject(new Error(`插件未获得调用权限：${command}`))
	}
	return invoke(command, args)
}

interface PluginLifecycle {
	onEnable?: () => void | Promise<void>
	onDisable?: () => void | Promise<void>
	dispose?: () => void
}

const pluginLifecycles = new Map<string, PluginLifecycle>()
const pluginListeners = new Map<string, Set<(...args: unknown[]) => void>>()

/** 向所有插件广播一个事件 */
export function emitPluginEvent(event: string, ...args: unknown[]) {
	for (const handler of pluginListeners.get(event) ?? []) {
		try {
			handler(...args)
		} catch (error) {
			console.warn(`[plugins] 事件处理失败: ${event}`, error)
		}
	}
}

/** 停用一个插件（为将来的插件管理预留） */
export async function disablePlugin(pluginId: string) {
	const lifecycle = pluginLifecycles.get(pluginId)
	if (!lifecycle) return
	await lifecycle.onDisable?.()
	lifecycle.dispose?.()
	pluginLifecycles.delete(pluginId)
	logPlugin('已停用', 'info', pluginId)
}

export function enablePlugin(pluginId: string) {
	const lifecycle = pluginLifecycles.get(pluginId)
	if (!lifecycle) return
	void lifecycle.onEnable?.()
}

/** 为单个插件创建独立的 API 上下文（权限、日志都隔离） */
export function createPluginContext(pluginId: string, permissions: string[] = []) {
	const granted = new Set(permissions)
	let lastNotify: { message: string; at: number } = { message: '', at: 0 }
	let pluginContext: ReturnType<typeof createPluginContext>
	const invokeWithPermission = (command: string, args?: Record<string, unknown>) => {
		const required = PERMISSION_BY_COMMAND[command]
		if (required && !required.every((permission) => granted.has(permission))) {
			logPlugin(`权限不足: ${command}（需要 ${required.join(', ')}）`, 'warn', pluginId)
			return Promise.reject(new Error(`插件权限不足: ${command}`))
		}
		return safeInvoke(command, args)
	}

	pluginContext = {
		pluginId,
		apiVersion: PLUGIN_API_VERSION,
		permissions: granted,
		vue: Vue,
		ui: UI,
		icons: ICONS,
		api: {
			invoke: invokeWithPermission,
		},
		registerNavItem: (item: PluginPanel) =>
			registerPanelEntry('navItems', { ...item, pluginId }),
		registerSidebarSection: (item: PluginPanel) =>
			registerPanelEntry('sidebarSections', { ...item, pluginId }),
		registerSettingsSection: (item: PluginPanel) =>
			registerPanelEntry('settingsSections', { ...item, pluginId }),
		registerPanel: (item: PluginPanel) => registerPanelEntry('panels', { ...item, pluginId }),
		log: (message: string, level: 'info' | 'warn' | 'error' = 'info') =>
			logPlugin(message, level, pluginId),

		/** 注册一个账号提供者（如离线登录） */
		registerAccountProvider(provider: {
			id: string
			name: string
			login: (username: string, password?: string) => Promise<PluginAccountProfile>
		}) {
			if (accountProviders.some((existing) => existing.id === provider.id)) return
			accountProviders.push(provider)
			logPlugin(`注册账号提供者: ${provider.name}`, 'info', pluginId)
		},

		/** 把账号写入启动器的账号列表 */
		addAccount(profile: PluginAccountProfile) {
			if (pluginAccounts.some((existing) => existing.uuid === profile.uuid)) return
			pluginAccounts.push({ ...profile, pluginId })
			persistPluginAccounts()
			logPlugin(`添加账号: ${profile.name}`, 'info', pluginId)
		},

		listAccounts: () => pluginAccounts,

		/** 发送全局通知（同一条短时间内去重） */
		notify(message: string, type: 'info' | 'success' | 'warning' | 'error' = 'info') {
			const now = Date.now()
			if (lastNotify.message === message && now - lastNotify.at < 3000) return
			lastNotify = { message, at: now }
			pushPluginToast(message, type)
			logPlugin(`通知: ${message}`, 'info', pluginId)
		},

		/** 隐藏内置导航项（如 screenshots / browse） */
		hideNavItem: (id: string) => {
			hiddenNavIds.add(id)
			if (!hiddenByPlugin.has(pluginId)) hiddenByPlugin.set(pluginId, new Set())
			hiddenByPlugin.get(pluginId)!.add(id)
			logPlugin(`隐藏导航项: ${id}`, 'info', pluginId)
		},
		showNavItem: (id: string) => {
			hiddenNavIds.delete(id)
			logPlugin(`恢复导航项: ${id}`, 'info', pluginId)
		},

		/** 以插件提供的身份启动实例（离线登录用） */
		launchWithOverrides(
			instanceId: string,
			overrides: {
				username?: string
				uuid?: string
				accessToken?: string
				skipSessionJoin?: boolean
				extraJvmArgs?: string[]
			},
		) {
			logPlugin(`以覆盖身份启动: ${instanceId}`, 'info', pluginId)
			return invoke('plugin:instance|instance_run', {
				instanceId,
				serverAddress: null,
				overrides,
			})
		},

		/** 监听宿主事件（如 instance.launch.before） */
		on(event: string, handler: (...args: unknown[]) => void) {
			if (!pluginListeners.has(event)) pluginListeners.set(event, new Set())
			pluginListeners.get(event)!.add(handler)
			return () => pluginListeners.get(event)?.delete(handler)
		},

		/** 插件入口：在这里注册 UI 与事件，并返回生命周期回调 */
		setup(factory: (api: unknown) => PluginLifecycle | void) {
			try {
				const lifecycle = factory(pluginContext) ?? {}
				pluginLifecycles.set(pluginId, lifecycle)
				logPlugin('已初始化', 'info', pluginId)
				void lifecycle.onEnable?.()
			} catch (error) {
				logPlugin(`初始化失败: ${String(error)}`, 'error', pluginId)
			}
		},
	}
	return pluginContext
}

type RegistryKey = 'panels' | 'navItems' | 'sidebarSections' | 'settingsSections'

function registerPanelEntry(key: RegistryKey, item: PluginPanel) {
	if (!item?.id || !item?.title || !item?.component) {
		logPlugin(`注册失败（缺少 id/title/component）`, 'warn', item?.pluginId)
		return
	}
	const list = pluginRegistry[key]
	if (list.some((existing) => existing.id === item.id)) {
		logPlugin(`重复注册已忽略: ${item.id}`, 'warn', item.pluginId)
		return
	}
	list.push({
		id: item.id,
		title: item.title,
		icon: item.icon ? markRaw(item.icon as object) : undefined,
		component: markRaw(item.component as object),
		pluginId: item.pluginId,
	})
	logPlugin(`注册面板: ${item.title}（${key}）`, 'info', item.pluginId)
}

export function installPluginHost() {
	const host = {
		vue: Vue,
		ui: UI,
		icons: ICONS,
		reactive,
		markRaw,

		/** 插件通过它注册一个嵌入主界面的面板 */
		registerPanel(panel: PluginPanel) {
			if (!panel?.id || !panel?.title || !panel?.component) return
			if (panels.some((existing) => existing.id === panel.id)) return
			panels.push({
				id: panel.id,
				title: panel.title,
				icon: panel.icon ? markRaw(panel.icon as object) : undefined,
				component: markRaw(panel.component as object),
			})
		},

		/** 注册一个左侧导航条目（打开面板页） */
		registerNavItem(item: PluginPanel) {
			if (!item?.id || !item?.title || !item?.component) return
			if (pluginRegistry.navItems.some((existing) => existing.id === item.id)) return
			pluginRegistry.navItems.push({
				id: item.id,
				title: item.title,
				icon: item.icon ? markRaw(item.icon as object) : undefined,
				component: markRaw(item.component as object),
			})
		},

		/** 注册一个右侧栏分区（直接嵌在侧栏里） */
		registerSidebarSection(section: PluginPanel) {
			if (!section?.id || !section?.title || !section?.component) return
			if (pluginRegistry.sidebarSections.some((existing) => existing.id === section.id)) return
			pluginRegistry.sidebarSections.push({
				id: section.id,
				title: section.title,
				component: markRaw(section.component as object),
			})
		},

		/** 注册一个设置页签（出现在设置弹窗里） */
		registerSettingsSection(section: PluginPanel) {
			if (!section?.id || !section?.title || !section?.component) return
			if (pluginRegistry.settingsSections.some((existing) => existing.id === section.id)) return
			pluginRegistry.settingsSections.push({
				id: section.id,
				title: section.title,
				component: markRaw(section.component as object),
			})
		},

		/** 插件上下文工厂（推荐：每个插件用它拿一份） */
		createPluginContext,

		/** 安全封装过的启动器 API */
		api: {
			invoke: safeInvoke,
			getOS: () => safeInvoke('plugin:utils|get_os'),
		},
	}

	;(window as unknown as Record<string, unknown>).AirCore = host
	return host
}

/** 卸下一个插件：调 dispose，并清掉它注册的所有面板 */
export async function unloadPlugin(pluginId: string) {
	const lifecycle = pluginLifecycles.get(pluginId)
	await lifecycle?.onDisable?.()
	lifecycle?.dispose?.()
	pluginLifecycles.delete(pluginId)
	loadedPlugins.delete(pluginId)

	for (const key of [
		'panels',
		'navItems',
		'sidebarSections',
		'settingsSections',
	] as const) {
		const list = pluginRegistry[key] as { id: string; pluginId?: string }[]
		for (let index = list.length - 1; index >= 0; index--) {
			if (list[index].pluginId === pluginId) list.splice(index, 1)
		}
	}
	for (const id of hiddenByPlugin.get(pluginId) ?? []) {
		hiddenNavIds.delete(id)
	}
	hiddenByPlugin.delete(pluginId)
	clearSandbox(pluginId)
	window.dispatchEvent(new CustomEvent('aircore:plugin-unloaded', { detail: pluginId }))
	logPlugin('已卸下', 'info', pluginId)
}

/** 沙箱插件重载时，刷新它的导航项（图标 / 标题） */
async function refreshSandboxNav(pluginId: string) {
	try {
		const list = (await invoke('plugin:extensions|extensions_list')) as {
			id: string
			name?: string
			icon?: string
		}[]
		const extension = list.find((item) => item.id === pluginId)
		pluginState.extensions = list
		const nav = pluginRegistry.navItems.find((item) => item.id === pluginId)
		if (nav && extension) {
			nav.title = extension.name ?? nav.title
			nav.icon = resolvePluginIcon(extension.icon)
		}
	} catch {
		// 忽略
	}
}

/** 热重载：监听 extensions 目录，变化就卸下并重新加载 */
export async function startPluginHotReload(getVersions: () => Promise<[string, number][]>) {
	let previous = new Map<string, number>()
	try {
		for (const [id, mtime] of await getVersions()) previous.set(id, mtime)
	} catch {
		// 忽略
	}

	const pendingReload = new Map<string, number>()
	const RELOAD_DELAY = 600

	window.setInterval(async () => {
		// 没有任何启用的插件时，不做任何 IPC 轮询
		if (!pluginState.extensions.some((ext) => ext.enabled !== false)) return

		let versions: [string, number][]
		try {
			versions = await getVersions()
		} catch {
			return
		}
		const next = new Map(versions)
		for (const [id, mtime] of next) {
			const before = previous.get(id)
			if (before !== undefined && before !== mtime) {
				pendingReload.set(id, Date.now())
			} else if (before === undefined) {
				// 新增插件：直接加载
				pendingReload.set(id, Date.now())
			}
		}
		// 目录被删除的插件：卸下并清理
		const removed = [...previous.keys()].filter((id) => !next.has(id))
		for (const id of removed) {
			logPlugin('插件目录已删除，卸下', 'warn', id)
			await unloadPlugin(id)
		}

		previous = next

		// 等文件写完再重载，避免读到写到一半的脚本
		for (const [id, at] of [...pendingReload]) {
			if (Date.now() - at < RELOAD_DELAY) continue
			pendingReload.delete(id)
			const extension = pluginState.extensions.find((ext) => ext.id === id)
			if (extension?.enabled === false) continue
			// 沙箱插件（HTML）：不注入脚本，发个重载信号让 iframe 重建
			if ((extension?.entry ?? '').endsWith('.html')) {
				void refreshSandboxNav(id)
				bumpSandboxToken(id)
				logPlugin('沙箱插件已变更，面板将重新加载', 'info', id)
				continue
			}
			logPlugin('检测到变更，重载中', 'info', id)
			await unloadPlugin(id)
			await loadPluginScript(id, extension?.entry ?? 'index.js')
		}
	}, 1500)
}

/** 把扩展目录里的 index.js 作为普通脚本加载进当前页面 */


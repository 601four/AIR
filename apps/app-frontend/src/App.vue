<script setup>
import { ModrinthApiError } from '@modrinth/api-client'
import {
	ArrowBigUpDashIcon,
	ArrowLeftRightIcon,
	BoxesIcon,
	CompassIcon,
	ImageIcon,
	LibraryIcon,
	LogInIcon,
	LogOutIcon,
	PlayIcon,
	PlusIcon,
	RefreshCwIcon,
	ServerStackIcon,
	SettingsIcon,
	ShirtIcon,
	SpinnerIcon,
	ToggleRightIcon,
	UserIcon,
	UserPlusIcon,
	XIcon,
} from '@modrinth/assets'
import {
	AccountSwitchOverlay,
	Admonition,
	Avatar,
	commonMessages,
	commonSettingsMessages,
	ContentInstallModal,
	ContentUpdaterModal,
	CreationFlowModal,
	defineMessages,
	I18nDebugPanel,
	IconButton,
	LoadingBar,
	NotificationPanel,
	PopupNotificationPanel,
	provideLoadingState,
	provideModalBehavior,
	provideModrinthClient,
	provideNotificationManager,
	providePageContext,
	providePopupNotificationManager,
	TeleportOverflowMenu,
	TooltipDirective,
	useDebugLogger,
	useFormatBytes,
	useHostingIntercom,
	UserRoleIcon,
	useVIntl,
} from '@modrinth/ui'
import { renderString } from '@modrinth/utils/parse'
import { useQueries, useQuery, useQueryClient } from '@tanstack/vue-query'
import { getVersion } from '@tauri-apps/api/app'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'
import { openUrl } from '@tauri-apps/plugin-opener'
import { type } from '@tauri-apps/plugin-os'
import { saveWindowState, StateFlags } from '@tauri-apps/plugin-window-state'
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import SandboxHost from '@/components/ui/SandboxHost.vue'
import { RouterView, useRoute, useRouter } from 'vue-router'

import { useRouteLoading } from '@/app/runtime/use-route-loading'
import AppSidebar from '@/app/shell/sidebar/index.vue'
import AppTitleBar from '@/app/shell/title-bar/index.vue'
import ErrorModal from '@/components/ui/ErrorModal.vue'
import AddServerToInstanceModal from '@/components/ui/install_flow/AddServerToInstanceModal.vue'
import UnknownPackWarningModal from '@/components/ui/install_flow/UnknownPackWarningModal.vue'
import IconEditorModal from '@/components/ui/instance_settings/icon-editor-modal/index.vue'
import MinecraftAuthErrorModal from '@/components/ui/minecraft-auth-error-modal/MinecraftAuthErrorModal.vue'
import MinecraftRequiredModal from '@/components/ui/minecraft-required-modal/MinecraftRequiredModal.vue'
import InstallToPlayModal from '@/components/ui/modal/InstallToPlayModal.vue'
import ModpackAlreadyInstalledModal from '@/components/ui/modal/ModpackAlreadyInstalledModal.vue'
import ModrinthAccountRequiredModal from '@/components/ui/modal/ModrinthAccountRequiredModal.vue'
import UpdateToPlayModal from '@/components/ui/modal/UpdateToPlayModal.vue'
import NavButton from '@/components/ui/NavButton.vue'
import QuickInstanceSwitcher from '@/components/ui/QuickInstanceSwitcher.vue'
import SharedInstanceInviteHandler from '@/components/ui/shared-instances/shared-instance-invite-handler/index.vue'
import SplashScreen from '@/components/ui/SplashScreen.vue'
import SurveyPopup from '@/components/ui/SurveyPopup.vue'
import { useCheckDisableMouseover } from '@/composables/macCssFix.js'
import { useAppEvent } from '@/composables/use-app-event'
import { applyAccentColor, useAccentColor } from '@/composables/use-accent-color'
import {
	installPluginHost,
	setHostNotifier,
	isNavHidden,
	pluginRegistry,
	startPluginHotReload,
	pluginState,
	pluginToasts,
	registerSandboxNav,
} from '@/plugin-host'
import PluginDiagnostics from '@/components/ui/PluginDiagnostics.vue'
import { useAppSettings } from '@/composables/use-app-settings.ts'
import { useError } from '@/composables/use-error.js'
import { useInstanceMetadataRefresh } from '@/composables/use-instance-metadata-refresh'
import { useQuickInstanceLimit } from '@/composables/use-quick-instance-limit.ts'
import { isDarkTheme, useTheme } from '@/composables/use-theme.ts'
import { config } from '@/config'
import { provideMinecraftAccounts } from '@/features/minecraft-accounts/context'
import AppSettingsModal from '@/features/settings/app-settings-modal/index.vue'
import { provideSettingsActions } from '@/features/settings/settings-actions'
import NewUpdateModal from '@/features/updates/new-update-modal/index.vue'
import { getAccountAppearance, rememberAccountAppearance } from '@/helpers/account-appearance.ts'
import {
	hide_ads_window,
	init_ads_window,
	perform_ads_consent_action,
	release_ads_window_hold,
	should_show_ads_consent_popup,
	take_ads_window_hold,
} from '@/helpers/ads.js'
import { debugAnalytics, initAnalytics, trackEvent } from '@/helpers/analytics'
import { check_reachable } from '@/helpers/auth.js'
import { get_user, get_user_many, get_version } from '@/helpers/cache.js'
import { gameSettingsQueryOptions } from '@/helpers/game-options'
import { install_create_modpack_instance, install_get_modpack_preview } from '@/helpers/install'
import {
	can_current_user_use_shared_instances,
	get as getInstance,
	run,
	set_global_synced_option,
} from '@/helpers/instance'
import { maxMemoryQueryOptions } from '@/helpers/jre.js'
import {
	get as getCreds,
	getAll as getAllCreds,
	login,
	logout,
	removeUser,
	setActive,
} from '@/helpers/mr_auth.ts'
import { mergeUrlQuery, parseModrinthLink } from '@/helpers/project-links.ts'
import {
	appSettingsKeys,
	appSettingsQueryOptions,
	get as getSettings,
	set as setSettings,
} from '@/helpers/settings.ts'
import { debugStartup, traceStartupStep } from '@/helpers/startup-debug'
import { get_opening_command, initialize_state } from '@/helpers/state'
import {
	gameOptionsSyncSourcesQueryOptions,
	globalSyncedOptionsQueryOptions,
	initializedSyncedOptionsQueryOptions,
	syncedOptionsKeys,
	syncedServersQueryOptions,
} from '@/helpers/synced-options'
import { syncedPackQueryOptions } from '@/helpers/synced-packs'
import { hasActivePride26Midas, hasMidasBadge } from '@/helpers/user-campaigns.ts'
import { get_user_preferences } from '@/helpers/user-preferences.ts'
import { parse_modrinth_user_link } from '@/helpers/users'
import {
	areUpdatesEnabled,
	enqueueUpdateForInstallation,
	getOS,
	getUpdateSize,
	isDev,
	isNetworkMetered,
	setRestartAfterPendingUpdate,
} from '@/helpers/utils.js'
import { start_join_server, start_join_singleplayer_world } from '@/helpers/worlds.ts'
import i18n, { setLocale } from '@/i18n.config'
import {
	instanceKeys,
	instanceListQueryOptions,
	screenshotKeys,
} from '@/pages/instance/query-options'
import { createAppClient } from '@/platform/modrinth-client'
import {
	appUpdateState,
	downloadAvailableAppUpdate,
	getNextAppUpdatePopupTime,
	installAvailableAppUpdate,
	markAppUpdateActionable,
	markAppUpdatePopupShown,
	openAppUpdateChangelog,
	setAppUpdateActions,
} from '@/providers/app-update.ts'
import { createContentInstall, provideContentInstall } from '@/providers/content-install'
import {
	provideAppUpdateDownloadProgress,
	subscribeToDownloadProgress,
} from '@/providers/download-progress.ts'
import { createServerInstall, provideServerInstall } from '@/providers/server-install'
import { setupProviders } from '@/providers/setup'
import { setupAppEventsProvider } from '@/providers/setup/app-events'
import { setupAuthProvider } from '@/providers/setup/auth'
import { setupAppUserPreferencesProvider } from '@/providers/setup/user-preferences.ts'
import { createBreadcrumbManager, provideBreadcrumbManager } from '@/shared/breadcrumbs'
import { appMessages } from '@/utils/app-messages'

import { AppNotificationManager } from './providers/app-notifications'
import { AppPopupNotificationManager } from './providers/app-popup-notifications'

debugStartup('App setup entered')
const appSettings = useAppSettings()
const appTheme = useTheme()
const quickInstances = useQuickInstanceLimit()
const router = useRouter()
const route = useRoute()
const { channel: appEventChannel, events: appEvents } = setupAppEventsProvider()
useInstanceMetadataRefresh(appEvents)
const breadcrumbManager = createBreadcrumbManager()
provideBreadcrumbManager(breadcrumbManager)
let fullscreenAdsWindowHold = false

async function handleFullscreenChange() {
	const fullscreen = document.fullscreenElement !== null
	if (fullscreen === fullscreenAdsWindowHold) return

	fullscreenAdsWindowHold = fullscreen
	try {
		if (fullscreen) {
			await take_ads_window_hold()
		} else {
			await release_ads_window_hold()
		}
	} catch (error) {
		fullscreenAdsWindowHold = !fullscreen
		handleError(error)
	}
}

const APP_LEFT_NAV_WIDTH = '4rem'
const APP_SIDEBAR_WIDTH = 300
const INTERCOM_BUBBLE_DEFAULT_PADDING = 20
const credentials = ref()
const storedModrinthAccounts = ref([])
let credentialsRefreshId = 0
const sidebarToggled = ref(true)
watch(
	() => appSettings.toggleSidebar,
	(toggleSidebar) => {
		sidebarToggled.value = !toggleSidebar
	},
)
const forceSidebar = computed(
	() =>
		route.path.startsWith('/browse') ||
		route.path.startsWith('/project') ||
		route.path.startsWith('/user'),
)
const sidebarVisible = computed(() => sidebarToggled.value || forceSidebar.value)

const viewportEl = ref()
const sidebarAnimation = ref()
const viewportAnimation = ref()

const SIDEBAR_ANIM_MS = 220
const SIDEBAR_EASE = 'cubic-bezier(0.32, 0.72, 0, 1)'

/** 不带动画地直接应用状态（启动时用，避免初始错位） */
function applySidebarState(open) {
	sidebarAnimation.value?.cancel?.()
	viewportAnimation.value?.cancel?.()
	const sidebarEl = sidebar.value?.$el
	if (sidebarEl) sidebarEl.style.transform = open ? 'translateX(0)' : 'translateX(100%)'
}

/** 带动画的展开/收起，支持中途反转 */
function animateSidebarWithWaapi(open) {
	const sidebarEl = sidebar.value?.$el
	if (sidebarEl) {
		const currentTransform = getComputedStyle(sidebarEl).transform
		sidebarAnimation.value?.cancel?.()
		sidebarAnimation.value = sidebarEl.animate(
			[
				{ transform: currentTransform && currentTransform !== 'none' ? currentTransform : 'translateX(0)' },
				{ transform: open ? 'translateX(0)' : 'translateX(100%)' },
			],
			{ duration: SIDEBAR_ANIM_MS, easing: SIDEBAR_EASE, fill: 'forwards' },
		)
	}


}

watch(sidebarVisible, (visible) => {
	// 切换瞬间暂停内容区的 CSS 过渡，避免卡片随宽度变化被“滑动缩放”
	document.body.classList.add('sidebar-resizing')
	animateSidebarWithWaapi(visible)
	setTimeout(() => {
		document.body.classList.remove('sidebar-resizing')
	}, 120)
})

onMounted(() => {
	nextTick(() => applySidebarState(sidebarVisible.value))

	// 临时调试：把所有可滚动元素标色，用于定位重叠的滚动条
	// 仅开发者模式 + 开发构建下才显示滚动区域标色
	if (import.meta.env.DEV && appSettings.devMode) {
		setTimeout(() => {
			const colors = ['red', 'lime', 'cyan', 'magenta', 'orange', 'yellow']
			let index = 0
			document.querySelectorAll('*').forEach((element) => {
				if (!(element instanceof HTMLElement)) return
				const style = getComputedStyle(element)
				const scrollable =
					(style.overflowY === 'auto' || style.overflowY === 'scroll' || style.overflowY === 'overlay') &&
					element.scrollHeight > element.clientHeight + 4
				if (!scrollable) return
				element.style.outline = `2px solid ${colors[index % colors.length]}`
				element.style.outlineOffset = '-2px'
				console.log(
					'[scroll-debug]',
					colors[index % colors.length],
					element.className,
					`scrollHeight=${element.scrollHeight} clientHeight=${element.clientHeight}`,
				)
				index++
			})
		}, 2500)
	}
})

// 由路由（如进入发现页）强制展开侧栏时，直接显示，不播动画


// 侧栏展开/收起期间，内容区轻微淡化，掩盒卡片重排的视觉跳变
let sidebarShiftTimer = undefined
// 去商业化：托管功能已移除，这两个状态恒为 false
const hostingRouteActive = computed(() => false)
const hostingUpdateRequired = computed(() => false)
const hostingIntercomIdentityKey = computed(() => {
	const rawServerId = route.params.id
	const serverId = Array.isArray(rawServerId) ? rawServerId[0] : rawServerId
	const userId = credentials.value?.user_id ?? credentials.value?.user?.id ?? 'anonymous'
	return `${userId}:${serverId ?? 'hosting'}`
})
const hostingIntercom = useHostingIntercom({
	enabled: computed(
		() => hostingRouteActive.value && !hostingUpdateRequired.value && !!credentials.value?.session,
	),
	appId: 'ykeritl9',
	fetchToken: fetchIntercomToken,
	identityKey: hostingIntercomIdentityKey,
	horizontalPadding: computed(() =>
		sidebarVisible.value
			? APP_SIDEBAR_WIDTH + INTERCOM_BUBBLE_DEFAULT_PADDING
			: INTERCOM_BUBBLE_DEFAULT_PADDING,
	),
})

const notificationManager = new AppNotificationManager()
provideNotificationManager(notificationManager)
const { handleError, addNotification } = notificationManager

useAppEvent(
	'warning',
	(event) =>
		addNotification({
			title: formatMessage(messages.warning),
			text: event.message,
			type: 'warning',
		}),
	appEvents,
)

const popupNotificationManager = new AppPopupNotificationManager()
providePopupNotificationManager(popupNotificationManager)
const { addPopupNotification } = popupNotificationManager
let adsConsentPopupId = null
useAppEvent('ads_consent_required', handleAdsConsentRequired, appEvents)

const tauriApiClient = createAppClient()
provideModrinthClient(tauriApiClient)
const { data: authenticatedModrinthUser } = useQuery({
	queryKey: computed(() => ['authenticated-user', 'campaigns', credentials.value?.user?.id]),
	queryFn: () => tauriApiClient.labrinth.users_v3.getAuthenticated(),
	enabled: () => !!credentials.value?.session,
	retry: false,
})
useQuery({
	queryKey: computed(() => instanceKeys.sharedEligibility(credentials.value?.user?.id)),
	queryFn: can_current_user_use_shared_instances,
	enabled: () => !!credentials.value?.session && !!credentials.value?.user?.id,
	retry: false,
	staleTime: Infinity,
	refetchOnMount: false,
	refetchOnWindowFocus: false,
	refetchOnReconnect: false,
})
const hasPlus = computed(
	() =>
		!!credentials.value?.user &&
		(hasMidasBadge(credentials.value.user) ||
			hasActivePride26Midas(authenticatedModrinthUser.value?.campaigns?.pride_26)),
)
const showAd = computed(
	() => sidebarVisible.value && !hasPlus.value && credentials.value !== undefined,
)
const adConsentAvailable = computed(() => credentials.value !== undefined && !hasPlus.value)
providePageContext({
	hierarchicalSidebarAvailable: ref(true),
	showAds: showAd,
	adConsentAvailable,
	floatingActionBarOffsets: {
		left: ref(APP_LEFT_NAV_WIDTH),
		right: computed(() => (sidebarVisible.value ? `${APP_SIDEBAR_WIDTH}px` : '0px')),
	},
	intercomBubble: hostingIntercom.intercomBubble,
	featureFlags: {
		serverRamAsBytesAlwaysOn: computed(() =>
			appSettings.getFeatureFlag('server_ram_as_bytes_always_on'),
		),
	},
	openExternalUrl: (url) => void openUrl(url),
})
provideModalBehavior({
	noblur: computed(() => !appTheme.advancedRendering),
	onShow: () => take_ads_window_hold(),
	onHide: () => release_ads_window_hold(),
})

const creationIconEditorModal = ref(null)
const creationGeneratedIcon = ref(null)
const creationIconTarget = ref('creation-flow')

const {
	installationModal,
	unknownPackWarningModal,
	fetchExistingInstanceNames,
	handleCreate,
	handleBrowseModpacks,
	searchProjects,
	getLoaderManifest,
	setModpackAlreadyInstalledModal,
	handleModpackDuplicateCreateAnyway,
	handleModpackDuplicateGoToInstance,
	onboardingChecklist,
	tags,
} = setupProviders(
	tauriApiClient,
	notificationManager,
	popupNotificationManager,
	appEvents,
	(iconPath) =>
		creationGeneratedIcon.value?.path === iconPath ? creationGeneratedIcon.value.config : null,
)

async function randomizeCreationIcon() {
	const generated = await creationIconEditorModal.value?.randomizeAndSave()
	if (!generated) return null

	creationGeneratedIcon.value = { path: generated.iconPath, config: generated.config }
	return {
		path: generated.iconPath,
		previewUrl: convertFileSrc(generated.iconPath),
	}
}

function customizeCreationIcon() {
	creationIconTarget.value = 'creation-flow'
	creationIconEditorModal.value?.show()
}

function customizeContentInstallIcon() {
	creationIconTarget.value = 'content-install'
	creationIconEditorModal.value?.show()
}

function onCreationIconSaved(iconPath, config) {
	creationGeneratedIcon.value = { path: iconPath, config }
	if (creationIconTarget.value === 'content-install') {
		modInstallModal.value?.setIcon(iconPath, convertFileSrc(iconPath))
		return
	}

	const context = installationModal.value?.ctx
	if (!context) return

	context.instanceIcon.value = null
	context.instanceIconUrl.value = convertFileSrc(iconPath)
	context.instanceIconPath.value = iconPath
}

const displayedServerInviteNotifications = new Set()
const serverInvitePopupNotificationIds = new Set()
let liveNotificationGeneration = 0
let liveNotificationsEnabled = true

const offline = ref(!navigator.onLine)
window.addEventListener('offline', () => {
	offline.value = true
})
window.addEventListener('online', () => {
	offline.value = false
})

const os = ref('')
const isDevEnvironment = ref(false)

const stateInitialized = ref(false)
const globalSyncedOptionsQuery = useQuery({
	...globalSyncedOptionsQueryOptions(),
	enabled: computed(() => stateInitialized.value),
})
useQueries({
	queries: computed(() =>
		[
			appSettingsQueryOptions(),
			instanceListQueryOptions(),
			maxMemoryQueryOptions(),
			gameSettingsQueryOptions(),
			initializedSyncedOptionsQueryOptions(),
			gameOptionsSyncSourcesQueryOptions(),
			syncedServersQueryOptions(),
			syncedPackQueryOptions('resourcepack'),
			syncedPackQueryOptions('datapack'),
		].map((options) => ({
			...options,
			enabled: stateInitialized.value && options.enabled !== false,
		})),
	),
})

const criticalErrorMessage = ref()

const isMaximized = ref(false)
const isFullscreen = ref(false)

watch([os, isFullscreen], ([osName, fullscreen]) => {
	document.documentElement.classList.toggle('mac-traffic-lights', osName === 'MacOS' && !fullscreen)
})

const authUnreachableDebug = useDebugLogger('AuthReachableChecker')
const authServerQuery = useQuery({
	queryKey: ['authServerReachability'],
	queryFn: async () => {
		await check_reachable()
		authUnreachableDebug('Auth servers are reachable')
		return true
	},
	enabled: false, // 去商业化：不再检查 Modrinth 认证服务器
	refetchInterval: 5 * 60 * 1000, // 5 minutes
	retry: false,
	refetchOnWindowFocus: false,
})

// 去商业化：Modrinth 登录已移除，这个"身份验证服务器连不上"的检查会把整个界面
// 替换成一条警告（因为它在模板里用的是 v-else 链），国内网络不稳时就会白屏。
// 现在恒为 false，检查逻辑本身保留但不再影响界面。
const authUnreachable = computed(() => {
	return false
})

let unlistenEditMenu

function handleEditMenuAction(action) {
	const event = new CustomEvent(`edit-menu:${action}`, { cancelable: true })
	if (document.dispatchEvent(event)) document.execCommand(action)
}

onMounted(() => {
})

// ResizeObserver 的循环告警属于噪音，不上报
function isResizeObserverNoise(value) {
	const text = value instanceof Error ? value.message : String(value ?? '')
	return text.includes('ResizeObserver loop')
}

window.addEventListener('unhandledrejection', (event) => {
	if (isResizeObserverNoise(event.reason)) return
	console.error('[UNHANDLED-REJECTION]', event.reason?.stack || event.reason)
})

window.addEventListener('error', (event) => {
	if (isResizeObserverNoise(event.error ?? event.message)) return
	console.error('[UNCAUGHT-ERROR]', event.error?.stack || event.message)
})

// 本地开发时的前端日志采集：把 console.warn/error 和未捕获异常写到
// %APPDATA%/com.aircore.launcher/logs/webview.log，方便排查问题
function installConsoleLogCapture() {
	if (!import.meta.env.DEV) return
	const queue = []
	let flushTimer
	let dropped = 0

	const flush = () => {
		flushTimer = undefined
		if (queue.length === 0) return
		const text = queue.splice(0, queue.length).join('\n') + '\n'
		void invoke('plugin:utils|debug_log_append', { lines: text }).catch(() => {})
		if (dropped > 0) {
			console.warn('[console-capture] dropped ' + dropped + ' duplicated lines')
			dropped = 0
		}
	}

	// 安装完立即写一行，用来确认钩子是否生效
	queue.push(new Date().toISOString() + ' [capture] installed')
	flushTimer = setTimeout(flush, 300)

	const push = (level, args) => {
		try {
			const text = args
				.map((arg) => {
					if (arg instanceof Error) return arg.stack ?? arg.message
					if (typeof arg === 'string') return arg
					try {
						return JSON.stringify(arg)
					} catch {
						return String(arg)
					}
				})
				.join(' ')
			if (queue.length > 500) {
				dropped += 1
				return
			}
			queue.push(new Date().toISOString() + ' [' + level + '] ' + text)
			if (!flushTimer) flushTimer = setTimeout(flush, 500)
		} catch {
			// ignore
		}
	}

	const originalWarn = console.warn.bind(console)
	const originalError = console.error.bind(console)
	console.warn = (...args) => {
		push('warn', args)
		originalWarn(...args)
	}
	console.error = (...args) => {
		push('error', args)
		originalError(...args)
	}
	window.addEventListener('error', (event) => {
		push('uncaught', [event.error ?? event.message])
	})
	window.addEventListener('unhandledrejection', (event) => {
		push('rejection', [event.reason])
	})
	window.addEventListener('beforeunload', () => flush())
}

// 立即安装：模块加载时就开始捕获，能收到首屏渲染期间的警告
installConsoleLogCapture()

onMounted(async () => {
	// 先把数据库里的 AirCore 偏好回填到本地缓存
	try {
		const { hydratePrefs } = await import('@/composables/use-prefs')
		await hydratePrefs()
	} catch {
		// 忽略
	}

	// 启动时恢复 Minecraft 下载源设置
	try {
		const source = localStorage.getItem('aircore-download-source') ?? 'official'
		void invoke('plugin:utils|set_download_source', { source })
	} catch {
		// 忽略
	}

	// 启动时恢复加载器下载源
	try {
		const loaderMode = localStorage.getItem('aircore-loader-mode') ?? 'cn'
		const loaderPrefix = localStorage.getItem('aircore-loader-prefix') ?? ''
		void invoke('plugin:utils|set_loader_source', {
			mode: loaderMode,
			prefix: loaderPrefix || null,
		})
	} catch {
		// 忽略
	}

	// 恢复强调色
	try {
		const { applyAccentColor } = await import('@/composables/use-accent-color')
		const color = localStorage.getItem('aircore-accent-color')
		if (color) applyAccentColor(color)
	} catch {
		// 忽略
	}

	// 插件宿主：暴露 Vue / UI 组件库，并加载已安装的插件
	// 把真正的错误堆栈打印出来，便于定位白屏原因
	installPluginHost()
	// 插件被卸下时，如果正停在它的页面上，自动回首页
	window.addEventListener('aircore:plugin-unloaded', (event) => {
		const pluginId = event.detail
		if (route.path === `/ext/${pluginId}`) void router.replace('/')
	})
	// 把启动器自己的通知系统交给插件，避免两套通知重叠
	setHostNotifier((message, type) => {
		const manager = notificationManager
		if (!manager?.addNotification) return
		manager.addNotification({
			type: type === 'error' ? 'error' : type === 'warning' ? 'warning' : 'info',
			title: formatMessage(messages.pluginsNotification),
			text: message,
		})
	})
	try {
		const extensions = await invoke('plugin:extensions|extensions_list')
		pluginState.extensions = extensions ?? []
		for (const extension of extensions ?? []) {
			if (extension.enabled === false) {
				continue
			}
			// 完全沙箱：插件必须是 index.html，不再将 JS 直接注入主页面
			if (!(extension.entry ?? '').endsWith('.html')) {
				pluginState.error = `插件 ${extension.id} 的入口不是 index.html，沙箱模式下不会加载`
				logPlugin('非沙箱插件已跳过', 'warn', extension.id)
				continue
			}
			registerSandboxNav(extension)
		}
		void startPluginHotReload(() =>
			invoke('plugin:extensions|extensions_versions'),
		)
	} catch (error) {
		pluginState.error = `读取插件列表失败: ${String(error)}`
		console.warn('[plugins] 列凐扩展失败', error)
	}
	if (!pluginRegistry.panels.some((panel) => panel.id === '__plugins')) {
		pluginRegistry.panels.push({
			id: '__plugins',
			title: '插件诊断',
			component: PluginDiagnostics,
		})
	}

	try {
		const listeners = await Promise.all([
			listen('edit-menu://undo', () => handleEditMenuAction('undo')),
			listen('edit-menu://redo', () => handleEditMenuAction('redo')),
		])
		unlistenEditMenu = () => listeners.forEach((unlisten) => unlisten())
	} catch (error) {
		handleError(error)
	}

	await useCheckDisableMouseover()
	try {
		handleAdsConsentRequired(await should_show_ads_consent_popup())
	} catch (error) {
		handleError(error)
	}

	document.querySelector('body').addEventListener('click', handleClick)
	document.querySelector('body').addEventListener('auxclick', handleAuxClick)
	document.querySelector('body').addEventListener('contextmenu', handleContextMenu)
	document.addEventListener('fullscreenchange', handleFullscreenChange)

	checkUpdates()
})

onUnmounted(async () => {
	document.querySelector('body').removeEventListener('click', handleClick)
	document.querySelector('body').removeEventListener('auxclick', handleAuxClick)
	document.querySelector('body').removeEventListener('contextmenu', handleContextMenu)
	document.removeEventListener('fullscreenchange', handleFullscreenChange)
	unlistenEditMenu?.()
	clearDelayedUpdatePopup()

	if (fullscreenAdsWindowHold) {
		fullscreenAdsWindowHold = false
		await release_ads_window_hold().catch(handleError)
	}
	await unlistenUpdateDownload?.()
})

const { formatMessage } = useVIntl()

// 自定义强调色：启动时应用，主题切换时重新应用
const accentColorState = useAccentColor()
applyAccentColor(accentColorState.accentColor.value)
watch(
	() => appTheme.active,
	() => applyAccentColor(accentColorState.accentColor.value),
)
const formatBytes = useFormatBytes()

const messages = defineMessages({
	pluginLogs: { id: 'app.navigation.pluginLogs', defaultMessage: 'Plugin logs (developer)' },
	libraryLabel: { id: 'app.navigation.library', defaultMessage: 'Instances' },
	pluginsNotification: { id: 'app.plugins.notificationTitle', defaultMessage: 'Plugins' },
	warning: { id: 'app.notification.warning', defaultMessage: 'Warning' },
	updateDownloadMissingVersion: {
		id: 'app.update.download-error.missing-version',
		defaultMessage: 'Failed to download update: no version available',
	},
	updateInstalledToastTitle: {
		id: 'app.update.complete-toast.title',
		defaultMessage: 'Version {version} was successfully installed!',
	},
	updateInstalledToastText: {
		id: 'app.update.complete-toast.text',
		defaultMessage: 'Click here to view the changelog.',
	},
	authUnreachableHeader: {
		id: 'app.auth-servers.unreachable.header',
		defaultMessage: 'Cannot reach authentication servers',
	},
	authUnreachableBody: {
		id: 'app.auth-servers.unreachable.body',
		defaultMessage:
			'Minecraft authentication servers may be down right now. Check your internet connection and try again later.',
	},
	adsConsentTitle: {
		id: 'app.ads-consent.title',
		defaultMessage: 'Your privacy and how ads support AirCore',
	},
	adsConsentBody: {
		id: 'app.ads-consent.body',
		defaultMessage:
			'Ads make AirCore possible and fund creator payouts. Our partners may store or access cookies in the app to personalize ads and measure performance.',
	},
	adsConsentManage: {
		id: 'app.ads-consent.manage',
		defaultMessage: 'Manage preferences',
	},
	adsConsentReject: {
		id: 'app.ads-consent.reject',
		defaultMessage: 'Reject all',
	},
	adsConsentAccept: {
		id: 'app.ads-consent.accept',
		defaultMessage: 'Accept all',
	},
	home: {
		id: 'app.nav.home',
		defaultMessage: 'Home',
	},
	modrinthHosting: {
		id: 'app.nav.modrinth-hosting',
		defaultMessage: 'AirCore Hosting',
	},
	screenshots: {
		id: 'app.nav.screenshots',
		defaultMessage: 'Screenshots',
	},
	createNewInstance: {
		id: 'app.nav.create-new-instance',
		defaultMessage: 'Create new instance',
	},
	modrinthAccount: {
		id: 'app.nav.modrinth-account',
		defaultMessage: 'AirCore account',
	},
	viewProfile: {
		id: 'app.nav.view-profile',
		defaultMessage: 'View profile',
	},
	addFriend: {
		id: 'friends.action.add-friend',
		defaultMessage: 'Add a friend',
	},
	signInToModrinthAccount: {
		id: 'app.nav.sign-in-to-modrinth-account',
		defaultMessage: 'Sign into AirCore',
	},
	loadingProfile: {
		id: 'app.nav.loading-profile',
		defaultMessage: 'Loading profile...',
	},
	switchAccount: {
		id: 'app.nav.switch-account',
		defaultMessage: 'Switch account',
	},
	addAccount: {
		id: 'app.nav.add-account',
		defaultMessage: 'Add account',
	},
	removeAccount: {
		id: 'app.nav.remove-account',
		defaultMessage: 'Remove account',
	},
	restarting: {
		id: 'app.restarting',
		defaultMessage: 'Restarting...',
	},
	upgradeToModrinthPlus: {
		id: 'app.nav.upgrade-to-modrinth-plus',
		defaultMessage: 'Upgrade to AirCore+',
	},
})

function handleAdsConsentRequired(required) {
	if (!required) {
		if (adsConsentPopupId !== null) {
			popupNotificationManager.removeNotification(adsConsentPopupId)
			adsConsentPopupId = null
		}
		return
	}

	if (
		adsConsentPopupId !== null &&
		popupNotificationManager.getNotifications().some((item) => item.id === adsConsentPopupId)
	) {
		return
	}

	const notification = addPopupNotification({
		contentType: 'standard',
		title: formatMessage(messages.adsConsentTitle),
		text: formatMessage(messages.adsConsentBody),
		type: 'info',
		hideIcon: true,
		autoCloseMs: null,
		dismissible: false,
		buttons: [
			{
				label: formatMessage(messages.adsConsentManage),
				action: () => perform_ads_consent_action('manage').catch(handleError),
				color: 'standard',
				keepOpen: true,
			},
			{
				label: formatMessage(messages.adsConsentReject),
				action: () => perform_ads_consent_action('reject').catch(handleError),
				color: 'brand',
				keepOpen: true,
			},
			{
				label: formatMessage(messages.adsConsentAccept),
				action: () => perform_ads_consent_action('accept').catch(handleError),
				color: 'brand',
				keepOpen: true,
			},
		],
	})

	adsConsentPopupId = notification.id
}

async function setupApp() {
	tags.initialize()
	await traceStartupStep('Initialize onboarding checklist', () => onboardingChecklist.initialize())

	const {
		native_decorations,
		theme,
		locale,
		telemetry,
		hide_nametag_skins_page,
		advanced_rendering,
		toggle_sidebar,
		show_files_tab_in_instances,
		show_worlds_tab_in_instances,
		show_screenshots_tab_in_instances,
		show_skin_selector_in_sidebar,
		refocus_on_game_close,
		compact_instance_cards,
		show_play_time,
		warn_on_unknown_modpacks,
		skip_non_essential_warnings,
		show_jump_in,
		always_show_copy_details,
		hide_installed_modpacks,
		advanced_filters_collapsed,
		dismissed_photosensitivity_filter_warning,
		friends_active_collapsed,
		friends_online_collapsed,
		friends_offline_collapsed,
		friends_pending_collapsed,
		developer_mode,
		feature_flags,
		pending_update_toast_for_version,
	} = await traceStartupStep('Read startup settings', getSettings)

	// Initialize locale from saved settings
	if (locale) {
		await traceStartupStep('Apply startup locale', () => setLocale(locale))
	}

	Object.assign(appSettings.featureFlags, feature_flags)
	isMaximized.value = await traceStartupStep('Read window maximized state', () =>
		getCurrentWindow().isMaximized(),
	)
	isFullscreen.value = await traceStartupStep('Read window fullscreen state', () =>
		getCurrentWindow().isFullscreen(),
	)
	os.value = await traceStartupStep('Read operating system', getOS)
	const dev = await traceStartupStep('Read development mode', isDev)
	isDevEnvironment.value = dev
	const version = await traceStartupStep('Read app version', getVersion)
	appSettings.nativeDecorations = native_decorations
	if (os.value !== 'MacOS') {
		await traceStartupStep('Apply window decorations', () =>
			getCurrentWindow().setDecorations(native_decorations),
		)
	}

	appTheme.preferred = theme
	appTheme.advancedRendering = advanced_rendering
	appSettings.hideNametagSkinsPage = hide_nametag_skins_page
	appSettings.toggleSidebar = toggle_sidebar
	appSettings.showFilesTabInInstances = show_files_tab_in_instances
	appSettings.showWorldsTabInInstances = show_worlds_tab_in_instances
	appSettings.showScreenshotsTabInInstances = show_screenshots_tab_in_instances
	appSettings.showSkinSelectorInSidebar = show_skin_selector_in_sidebar
	appSettings.refocusOnGameClose = refocus_on_game_close
	appSettings.compactInstanceCards = compact_instance_cards
	appSettings.showPlayTime = show_play_time
	appSettings.warnOnUnknownModpacks = warn_on_unknown_modpacks
	appSettings.skipNonEssentialWarnings = skip_non_essential_warnings
	appSettings.showJumpIn = show_jump_in
	appSettings.alwaysShowCopyDetails = always_show_copy_details
	appSettings.hideInstalledModpacks = hide_installed_modpacks
	appSettings.advancedFiltersCollapsed = advanced_filters_collapsed
	appSettings.dismissedPhotosensitivityFilterWarning = dismissed_photosensitivity_filter_warning
	appSettings.friendsActiveCollapsed = friends_active_collapsed
	appSettings.friendsOnlineCollapsed = friends_online_collapsed
	appSettings.friendsOfflineCollapsed = friends_offline_collapsed
	appSettings.friendsPendingCollapsed = friends_pending_collapsed
	appSettings.devMode = developer_mode
	stateInitialized.value = true
	debugStartup('App state initialized')
	await traceStartupStep('Render initialized app', nextTick)
	await newUpdateModal.value?.notifyForVersion(version, pending_update_toast_for_version)

	await traceStartupStep('Register window resize listener', () =>
		getCurrentWindow().onResized(async () => {
			isMaximized.value = await getCurrentWindow().isMaximized()
			isFullscreen.value = await getCurrentWindow().isFullscreen()
		}),
	)

	if (telemetry) {
		initAnalytics()
		if (dev) debugAnalytics()
		trackEvent('Launched', { version, dev })
	}

	const osType = await traceStartupStep('Read operating system type', async () => type())
	if (osType === 'macos') {
		document.getElementsByTagName('html')[0].classList.add('mac')
	} else {
		document.getElementsByTagName('html')[0].classList.add('windows')
	}

	fetch(`https://api.modrinth.com/appCriticalAnnouncement.json?version=${version}`)
		.then((response) => response.json())
		.then((res) => {
			if (res && res.header && res.body) {
				criticalErrorMessage.value = res
			}
		})
		.catch(() => {
			console.log(
				`No critical announcement found at https://api.modrinth.com/appCriticalAnnouncement.json?version=${version}`,
			)
		})

	traceStartupStep('Read opening command', get_opening_command).then(handleCommand)
	traceStartupStep('Refresh startup credentials', fetchCredentials)

	if (pending_update_toast_for_version !== null) {
		const settings = await traceStartupStep(
			'Read settings to clear update notification',
			getSettings,
		)
		settings.pending_update_toast_for_version = null
		await traceStartupStep('Clear update notification', () => setSettings(settings))
	}
}

const stateFailed = ref(false)
const { loading, canNavigateBack, canNavigateForward, onSuspensePending, onSuspenseResolve } =
	useRouteLoading({ stateInitialized, stateFailed })
provideLoadingState(loading)

traceStartupStep('Initialize backend state', () => initialize_state(appEventChannel))
	.then(() => {
		traceStartupStep('Initialize frontend state', setupApp).catch((err) => {
			stateFailed.value = true
			console.error(err)
			error.showError(err, null, false, 'state_init')
		})
	})
	.catch((err) => {
		stateFailed.value = true
		console.error('Failed to initialize app', err)
		error.showError(err, null, false, 'state_init')
	})

const handleClose = async () => {
	await saveWindowState(StateFlags.ALL)
	await getCurrentWindow().close()
}

const queryClient = useQueryClient()

watch(stateInitialized, (ready) => {
	if (ready) {
		queryClient.prefetchQuery({
			queryKey: ['servers'],
			queryFn: async () => {
				const response = await tauriApiClient.archon.servers_v0.list({ limit: 100 })
				const hasMedalServers = response.servers.some((s) => s.is_medal)
				if (hasMedalServers) {
					const subscriptions = await tauriApiClient.labrinth.billing_internal.getSubscriptions()
					for (const server of response.servers) {
						if (server.is_medal) {
							const sub = subscriptions.find((s) => s.metadata?.id === server.server_id)
							if (sub) {
								server.medal_expires = new Date(
									new Date(sub.created).getTime() + 5 * 86400000,
								).toISOString()
							}
						}
					}
				}
				return response
			},
			staleTime: 30_000,
		})
		queryClient.prefetchQuery({
			queryKey: ['billing', 'subscriptions'],
			queryFn: () => tauriApiClient.labrinth.billing_internal.getSubscriptions(),
			staleTime: 30_000,
		})
		queryClient.prefetchQuery({
			queryKey: ['billing', 'payments'],
			queryFn: () => tauriApiClient.labrinth.billing_internal.getPayments(),
			staleTime: 30_000,
		})
	}
})

const error = useError()
const errorModal = ref()
const minecraftAuthErrorModal = ref()
const minecraftRequiredModal = ref()

const contentInstall = createContentInstall({ router, handleError, appEvents })
provideContentInstall(contentInstall)
const {
	instances: contentInstallInstances,
	compatibleLoaders: contentInstallLoaders,
	gameVersions: contentInstallGameVersions,
	loading: contentInstallLoading,
	defaultTab: contentInstallDefaultTab,
	preferredLoader: contentInstallPreferredLoader,
	preferredGameVersion: contentInstallPreferredGameVersion,
	releaseGameVersions: contentInstallReleaseGameVersions,
	projectInfo: contentInstallProjectInfo,
	handleInstallToInstance,
	handleCreateAndInstall,
	prepareNewInstance,
	handleNavigate: handleContentInstallNavigate,
	handleCancel: handleContentInstallCancel,
	setContentInstallModal,
	setModpackAlreadyInstalledModal: setContentInstallModpackAlreadyInstalledModal,
	handleModpackDuplicateCreateAnyway: handleContentInstallModpackDuplicateCreateAnyway,
	handleModpackDuplicateGoToInstance: handleContentInstallModpackDuplicateGoToInstance,
	setIncompatibilityWarningModal: setContentIncompatibilityWarningModal,
	incompatibilityWarningVersions: contentInstallIncompatibilityWarningVersions,
	incompatibilityWarningCurrentGameVersion: contentInstallIncompatibilityWarningCurrentGameVersion,
	incompatibilityWarningCurrentLoader: contentInstallIncompatibilityWarningCurrentLoader,
	incompatibilityWarningProjectType: contentInstallIncompatibilityWarningProjectType,
	incompatibilityWarningProjectIconUrl: contentInstallIncompatibilityWarningProjectIconUrl,
	incompatibilityWarningProjectName: contentInstallIncompatibilityWarningProjectName,
	incompatibilityWarningMessage: contentInstallIncompatibilityWarningMessage,
	incompatibilityWarningInstalling: contentInstallIncompatibilityWarningInstalling,
	handleIncompatibilityWarningInstall: handleContentInstallIncompatibilityWarningInstall,
	handleIncompatibilityWarningCancel: handleContentInstallIncompatibilityWarningCancel,
} = contentInstall

async function prepareCreationProjectInstall(projectId, projectType) {
	if (projectType === 'modpack') {
		await contentInstall.install(
			projectId,
			null,
			null,
			'CreationModalProject',
			undefined,
			(instanceId) => void router.push(`/instance/${encodeURIComponent(instanceId)}`),
		)
		return null
	}

	await prepareNewInstance(projectId)
	const info = contentInstallProjectInfo.value
	if (!info) throw new Error(`Project information is unavailable: '${projectId}'`)

	return {
		projectId,
		title: info.title,
		iconUrl: info.iconUrl,
		link: info.link,
		owner: info.owner,
		compatibleLoaders: [...contentInstallLoaders.value],
		gameVersions: [...contentInstallGameVersions.value],
		releaseGameVersions: new Set(contentInstallReleaseGameVersions.value),
	}
}

const serverInstall = createServerInstall({
	router,
	handleError,
	popupNotificationManager,
	appEvents,
})
provideServerInstall(serverInstall)
const {
	setInstallToPlayModal: setServerInstallToPlayModal,
	setUpdateToPlayModal: setServerUpdateToPlayModal,
	setAddServerToInstanceModal: setServerAddServerToInstanceModal,
	playServerProject,
} = serverInstall

const modInstallModal = ref()
const modpackAlreadyInstalledModal = ref()
const contentInstallModpackAlreadyInstalledModal = ref()
const addServerToInstanceModal = ref()
const incompatibilityWarningModal = ref()
const installToPlayModal = ref()
const sharedInstanceInviteHandler = ref()
const updateToPlayModal = ref()

const modrinthLoginModal = ref()
const appSettingsModal = ref()
const newUpdateModal = ref()

const settingsActions = provideSettingsActions({
	openSettings: () => appSettingsModal.value?.show(),
	openProfile: () => appSettingsModal.value?.showProfile(),
	openFeatureFlags: () => appSettingsModal.value?.showFeatureFlags(),
	openSyncedOptions: () => appSettingsModal.value?.showSyncedOptions(),
	openInstanceDefaults: (tab) => appSettingsModal.value?.showInstanceDefaults(tab),
})

watch(incompatibilityWarningModal, (modal) => {
	if (modal) {
		setContentIncompatibilityWarningModal(modal)
	}
})

const authProvider = setupAuthProvider(credentials, async (_redirectPath, flow, options) => {
	if (options?.showModal === false) {
		await signIn(flow)
	} else {
		await requestSignIn(flow)
	}
})

const userPreferences = setupAppUserPreferencesProvider(authProvider, notificationManager)
let userPreferencesSync = Promise.resolve()

watch(
	[userPreferences.preferences, stateInitialized],
	([preferences, initialized]) => {
		if (!preferences || !initialized) return

		userPreferencesSync = userPreferencesSync
			.then(async () => {
				const settings = await getSettings()
				const selectedTheme = preferences.appearance.auto ? 'system' : preferences.appearance.theme
				const userId = credentials.value?.user_id ?? credentials.value?.user?.id
				if (userId) {
					rememberAccountAppearance(userId, preferences.appearance)
				}
				const locale = preferences.localization.locale
				const behavior = preferences.behavior
				let settingsChanged = false

				if (i18n.global.locale.value !== locale) {
					await setLocale(locale)
				}

				if (settings.locale !== locale) {
					settings.locale = locale
					settingsChanged = true
				}




				if (settingsChanged) {
					await setSettings(settings)
					queryClient.setQueryData(appSettingsKeys.all, settings)
				}
			})
			.catch(handleError)
	},
	{ immediate: true },
)

async function validateSession(sessionToken) {
	try {
		const response = await tauriFetch(`${config.labrinthBaseUrl}/v2/user`, {
			method: 'GET',
			headers: { Authorization: sessionToken },
		})
		if (response.status === 401) return false
		return true
	} catch {
		return true
	}
}

async function fetchCredentials() {
	const hadSession = !!credentials.value?.session
	const refreshId = ++credentialsRefreshId
	credentials.value = undefined

	const creds = await traceStartupStep('Read stored credentials', getCreds).catch(handleError)
	if (refreshId !== credentialsRefreshId) return
	if (!creds && hadSession) clearLiveNotifications()

	if (creds && creds.user_id) {
		if (
			creds.session &&
			!(await traceStartupStep('Validate stored session', () => validateSession(creds.session)))
		) {
			if (refreshId !== credentialsRefreshId) return

			clearLiveNotifications()
			await removeUser(creds.user_id).catch(handleError)
			if (refreshId !== credentialsRefreshId) return

			credentials.value = null
			liveNotificationsEnabled = false
			await fetchStoredModrinthAccounts()
			return
		}
		creds.user = await traceStartupStep('Fetch signed-in user', () =>
			get_user(creds.user_id, 'bypass'),
		).catch(handleError)
		if (refreshId !== credentialsRefreshId) return
	}
	credentials.value = creds ?? null
	liveNotificationsEnabled = !!creds?.session
	await traceStartupStep('Load stored account profiles', fetchStoredModrinthAccounts)
}

async function signIn(flow = 'sign-in', addAccount = false) {
	try {
		await login(flow, addAccount)
		await fetchCredentials()
	} catch (error) {
		if (error?.code === 'login_canceled') {
			// user closed the login window
		} else {
			handleError(error)
		}
	}
}

async function requestSignIn(flow = 'sign-in', addAccount = false) {
	await modrinthLoginModal.value?.showSigningIn(flow, addAccount)
}

async function requestModrinthAuth(flow = 'sign-in', addAccount = false) {
	await signIn(flow, addAccount)
	return !!credentials.value?.session
}

async function logOut() {
	if (!credentials.value?.user) return
	await completeAccountSwitch(() => logout())
}

async function fetchStoredModrinthAccounts() {
	const all = (await getAllCreds().catch(handleError)) ?? []
	const ids = all.map((account) => account.user_id)
	const users = ids.length ? ((await get_user_many(ids).catch(handleError)) ?? []) : []
	const usersById = new Map(users.map((user) => [user.id, user]))

	storedModrinthAccounts.value = all.map((account) => ({
		...account,
		user: usersById.get(account.user_id) ?? {
			id: account.user_id,
			username: account.user_id,
			avatar_url: null,
			role: 'developer',
		},
	}))
}

const accountSwitcherAccounts = computed(() => {
	const currentId = credentials.value?.session ? credentials.value.user_id : null

	return storedModrinthAccounts.value.map((account) => ({
		...account,
		optionId: `account-${account.user_id}`,
		current: account.user_id === currentId,
	}))
})

const profileButtonTooltip = computed(() => {
	if (credentials.value === undefined) return formatMessage(messages.loadingProfile)
	if (credentials.value?.user) return formatMessage(messages.modrinthAccount)
	return formatMessage(messages.signInToModrinthAccount)
})

const accountSwitcherOptions = computed(() => [
	...accountSwitcherAccounts.value.map((account) => ({
		id: account.optionId,
		label: account.user.username,
		selected: account.current,
		action: () => switchModrinthAccount(account),
		trailingAction: {
			label: formatMessage(messages.removeAccount),
			icon: XIcon,
			color: 'red',
			action: () => forgetModrinthAccount(account.user_id),
		},
	})),
	{
		type: 'divider',
	},
	{
		id: 'add-account',
		label: formatMessage(messages.addAccount),
		icon: PlusIcon,
		action: () => requestSignIn('sign-in', true),
	},
])

const isSwitchingAccount = ref(false)

async function persistAppearanceTheme(appearance) {
	const selectedTheme = appearance.auto ? 'system' : appearance.theme
	appTheme.applyAccountAppearance(appearance)
	const settings = await getSettings()
	if (settings.theme !== selectedTheme) {
		settings.theme = selectedTheme
		await setSettings(settings)
	}
}

async function completeAccountSwitch(task) {
	isSwitchingAccount.value = true
	await nextTick()
	try {
		await task()
		window.location.reload()
	} catch (error) {
		isSwitchingAccount.value = false
		handleError(error)
	}
}

async function switchModrinthAccount(account) {
	if (account.current) return

	const cached = getAccountAppearance(account.user_id)
	if (cached) await persistAppearanceTheme(cached)
	await nextTick()

	await completeAccountSwitch(async () => {
		await setActive(account.user_id)
		if (cached) return

		try {
			const preferences = await get_user_preferences(account.user_id)
			rememberAccountAppearance(account.user_id, preferences.appearance)
			await persistAppearanceTheme(preferences.appearance)
			await nextTick()
		} catch {
			// no saved appearance for this account
		}
	})
}

async function forgetModrinthAccount(userId) {
	const isCurrent = credentials.value?.user_id === userId && !!credentials.value?.session
	if (isCurrent) {
		await completeAccountSwitch(() => removeUser(userId))
		return
	}

	await removeUser(userId).catch(handleError)
	await fetchStoredModrinthAccounts()
}

const modrinthAccountMenuOptions = computed(() => [
	{
		id: 'view-profile',
		label: formatMessage(messages.viewProfile),
		icon: UserIcon,
		action: () => router.push(`/user/${encodeURIComponent(credentials.value.user.username)}`),
	},
	{
		id: 'plus',
		label: formatMessage(messages.upgradeToModrinthPlus),
		icon: ArrowBigUpDashIcon,
		type: 'link',
		href: 'https://modrinth.plus?app',
		target: '_blank',
		tone: 'purple',
		shown: !hasPlus.value,
	},
	{
		id: 'add-friend',
		label: formatMessage(messages.addFriend),
		icon: UserPlusIcon,
		action: () => sidebar.value?.showAddFriendModal(),
	},
	{
		id: 'flags',
		label: formatMessage(commonSettingsMessages.featureFlags),
		icon: ToggleRightIcon,
		shown: appSettings.devMode,
		action: settingsActions.openFeatureFlags,
	},
	{
		type: 'divider',
	},
	{
		id: 'switch-account',
		label: formatMessage(messages.switchAccount),
		icon: ArrowLeftRightIcon,
		type: 'submenu',
		options: accountSwitcherOptions.value,
	},
	{
		id: 'sign-out',
		label: formatMessage(commonMessages.signOutButton),
		icon: LogOutIcon,
		tone: 'red',
		action: () => logOut(),
	},
])

async function fetchIntercomToken() {
	const creds = await getCreds()
	if (!creds?.session) {
		throw new Error('Not authenticated')
	}

	const params = new URLSearchParams()
	const rawServerId = route.params.id
	const serverId = Array.isArray(rawServerId) ? rawServerId[0] : rawServerId
	if (route.path.startsWith('/hosting/manage/') && typeof serverId === 'string') {
		params.set('server_id', serverId)
	}
	const query = params.size > 0 ? `?${params.toString()}` : ''

	const response = await tauriFetch(`${config.siteUrl}/api/intercom/messenger-jwt${query}`, {
		method: 'GET',
		headers: {
			Authorization: `Bearer ${creds.session}`,
		},
	})
	if (!response.ok) {
		throw new Error(`Failed to fetch Intercom token: ${response.status}`)
	}
	return await response.json()
}

watch(
	[stateInitialized, showAd, adConsentAvailable],
	async ([ready, showAds, canManageConsent]) => {
		if (!ready) return

		if (showAds) {
			await init_ads_window(true)
			return
		}

		await hide_ads_window(true)
		if (canManageConsent) {
			await init_ads_window()
		}
	},
	{ immediate: true },
)

onMounted(() => {
	invoke('show_window')

	error.setErrorModal(errorModal.value)
	error.setMinecraftAuthErrorModal(minecraftAuthErrorModal.value)
	error.setMinecraftRequiredModal(minecraftRequiredModal.value)

	setContentIncompatibilityWarningModal(incompatibilityWarningModal.value)
	setContentInstallModal(modInstallModal.value)
	setContentInstallModpackAlreadyInstalledModal(contentInstallModpackAlreadyInstalledModal.value)
	setModpackAlreadyInstalledModal(modpackAlreadyInstalledModal.value)
	setServerAddServerToInstanceModal(addServerToInstanceModal.value)
	setServerInstallToPlayModal(installToPlayModal.value)
	setServerUpdateToPlayModal(updateToPlayModal.value)
})

const sidebar = ref(null)
provideMinecraftAccounts(computed(() => sidebar.value?.minecraftAccounts ?? null))

useAppEvent('command', handleCommand, appEvents)
useAppEvent('notification', handleLiveNotification, appEvents)

async function markLiveNotificationRead(notification) {
	try {
		await tauriApiClient.labrinth.notifications_v2.markAsRead(notification.id)
	} catch (error) {
		if (error instanceof ModrinthApiError && error.statusCode === 404) {
			console.warn(`notification ${notification.id} could not be marked as read`, error)
			return
		}
		throw error
	}
}

async function respondToServerInvite(notification, action) {
	const serverId = notification.body?.server_id
	if (typeof serverId !== 'string') {
		throw new Error('Missing server ID for invite notification.')
	}

	await tauriApiClient.request(`/servers/${serverId}/invites/${action}`, {
		api: 'archon',
		version: 1,
		method: 'POST',
	})
	await markLiveNotificationRead(notification)

	return serverId
}

async function acceptServerInviteNotification(notification) {
	try {
		const serverId = await respondToServerInvite(notification, 'accept')
		await router.push(`/hosting/manage/${encodeURIComponent(serverId)}`)
		queryClient.invalidateQueries({ queryKey: ['servers'] })
	} catch (error) {
		handleError(error)
	}
}

async function declineServerInviteNotification(notification) {
	try {
		await respondToServerInvite(notification, 'decline')
	} catch (error) {
		handleError(error)
	}
}

function openServerInviteInviterProfile(inviterName) {
	if (!inviterName) return
	void router.push(`/user/${encodeURIComponent(inviterName)}`)
}

async function handleLiveNotification(notification) {
	if (!liveNotificationsEnabled || !notification?.body || notification.read) return
	if (await sharedInstanceInviteHandler.value?.handleNotification(notification)) return

	if (notification.body.type === 'server_invite') {
		if (displayedServerInviteNotifications.has(notification.id)) return

		const generation = liveNotificationGeneration
		displayedServerInviteNotifications.add(notification.id)

		const serverName =
			typeof notification.body.server_name === 'string' ? notification.body.server_name : 'a server'
		const inviterId = notification.body.invited_by
		const invitedBy =
			typeof inviterId === 'string' ? await get_user(inviterId, 'bypass').catch(() => null) : null
		if (generation !== liveNotificationGeneration) return

		const popupNotification = addPopupNotification({
			contentType: 'toast',
			title: serverName,
			type: 'server-invite',
			actorName: invitedBy?.username ?? null,
			actorAvatarUrl: invitedBy?.avatar_url ?? null,
			entityName: serverName,
			autoCloseMs: null,
			onAccept: () => acceptServerInviteNotification(notification),
			onDecline: () => declineServerInviteNotification(notification),
			onOpenActor: () => openServerInviteInviterProfile(invitedBy?.username ?? null),
		})
		serverInvitePopupNotificationIds.add(popupNotification.id)
	}
}

function clearLiveNotifications() {
	liveNotificationGeneration++
	liveNotificationsEnabled = false
	for (const id of serverInvitePopupNotificationIds) {
		popupNotificationManager.removeNotification(id)
	}
	displayedServerInviteNotifications.clear()
	serverInvitePopupNotificationIds.clear()
	sharedInstanceInviteHandler.value?.clearNotifications()
}

async function handleCommand(e) {
	if (!e) return

	if (e.event === 'RunMRPack') {
		// RunMRPack should directly install a local mrpack given a path
		if (e.path.endsWith('.mrpack')) {
			const location = { type: 'fromFile', path: e.path }
			const preview = await install_get_modpack_preview(location).catch(handleError)
			if (false) {
				// 去商业化：不再提示“非 AirCore 托管的整合包”（本项目没有托管服务）
				const splitPath = e.path.split(/[\\/]/)
				const fileName = splitPath ? splitPath[splitPath.length - 1] : e.path
				unknownPackWarningModal.value?.show(
					() => install_create_modpack_instance(location).then(() => undefined),
					fileName,
					preview.externalFilesInModpack,
				)
			} else {
				await install_create_modpack_instance(location).catch(handleError)
			}
			trackEvent('InstanceCreate', {
				source: 'CreationModalFileDrop',
			})
		}
	} else if (e.event === 'LaunchInstance') {
		const instance = await getInstance(e.id).catch(handleError)
		if (!instance || instance.quarantined) return

		if (e.server) {
			await start_join_server(e.id, e.server).catch(handleError)
		} else if (e.singleplayer_world) {
			await start_join_singleplayer_world(e.id, e.singleplayer_world).catch(handleError)
		} else {
			await run(e.id).catch(handleError)
		}
	} else if (e.event === 'InstallSharedInstanceInvite') {
		await sharedInstanceInviteHandler.value?.installFromInviteId(e.invite_id)
	} else if (e.event === 'InstallServer') {
		await router.push(`/project/${e.id}`)
		await playServerProject(e.id).catch(handleError)
	} else if (e.event === 'InstallVersion') {
		const version = await get_version(e.id, 'must_revalidate').catch(handleError)
		if (version) {
			await contentInstall
				.install(version.project_id, version.id, null, 'URLConfirmModal', undefined, undefined, {
					showProjectInfo: true,
				})
				.catch(handleError)
		}
	} else {
		await contentInstall
			.install(e.id, null, null, 'URLConfirmModal', undefined, undefined, { showProjectInfo: true })
			.catch(handleError)
	}
}

const appUpdateDownload = {
	progress: appUpdateState.progress,
	version: ref(),
}
let unlistenUpdateDownload

const {
	metered,
	finishedDownloading,
	downloading,
	restarting,
	availableUpdate,
	updateSize,
	updatesEnabled,
} = appUpdateState
let delayedUpdatePopupTimeout = null

const updatePopupMessages = defineMessages({
	updateAvailable: {
		id: 'app.update-popup.title',
		defaultMessage: 'Update available',
	},
	downloadComplete: {
		id: 'app.update-popup.download-complete',
		defaultMessage: 'Download complete',
	},
	meteredBody: {
		id: 'app.update-popup.body.metered',
		defaultMessage: `AirCore v{version} is available now! Since you're on a metered network, we didn't automatically download it.`,
	},
	downloadedBody: {
		id: 'app.update-popup.body.download-complete',
		defaultMessage: `AirCore v{version} has finished downloading. Reload to update now, or automatically when you close AirCore.`,
	},
	linuxBody: {
		id: 'app.update-popup.body.linux',
		defaultMessage:
			'AirCore v{version} is available. Use your package manager to update for the latest features and fixes!',
	},
	reload: {
		id: 'app.update-popup.reload',
		defaultMessage: 'Reload to update',
	},
	download: {
		id: 'app.update-popup.download',
		defaultMessage: 'Download ({size})',
	},
	changelog: {
		id: 'app.update-popup.changelog',
		defaultMessage: 'Changelog',
	},
})

function clearDelayedUpdatePopup() {
	if (delayedUpdatePopupTimeout !== null) {
		clearTimeout(delayedUpdatePopupTimeout)
		delayedUpdatePopupTimeout = null
	}
}

function getCurrentUpdatePromptStage() {
	return finishedDownloading.value ? 'downloaded' : 'available'
}

function scheduleDelayedUpdatePopup() {
	clearDelayedUpdatePopup()

	const version = availableUpdate.value?.version
	if (!version) {
		return
	}

	const nextPopupTime = getNextAppUpdatePopupTime(version, getCurrentUpdatePromptStage())
	if (nextPopupTime === null) {
		return
	}

	const delay = nextPopupTime - Date.now()
	if (delay <= 0) {
		showDelayedUpdatePopup()
		return
	}

	delayedUpdatePopupTimeout = setTimeout(showDelayedUpdatePopup, Math.min(delay, 2_147_483_647))
}

function showDelayedUpdatePopup() {
	const update = availableUpdate.value
	if (!update) {
		return
	}

	const stage = getCurrentUpdatePromptStage()
	const nextPopupTime = getNextAppUpdatePopupTime(update.version, stage)
	if (nextPopupTime === null) {
		return
	}

	if (Date.now() < nextPopupTime) {
		scheduleDelayedUpdatePopup()
		return
	}

	if (metered.value && !finishedDownloading.value) {
		addPopupNotification({
			contentType: 'standard',
			title: formatMessage(updatePopupMessages.updateAvailable),
			text: formatMessage(updatePopupMessages.meteredBody, { version: update.version }),
			type: 'info',
			autoCloseMs: null,
			buttons: [
				{
					label: formatMessage(updatePopupMessages.download, {
						size: formatBytes(updateSize.value ?? 0),
					}),
					action: () => downloadAvailableAppUpdate(),
					color: 'brand',
				},
				{
					label: formatMessage(updatePopupMessages.changelog),
					action: () => openAppUpdateChangelog(),
					keepOpen: true,
				},
			],
		})
	} else if (finishedDownloading.value) {
		addPopupNotification({
			contentType: 'standard',
			title: formatMessage(updatePopupMessages.downloadComplete),
			text: formatMessage(updatePopupMessages.downloadedBody, {
				version: update.version,
			}),
			type: 'success',
			autoCloseMs: null,
			buttons: [
				{
					label: formatMessage(updatePopupMessages.reload),
					action: () => installAvailableAppUpdate(),
					color: 'brand',
				},
				{
					label: formatMessage(updatePopupMessages.changelog),
					action: () => openAppUpdateChangelog(),
					keepOpen: true,
				},
			],
		})
	} else {
		scheduleDelayedUpdatePopup()
		return
	}

	markAppUpdatePopupShown(update.version, stage)
}

async function checkUpdates() {
	if (!(await areUpdatesEnabled())) {
		console.log('Skipping update check as updates are disabled in this build or environment')
		updatesEnabled.value = false

		if (os.value === 'Linux' && !isDevEnvironment.value) {
			checkLinuxUpdates()
			setInterval(checkLinuxUpdates, 5 * 60 * 1000)
		}
		return
	}

	async function performCheck() {
		const update = await invoke('plugin:updater|check')
		if (!update) {
			console.log('No update available')
			return
		}

		const isExistingUpdate = update.version === availableUpdate.value?.version

		if (isExistingUpdate) {
			console.log('Update is already known')
			scheduleDelayedUpdatePopup()
			return
		}

		appUpdateDownload.progress.value = 0
		finishedDownloading.value = false
		downloading.value = false
		updateSize.value = null
		availableUpdate.value = update

		console.log(`Update ${update.version} is available.`)

		metered.value = await isNetworkMetered()
		if (!metered.value) {
			console.log('Starting download of update')
			downloadUpdate(update)
		} else {
			console.log(`Metered connection detected, not auto-downloading update.`)
			markAppUpdateActionable(update.version)
			scheduleDelayedUpdatePopup()
		}

		getUpdateSize(update.rid).then((size) => (updateSize.value = size))
	}

	await performCheck()
	setTimeout(
		() => {
			checkUpdates()
		},
		5 /* min */ * 60 /* sec */ * 1000 /* ms */,
	)
}

async function checkLinuxUpdates() {
	try {
		const [response, currentVersion] = await Promise.all([
			fetch('https://launcher-files.modrinth.com/updates.json'),
			getVersion(),
		])
		const updates = await response.json()
		const latestVersion = updates?.version

		if (latestVersion && latestVersion !== currentVersion) {
			markAppUpdateActionable(latestVersion)
			const nextPopupTime = getNextAppUpdatePopupTime(latestVersion)
			if (nextPopupTime !== null && Date.now() >= nextPopupTime) {
				addPopupNotification({
					contentType: 'standard',
					title: formatMessage(updatePopupMessages.updateAvailable),
					text: formatMessage(updatePopupMessages.linuxBody, { version: latestVersion }),
					type: 'info',
					autoCloseMs: null,
				})
				markAppUpdatePopupShown(latestVersion)
			}
		}
	} catch (e) {
		console.error('Failed to check for updates:', e)
	}
}

async function downloadAvailableUpdate() {
	return downloadUpdate(availableUpdate.value)
}

async function downloadUpdate(versionToDownload) {
	if (!versionToDownload) {
		handleError(formatMessage(messages.updateDownloadMissingVersion))
		return
	}

	if (downloading.value || appUpdateDownload.progress.value !== 0) {
		console.error(`Update ${versionToDownload.version} already downloading`)
		return
	}

	console.log(`Downloading update ${versionToDownload.version}`)
	downloading.value = true

	try {
		enqueueUpdateForInstallation(versionToDownload.rid)
			.then(() => {
				downloading.value = false
				finishedDownloading.value = true
				unlistenUpdateDownload?.()
				unlistenUpdateDownload = null
				console.log('Finished downloading!')
				markAppUpdateActionable(versionToDownload.version, 'downloaded')
				scheduleDelayedUpdatePopup()
			})
			.catch((e) => {
				downloading.value = false
				appUpdateDownload.progress.value = 0
				handleError(e)
			})
		unlistenUpdateDownload = await subscribeToDownloadProgress(
			appEvents,
			appUpdateDownload,
			versionToDownload.version,
		)
	} catch (e) {
		downloading.value = false
		appUpdateDownload.progress.value = 0
		handleError(e)
	}
}

async function installUpdate() {
	restarting.value = true

	try {
		await setRestartAfterPendingUpdate(true)
	} catch (e) {
		restarting.value = false
		handleError(e)
		return
	}
	setTimeout(async () => {
		await handleClose()
	}, 250)
}

setAppUpdateActions({
	download: downloadAvailableUpdate,
	install: installUpdate,
	changelog: () => openUrl('https://modrinth.com/news/changelog?filter=app'),
})

async function openModrinthProjectLinkInApp(parsed) {
	const { slug, pathSuffix, url } = parsed
	const loadToken = loading.begin()
	try {
		const { id } = await tauriApiClient.labrinth.projects_v2.check(slug)
		const query = mergeUrlQuery(route.query, url)
		await router.push({
			path: `/project/${id}${pathSuffix}`,
			query,
			hash: url.hash || undefined,
		})
	} catch (err) {
		if (err instanceof ModrinthApiError && err.statusCode === 404) {
			openUrl(url.href)
		} else {
			handleError(err)
		}
	} finally {
		loading.end(loadToken)
	}
}

function handleClick(e) {
	let target = e.target
	while (target != null) {
		if (target.matches('a')) {
			if (
				target.href &&
				['http://', 'https://', 'mailto:', 'tel:'].some((v) => target.href.startsWith(v)) &&
				!target.classList.contains('router-link-active') &&
				!target.href.startsWith('http://localhost') &&
				!target.href.startsWith('https://tauri.localhost') &&
				!target.href.startsWith('http://tauri.localhost')
			) {
				const userPath = parse_modrinth_user_link(target.href)
				const parsed = parseModrinthLink(target.href)
				if (userPath) {
					void router.push(userPath)
				} else if (target.target !== '_blank' && parsed) {
					void openModrinthProjectLinkInApp(parsed)
				} else {
					openUrl(target.href)
				}
			}
			e.preventDefault()
			break
		}
		target = target.parentElement
	}
}

function handleAuxClick(e) {
	// disables middle click -> new tab
	if (e.button === 1) {
		e.preventDefault()
		// instead do a left click
		const event = new MouseEvent('click', {
			view: window,
			bubbles: true,
			cancelable: true,
		})
		e.target.dispatchEvent(event)
	}
}

function handleContextMenu(event) {
	const target = event.target
	if (target instanceof Element) {
		if (target.closest('img, textarea, [contenteditable="true"]')) return
		const input = target.closest('input')
		if (
			input &&
			!['button', 'checkbox', 'radio', 'submit', 'reset', 'file', 'range'].includes(input.type)
		) {
			return
		}
	}

	const selection = window.getSelection()
	if (
		target instanceof Node &&
		selection &&
		!selection.isCollapsed &&
		selection.containsNode(target, true)
	) {
		return
	}

	event.preventDefault()
}

provideAppUpdateDownloadProgress(appUpdateDownload)
</script>

<template>
	<TooltipDirective />
	<SplashScreen v-if="!stateFailed" ref="splashScreen" data-tauri-drag-region />
	<div id="teleports"></div>
	<AccountSwitchOverlay :show="isSwitchingAccount" />
	<div
		v-if="stateInitialized"
		class="app-grid-layout relative"
		:class="{ 'disable-advanced-rendering': !appTheme.advancedRendering }"
	>
		<Transition name="fade">
			<div
				v-if="restarting"
				data-tauri-drag-region
				class="inset-0 fixed bg-black/80 backdrop-blur z-[200] flex items-center justify-center"
			>
				<span
					data-tauri-drag-region
					class="flex items-center gap-4 text-contrast font-semibold text-xl select-none cursor-default"
				>
					<RefreshCwIcon data-tauri-drag-region class="animate-spin w-6 h-6" />
					{{ formatMessage(messages.restarting) }}
				</span>
			</div>
		</Transition>
		<AppSettingsModal ref="appSettingsModal" />
		<NewUpdateModal ref="newUpdateModal" />
		<Suspense>
			<ModrinthAccountRequiredModal ref="modrinthLoginModal" :request-auth="requestModrinthAuth" />
		</Suspense>
		<CreationFlowModal
			ref="installationModal"
			type="instance"
			show-snapshot-toggle
			:fetch-existing-instance-names="fetchExistingInstanceNames"
			:search-projects="searchProjects"
			:prepare-project-install="prepareCreationProjectInstall"
			:create-project-install="handleCreateAndInstall"
			:get-loader-manifest="getLoaderManifest"
			:randomize-instance-icon="randomizeCreationIcon"
			:customize-instance-icon="customizeCreationIcon"
			@create="handleCreate"
			@browse-modpacks="handleBrowseModpacks"
		/>
		<IconEditorModal
			ref="creationIconEditorModal"
			:config="creationGeneratedIcon?.config"
			@saved="onCreationIconSaved"
		/>
		<UnknownPackWarningModal ref="unknownPackWarningModal" />
		<div
			class="app-grid-navbar bg-bg-raised flex flex-col p-[0.5rem] pt-0 gap-[0.25rem] w-[--left-bar-width]"
		>
			<NavButton
				v-tooltip.right="formatMessage(messages.home)"
				to="/"
				:is-primary="(route) => route.path === '/'"
				:is-subpage="
					() =>
						(route.path.startsWith('/browse') || route.path.startsWith('/project')) && route.query.i
				"
			>
				<PlayIcon class="ml-0.5" />
			</NavButton>
				<NavButton
					v-tooltip.right="formatMessage(messages.libraryLabel)"
					to="/library"
					:is-primary="() => route.path === '/library'"
				>
					<LibraryIcon />
				</NavButton>
			<NavButton
				v-if="!isNavHidden('browse')"
				v-tooltip.right="formatMessage(commonMessages.discoverContentLabel)"
				to="/browse/modpack"
				:is-primary="() => route.path.startsWith('/browse') && !route.query.i && !route.query.sid"
				:is-subpage="
					(route) => route.path.startsWith('/project') && !route.query.i && !route.query.sid
				"
			>
				<CompassIcon />
			</NavButton>

			<!-- 插件面板 -->
			<NavButton
				v-for="panel in pluginRegistry.navItems"
				:key="panel.id"
				v-tooltip.right="panel.title"
				:to="`/ext/${panel.id}`"
				:is-primary="() => route.path === `/ext/${panel.id}`"
			>
				<img
					v-if="typeof panel.icon === 'string'"
					:src="panel.icon"
					alt=""
					class="h-6 w-6 rounded"
				/>
				<component :is="panel.icon ?? ToggleRightIcon" v-else />
			</NavButton>

			<!-- 开发者模式下的组件画廊入口 -->
			<NavButton
				v-if="appSettings.devMode && !isNavHidden('gallery')"
				v-tooltip.right="formatMessage(messages.pluginLogs)"
				to="/gallery"
			>
				<ToggleRightIcon />
			</NavButton>

			<NavButton
				v-if="appSettings.showSkinSelectorInSidebar && !isNavHidden('skins')"
				v-tooltip.right="formatMessage(appMessages.skinSelectorLabel)"
				to="/skins"
			>
				<ShirtIcon />
			</NavButton>
			<NavButton
				v-if="globalSyncedOptionsQuery.data.value?.screenshots && !isNavHidden('screenshots')"
				v-tooltip.right="formatMessage(messages.screenshots)"
				to="/screenshots"
			>
				<ImageIcon />
			</NavButton>
			<!-- 去商业化：AirCore Hosting 入口已移除 -->
			<NavButton
				v-if="false"
				v-tooltip.right="formatMessage(messages.modrinthHosting)"
				to="/hosting/manage"
				:is-primary="(r) => r.path === '/hosting/manage' || r.path === '/hosting/manage/'"
				:is-subpage="
					(r) =>
						(r.path.startsWith('/hosting/manage/') && r.path !== '/hosting/manage/') ||
						((r.path.startsWith('/browse') || r.path.startsWith('/project')) && r.query.sid)
				"
			>
				<ServerStackIcon />
			</NavButton>
			<suspense>
				<QuickInstanceSwitcher />
			</suspense>
			<!-- 新建实例：放在底部，紧贴设置图标上方 -->
			<NavButton
				v-tooltip.right="formatMessage(messages.createNewInstance)"
				:to="() => installationModal?.show()"
				:disabled="offline"
			>
				<PlusIcon />
			</NavButton>
			<NavButton
				v-tooltip.right="formatMessage(commonMessages.settingsLabel)"
				:to="settingsActions.openSettings"
			>
				<SettingsIcon />
			</NavButton>
			<IconButton
				v-if="credentials === undefined"
				v-tooltip.right="profileButtonTooltip"
				type="quiet"
				size="xl"
				disabled
				class="pointer-events-none"
				:label="formatMessage(messages.loadingProfile)"
			>
				<SpinnerIcon class="animate-spin" />
			</IconButton>
			<TeleportOverflowMenu
				v-else-if="credentials?.user"
				v-tooltip.right="profileButtonTooltip"
				type="quiet"
				size="xl"
				:label="formatMessage(messages.modrinthAccount)"
				:options="modrinthAccountMenuOptions"
				placement="right-end"
				:distance="4"
				class="brightness-100 hover:!brightness-100 focus-visible:!brightness-100"
			>
				<Avatar
					:src="credentials?.user?.avatar_url"
					alt=""
					size="32px"
					circle
					no-shadow
					class="pointer-events-none !size-8"
				/>
				<template
					v-for="account in accountSwitcherAccounts"
					:key="account.user_id"
					#[account.optionId]
				>
					<Avatar :src="account.user.avatar_url" size="1.25rem" aria-hidden="true" circle />
					{{ account.user.username }}
					<UserRoleIcon :role="account.user.role" />
				</template>
			</TeleportOverflowMenu>
			<TeleportOverflowMenu
				v-else-if="accountSwitcherAccounts.length > 0"
				v-tooltip.right="profileButtonTooltip"
				type="quiet"
				size="xl"
				:label="formatMessage(messages.signInToModrinthAccount)"
				:options="accountSwitcherOptions"
				placement="right-end"
				:distance="4"
			>
				<LogInIcon class="!text-brand" />
				<template
					v-for="account in accountSwitcherAccounts"
					:key="account.user_id"
					#[account.optionId]
				>
					<Avatar :src="account.user.avatar_url" size="1.25rem" aria-hidden="true" circle />
					{{ account.user.username }}
					<UserRoleIcon :role="account.user.role" />
				</template>
			</TeleportOverflowMenu>
			<!-- 去商业化：左下角的 Modrinth 账号登录按钮已移除 -->
		</div>
		<AppTitleBar
			:can-navigate-back="canNavigateBack"
			:can-navigate-forward="canNavigateForward"
			:show-sidebar-toggle="!forceSidebar && appSettings.toggleSidebar"
			:sidebar-toggled="sidebarToggled"
			@toggle-sidebar="sidebarToggled = !sidebarToggled"
		/>
	</div>
	<div
		v-if="stateInitialized"
		class="app-contents"
		:style="{ '--right-bar-width': sidebarVisible ? '300px' : '0px' }"
		:class="{
			'sidebar-enabled': sidebarVisible,
			'disable-advanced-rendering': !appTheme.advancedRendering,
		}"
	>
		<div
			ref="viewportEl"
			class="app-viewport flex-grow router-view"
			:style="{ marginRight: sidebarVisible ? '300px' : '0px' }"
		>
			<SurveyPopup />
						<Teleport to="body">
	<div
					class="loading-indicator-container h-8 fixed z-50 pointer-events-none"
					:style="{
						top: '3rem',
						left: '4rem',
						right: sidebarVisible ? '300px' : '0px',
					}"
				>
					<LoadingBar position="absolute" />
				</div>
			</Teleport>
			<div
				v-if="appSettings.featureFlags.page_path"
				class="absolute bottom-0 left-0 m-2 bg-tooltip-bg text-tooltip-text font-semibold rounded-full px-2 py-1 text-xs z-50"
			>
				{{ route.fullPath }}
			</div>
			<div
				id="background-teleport-target"
				class="absolute h-full -z-10 rounded-tl-[--radius-xl] overflow-hidden"
				:style="{
					width: sidebarVisible ? 'calc(100% - 300px)' : '100%',
				}"
			></div>
			<Admonition
				v-if="criticalErrorMessage"
				type="critical"
				:header="criticalErrorMessage.header"
				class="m-6 mb-0"
			>
				<div
					class="markdown-body text-primary"
					v-html="renderString(criticalErrorMessage.body ?? '')"
				></div>
			</Admonition>
			<Admonition
				v-if="authUnreachable"
				type="warning"
				:header="formatMessage(messages.authUnreachableHeader)"
				class="m-6 mb-0"
			>
				{{ formatMessage(messages.authUnreachableBody) }}
			</Admonition>
			<!-- hosting removed -->
			<RouterView v-else v-slot="{ Component }">
				<template v-if="Component">
					<Suspense @pending="onSuspensePending" @resolve="onSuspenseResolve">
						<component :is="Component"></component>
					</Suspense>
				</template>
			</RouterView>
		</div>
		<AppSidebar
			ref="sidebar"
			:credentials="credentials"
			:has-plus="hasPlus"
			:show-ad="showAd"
			:visible="sidebarVisible"
			:request-sign-in="() => requestSignIn()"
			@create-instance="installationModal?.show()"
			@login-modrinth="signIn"
		/>
	</div>
	<SandboxHost />
	<I18nDebugPanel />
	<NotificationPanel :has-sidebar="sidebarVisible" />
	<PopupNotificationPanel :has-sidebar="sidebarVisible" />
		<Teleport to="body">
			<div class="pointer-events-none fixed bottom-6 right-6 z-[9999] flex flex-col gap-2">
				<div
					v-for="toast in pluginToasts"
					:key="toast.id"
					class="pointer-events-auto max-w-sm rounded-xl border border-solid border-surface-4 bg-surface-3 px-4 py-3 text-sm text-contrast shadow-lg"
				>
					{{ toast.message }}
				</div>
			</div>
		</Teleport>
	<ErrorModal ref="errorModal" />
	<MinecraftAuthErrorModal ref="minecraftAuthErrorModal" />
	<MinecraftRequiredModal ref="minecraftRequiredModal" />
	<ContentInstallModal
		ref="modInstallModal"
		:instances="contentInstallInstances"
		:compatible-loaders="contentInstallLoaders"
		:game-versions="contentInstallGameVersions"
		:loading="contentInstallLoading"
		:default-tab="contentInstallDefaultTab"
		:preferred-loader="contentInstallPreferredLoader"
		:preferred-game-version="contentInstallPreferredGameVersion"
		:release-game-versions="contentInstallReleaseGameVersions"
		:project-info="contentInstallProjectInfo"
		:randomize-icon="randomizeCreationIcon"
		:customize-icon="customizeContentInstallIcon"
		@install="handleInstallToInstance"
		@create-and-install="handleCreateAndInstall"
		@navigate="handleContentInstallNavigate"
		@cancel="handleContentInstallCancel"
	/>
	<ModpackAlreadyInstalledModal
		ref="modpackAlreadyInstalledModal"
		@create-anyway="handleModpackDuplicateCreateAnyway"
		@go-to-instance="handleModpackDuplicateGoToInstance"
	/>
	<AddServerToInstanceModal ref="addServerToInstanceModal" />
	<ContentUpdaterModal
		ref="incompatibilityWarningModal"
		mode="incompatibility-warning"
		:versions="contentInstallIncompatibilityWarningVersions"
		:current-game-version="contentInstallIncompatibilityWarningCurrentGameVersion"
		:current-loader="contentInstallIncompatibilityWarningCurrentLoader"
		current-version-id=""
		:is-app="true"
		:project-type="contentInstallIncompatibilityWarningProjectType"
		:project-icon-url="contentInstallIncompatibilityWarningProjectIconUrl"
		:project-name="contentInstallIncompatibilityWarningProjectName"
		:warning="contentInstallIncompatibilityWarningMessage"
		:action-loading="contentInstallIncompatibilityWarningInstalling"
		@update="handleContentInstallIncompatibilityWarningInstall"
		@cancel="handleContentInstallIncompatibilityWarningCancel"
	/>
	<ModpackAlreadyInstalledModal
		ref="contentInstallModpackAlreadyInstalledModal"
		@create-anyway="handleContentInstallModpackDuplicateCreateAnyway"
		@go-to-instance="handleContentInstallModpackDuplicateGoToInstance"
	/>
	<SharedInstanceInviteHandler ref="sharedInstanceInviteHandler" />
	<InstallToPlayModal ref="installToPlayModal" :show-external-warnings="false" />
	<UpdateToPlayModal ref="updateToPlayModal" :show-external-warnings="false" />
</template>

<style lang="scss">
/* 侧栏切换的瞬间，禁用内容区所有 CSS 过渡，防止卡片被拉伸/缩放 */
body.sidebar-resizing * {
	transition-duration: 0s !important;
	transition-delay: 0s !important;
}
</style>

<style lang="scss" scoped>
.app-grid-layout,
.app-contents {
	--top-bar-height: 3rem;
	--left-bar-width: 4rem;
	--right-bar-width: 300px;
}

.app-grid-layout {
	display: grid;
	grid-template: 'status status' 'nav dummy';
	grid-template-columns: auto 1fr;
	grid-template-rows: auto minmax(0, 1fr);
	position: relative;
	//z-index: 0;
	background-color: var(--color-raised-bg);
	height: 100vh;
}

.app-grid-navbar {
	grid-area: nav;
	min-height: 0;
	position: relative;
	z-index: 2;

	> :deep(*) {
		flex-shrink: 0;
	}
}

[data-tauri-drag-region-exclude] {
	-webkit-app-region: no-drag;
}

.app-contents {
	position: absolute;
	z-index: 1;
	left: var(--left-bar-width);
	top: var(--top-bar-height);
	right: 0;
	bottom: 0;
	height: calc(100vh - var(--top-bar-height));
	background-color: var(--color-bg);
	border-top-left-radius: var(--radius-xl);

	display: grid;
	grid-template-columns: 1fr 0px;

		&.sidebar-enabled {
		grid-template-columns: 1fr 0px;
	}

	/* 侧栏：绝对定位；无论何时由类决定位置，不会与 JS 状态脱节 */
	.app-sidebar {
		position: absolute;
		top: 0;
		right: 0;
		height: 100%;
		width: 300px;
		transform: translateX(0);
	}

	&:not(.sidebar-enabled) .app-sidebar {
		transform: translateX(100%);
	}

	.app-viewport {
		scrollbar-gutter: stable;
		contain: paint;
	}

	&.sidebar-enabled .app-viewport {
		margin-right: 300px;
	}

	&.sidebar-shifting .app-viewport {
		pointer-events: none;
	}
}

.loading-indicator-container {
	border-top-left-radius: var(--radius-xl);
	overflow: hidden;
}

.disable-advanced-rendering {
	&.app-contents::before {
		box-shadow: none;
	}

	*,
	:deep(*) {
		box-shadow: none !important;
		--tw-drop-shadow:;
	}
}

.app-viewport {
	flex-grow: 1;
	height: 100%;
	overflow: auto;
	overflow-x: hidden;
	scrollbar-gutter: stable;
}

.app-contents::before {
	z-index: 30;
	content: '';
	position: fixed;
	left: var(--left-bar-width);
	top: var(--top-bar-height);
	right: calc(-1 * var(--left-bar-width));
	bottom: calc(-1 * var(--left-bar-width));
	border-radius: var(--radius-xl);
	box-shadow: 1px 1px 15px rgba(0, 0, 0, 0.1) inset;
	border-color: var(--surface-5);
	border-width: 1px;
	border-style: solid;
	pointer-events: none;
}

@media (prefers-reduced-motion: no-preference) {
	.nav-button-animated-enter-active {
		transition: all 0.5s cubic-bezier(0.15, 1.4, 0.64, 0.96);
	}

	.nav-button-animated-leave-active {
		transition: all 0.25s ease;
	}

	.nav-button-animated-enter-active {
		position: relative;
	}

	.nav-button-animated-enter-active::before {
		content: '';
		inset: 0;
		border-radius: 100vw;
		background-color: var(--color-brand-highlight);
		position: absolute;
		animation: pop 0.5s ease-in forwards;
		opacity: 0;
	}

	@keyframes pop {
		0% {
			scale: 0.5;
		}
		50% {
			opacity: 0.5;
		}
		100% {
			scale: 1.5;
		}
	}

	.nav-button-animated-enter-from {
		scale: 0.5;
		translate: -2rem 0;
		opacity: 0;
	}

	.nav-button-animated-leave-to {
		scale: 0.75;
		opacity: 0;
	}

	.fade-enter-active {
		transition: 0.25s ease-in-out;
	}

	.fade-enter-from {
		opacity: 0;
	}
}
</style>
<style>
.os-theme-dark,
.os-theme-light {
	--os-handle-bg: var(--color-scrollbar) !important;
	--os-handle-bg-hover: var(--color-scrollbar) !important;
	--os-handle-bg-active: var(--color-scrollbar) !important;
}

.windows {
	.fake-appbar {
		height: 2.5rem !important;
	}

	.info-card {
		right: 22rem;
	}

	.profile-card {
		right: 8rem;
	}
}
</style>

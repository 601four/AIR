import { reactive, ref } from 'vue'

export const DEFAULT_FEATURE_FLAGS = {
	project_background: false,
	page_path: false,
	server_project_qa: false,
	show_version_environment_column: false,
	server_ram_as_bytes_always_on: false,
	always_show_app_controls: false,
	show_sync_instances_update_modal: false,
	pride_fundraiser: true,
	i18n_debug: false,
	localhost_sign_in: false,
}

export type FeatureFlag = keyof typeof DEFAULT_FEATURE_FLAGS
type FeatureFlags = Record<FeatureFlag, boolean>

const featureFlags = reactive<FeatureFlags>({ ...DEFAULT_FEATURE_FLAGS })

function getFeatureFlag(key: FeatureFlag): boolean {
	return featureFlags[key] ?? DEFAULT_FEATURE_FLAGS[key]
}

const appSettings = reactive({
	showJumpIn: true,
	alwaysShowCopyDetails: false,
	hideInstalledModpacks: false,
	advancedFiltersCollapsed: true,
	dismissedPhotosensitivityFilterWarning: false,
	friendsActiveCollapsed: false,
	friendsOnlineCollapsed: false,
	friendsOfflineCollapsed: true,
	friendsPendingCollapsed: true,
	refocusOnGameClose: false,
	compactInstanceCards: false,
	showPlayTime: true,
	warnOnUnknownModpacks: true,
	skipNonEssentialWarnings: false,
	hideNametagSkinsPage: false,
	toggleSidebar: false,
	showFilesTabInInstances: true,
	showWorldsTabInInstances: true,
	showScreenshotsTabInInstances: false,
	showSkinSelectorInSidebar: true,
	nativeDecorations: false,
	devMode: false,
	accentColor: '#ff5050',
	autoMemory: false,
	prelaunchOptimize: false,
	downloadSource: 'official',
	loaderMode: 'cn',
	loaderPrefix: '',
	modrinthMirror: false,
	sidebarNews: true,
	minimizeToTray: true,
	featureFlags,
	getFeatureFlag,
})

export function useAppSettings() {
	return appSettings
}


/** 保存 AirCore 自定义字段到 settings 表（当前仅用于强调色 / 内存开关的立即生效场景） */
export async function updateAirCoreSettings(patch: Record<string, unknown>): Promise<void> {
	const { get, set } = await import('@/helpers/settings.ts')
	const latest = await get()
	await set({ ...latest, ...patch } as never)
}

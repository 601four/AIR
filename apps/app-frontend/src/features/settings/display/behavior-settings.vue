<script setup lang="ts">
import {
	defineMessages,
	injectNotificationManager,
	Toggle,
	useSavable,
	useVIntl,
} from '@modrinth/ui'
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { onBeforeUnmount, onMounted } from 'vue'

import { useAppSettings } from '@/composables/use-app-settings.ts'
import { injectAppSettingsModal } from '@/features/settings/app-settings-modal/context'
import {
	type AppSettings,
	appSettingsKeys,
	appSettingsQueryOptions,
	get,
	set,
} from '@/helpers/settings.ts'

const appSettings = useAppSettings()
const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const settingsModal = injectAppSettingsModal()
const queryClient = useQueryClient()

const messages = defineMessages({
	startupAndNavigationTitle: {
		id: 'app.behavior-settings.startup-and-navigation.title',
		defaultMessage: 'Startup and navigation',
	},
	contentTitle: {
		id: 'app.behavior-settings.content.title',
		defaultMessage: 'Home and content',
	},
	confirmationsTitle: {
		id: 'app.behavior-settings.confirmations.title',
		defaultMessage: 'Confirmations',
	},
	minimizeLauncherTitle: {
		id: 'app.appearance-settings.minimize-launcher.title',
		defaultMessage: 'Minimize app',
	},
	minimizeLauncherDescription: {
		id: 'app.appearance-settings.minimize-launcher.description',
		defaultMessage: 'Minimize AirCore when Minecraft starts.',
	},
	refocusOnGameCloseTitle: {
		id: 'app.behavior-settings.refocus-on-game-close.title',
		defaultMessage: 'Refocus app when Minecraft closes',
	},
	refocusOnGameCloseDescription: {
		id: 'app.behavior-settings.refocus-on-game-close.description',
		defaultMessage: 'Bring AirCore to the foreground when Minecraft exits.',
	},
	defaultLandingPageHome: {
		id: 'app.appearance-settings.default-landing-page.home',
		defaultMessage: 'Home',
	},
	defaultLandingPageLibrary: {
		id: 'app.appearance-settings.default-landing-page.library',
		defaultMessage: 'Library',
	},
	toggleSidebarTitle: {
		id: 'app.appearance-settings.toggle-sidebar.title',
		defaultMessage: 'Hide right sidebar',
	},
	toggleSidebarDescription: {
		id: 'app.appearance-settings.toggle-sidebar.description',
		defaultMessage: 'Hide the right sidebar by default and add a button to show or hide it.',
	},
	compactModeTitle: {
		id: 'app.appearance-settings.compact-mode.title',
		defaultMessage: 'Compact mode',
	},
	compactModeDescription: {
		id: 'app.appearance-settings.compact-mode.description',
		defaultMessage: 'Display library instances in a compact row layout.',
	},
	showPlayTimeTitle: {
		id: 'app.appearance-settings.show-play-time.title',
		defaultMessage: 'Show play time',
	},
	showPlayTimeDescription: {
		id: 'app.appearance-settings.show-play-time.description',
		defaultMessage: `Show how long you've played each instance.`,
	},
	hideNametagTitle: {
		id: 'app.appearance-settings.hide-nametag.title',
		defaultMessage: 'Hide nametag',
	},
	hideNametagDescription: {
		id: 'app.appearance-settings.hide-nametag.description',
		defaultMessage: 'Hide your username above the player preview on the Skin selector page.',
	},
	unknownPackWarningTitle: {
		id: 'app.appearance-settings.unknown-pack-warning.title',
		defaultMessage: 'Warn me before installing unknown modpacks',
	},
	unknownPackWarningDescription: {
		id: 'app.appearance-settings.unknown-pack-warning.description',
		defaultMessage:
			"Show a safety warning before installing a AirCore Pack (.mrpack) that isn't hosted on Modrinth.",
	},
	skipNonEssentialWarningsTitle: {
		id: 'app.appearance-settings.skip-non-essential-warnings.title',
		defaultMessage: 'Skip non-essential warnings',
	},
	skipNonEssentialWarningsDescription: {
		id: 'app.appearance-settings.skip-non-essential-warnings.description',
		defaultMessage:
			'Skip confirmations for low-risk actions such as duplicate installs, normal content deletion, bulk updates, unlinking, and repairs. Warnings for dangerous actions are always shown.',
	},
})

type BehaviorSettingsState = {
	minimizeApp: boolean
	refocusOnGameClose: boolean
	hideRightSidebar: boolean
	compactInstanceCards: boolean
	showPlayTime: boolean
	hideNametag: boolean
	warnOnUnknownModpacks: boolean
	skipNonEssentialWarnings: boolean
}

const settingsQuery = useQuery(appSettingsQueryOptions())
await settingsQuery.suspense()

function getBehaviorSettingsState(settings: AppSettings): BehaviorSettingsState {
	return {
		minimizeApp: settings.hide_on_process_start,
		refocusOnGameClose: settings.refocus_on_game_close,
		compactInstanceCards: settings.compact_instance_cards,
		showPlayTime: settings.show_play_time,
		warnOnUnknownModpacks: settings.warn_on_unknown_modpacks,
		skipNonEssentialWarnings: settings.skip_non_essential_warnings,
		hideRightSidebar: settings.toggle_sidebar,
		hideNametag: settings.hide_nametag_skins_page,
	}
}

const settingsMutation = useMutation({
	mutationKey: appSettingsKeys.update,
	scope: { id: 'app-settings' },
	mutationFn: async (value: BehaviorSettingsState) => {

		const latestSettings = await get()
		const nextSettings: AppSettings = {
			...latestSettings,
			hide_on_process_start: value.minimizeApp,
			toggle_sidebar: value.hideRightSidebar,
			hide_nametag_skins_page: value.hideNametag,
			refocus_on_game_close: value.refocusOnGameClose,
			compact_instance_cards: value.compactInstanceCards,
			show_play_time: value.showPlayTime,
			warn_on_unknown_modpacks: value.warnOnUnknownModpacks,
			skip_non_essential_warnings: value.skipNonEssentialWarnings,
		}

		await set(nextSettings)
		queryClient.setQueryData(appSettingsKeys.all, nextSettings)
		appSettings.refocusOnGameClose = value.refocusOnGameClose
		appSettings.compactInstanceCards = value.compactInstanceCards
		appSettings.showPlayTime = value.showPlayTime
		appSettings.warnOnUnknownModpacks = value.warnOnUnknownModpacks
		appSettings.skipNonEssentialWarnings = value.skipNonEssentialWarnings
		appSettings.toggleSidebar = value.hideRightSidebar
		appSettings.hideNametagSkinsPage = value.hideNametag
	},
	onMutate: () => queryClient.cancelQueries({ queryKey: appSettingsKeys.all }),
	onError: handleError,
	onSettled: () => queryClient.invalidateQueries({ queryKey: appSettingsKeys.all }),
})

const { saved, current, changes, saving, hasChanges, reset, save } = useSavable(
	() => getBehaviorSettingsState(settingsQuery.data.value!),
	() => settingsMutation.mutateAsync({ ...current.value }),
)

async function saveBehaviorSettings(): Promise<void> {
	try {
		await save()
	} catch {
		return
	}
}

onMounted(() => {
	settingsModal.registerUnsavedChangesController({
		hasChanges: () => hasChanges.value,
		getOriginal: () => saved.value,
		getModified: () => changes.value,
		isSaving: () => saving.value,
		reset,
		save: saveBehaviorSettings,
	})
})

onBeforeUnmount(() => {
	settingsModal.registerUnsavedChangesController(null)
})
</script>
<template>
	<section class="mt-0">
		<h2 class="m-0 text-xl font-semibold text-contrast">
			{{ formatMessage(messages.startupAndNavigationTitle) }} <!-- 显示：启动与导航 -->
		</h2>
		<div class="mt-4 flex flex-col gap-6">
			<div class="flex items-center justify-between gap-4">
				<div>
					<h3 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.minimizeLauncherTitle) }} <!-- 显示：最小化 App -->
					</h3>
					<p class="m-0 mt-1">
						{{ formatMessage(messages.minimizeLauncherDescription) }} <!-- 显示：当 Minecraft 进程启动时，将 AirCore 最小化。 -->
					</p>
				</div>
				<Toggle id="minimize-launcher" v-model="current.minimizeApp" />
			</div>

			<div class="flex items-center justify-between gap-4">
				<div>
					<h3 id="refocus-on-game-close-label" class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.refocusOnGameCloseTitle) }} <!-- 显示：当《我的世界》关闭时，重新聚焦于AirCore -->
					</h3>
					<p id="refocus-on-game-close-description" class="m-0 mt-1">
						{{ formatMessage(messages.refocusOnGameCloseDescription) }} <!-- 显示：当 Minecraft 退出时，将AirCore调至前台。 -->
					</p>
				</div>
				<Toggle
					id="refocus-on-game-close"
					v-model="current.refocusOnGameClose"
					aria-labelledby="refocus-on-game-close-label"
					aria-describedby="refocus-on-game-close-description"
				/>
			</div>

			<div class="flex items-center justify-between gap-4">
				<div>
					<h3 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.toggleSidebarTitle) }} <!-- 显示：隐藏右侧边栏 -->
					</h3>
					<p class="m-0 mt-1">{{ formatMessage(messages.toggleSidebarDescription) }}</p>  <!-- 显示：默认隐藏右侧边栏，并添加一个按钮来显示或隐藏它。 -->
				</div>
				<Toggle id="toggle-sidebar" v-model="current.hideRightSidebar" />
			</div>
		</div>
	</section>

	<section class="mt-8 border-0 border-t border-solid border-divider pt-6">
		<h2 class="m-0 text-xl font-semibold text-contrast">
			{{ formatMessage(messages.contentTitle) }} <!-- 显示：主页与内容 -->
		</h2>
		<div class="mt-4 flex flex-col gap-6">
			<div class="flex items-center justify-between gap-4">
				<div>
					<h3 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.compactModeTitle) }} <!-- 显示：紧凑模式 -->
					</h3>
					<p class="m-0 mt-1">{{ formatMessage(messages.compactModeDescription) }}</p>  <!-- 显示：用紧凑行式布局显示库实例。 -->
				</div>
				<Toggle id="compact-mode" v-model="current.compactInstanceCards" />
			</div>

			<div class="flex items-center justify-between gap-4">
				<div>
					<h3 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.showPlayTimeTitle) }} <!-- 显示：显示游玩时间 -->
					</h3>
					<p class="m-0 mt-1">{{ formatMessage(messages.showPlayTimeDescription) }}</p>  <!-- 显示：显示您在每个实例中游玩的时间。 -->
				</div>
				<Toggle id="show-play-time" v-model="current.showPlayTime" />
			</div>

			<div class="flex items-center justify-between gap-4">
				<div>
					<h3 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.hideNametagTitle) }} <!-- 显示：隐藏名牌 -->
					</h3>
					<p class="m-0 mt-1">{{ formatMessage(messages.hideNametagDescription) }}</p>  <!-- 显示：在皮肤选择器页面的玩家预览上隐藏用户名。 -->
				</div>
				<Toggle id="hide-nametag-skins-page" v-model="current.hideNametag" />
			</div>
		</div>
	</section>

	<section class="mt-8 border-0 border-t border-solid border-divider pt-6">
		<h2 class="m-0 text-xl font-semibold text-contrast">
			{{ formatMessage(messages.confirmationsTitle) }} <!-- 显示：安全确认 -->
		</h2>
		<div class="mt-4 flex flex-col gap-6">
			<div class="flex items-center justify-between gap-4">
				<div>
					<h3 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.skipNonEssentialWarningsTitle) }} <!-- 显示：跳过非必要的警告 -->
					</h3>
					<p class="m-0 mt-1">
						{{ formatMessage(messages.skipNonEssentialWarningsDescription) }} <!-- 显示：自动跳过低风险确认，包括重复安装整合包、常规内容删除、批量更新、解除整合包关联及修复提示。危险警告仍会显示。 -->
					</p>
				</div>
				<Toggle id="skip-non-essential-warnings" v-model="current.skipNonEssentialWarnings" />
			</div>
		</div>
	</section>
</template>

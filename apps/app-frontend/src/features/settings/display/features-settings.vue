<!--
  这个文件是「设置 → 功能」页的界面。
  模板里的 {{ formatMessage(messages.xxx) }} 是“文字查表”：
    · messages.xxx 定义在同一文件下方的 defineMessages 里（带 id 和说明）
    · 实际显示的中文在 locales/zh-CN/index.json 里，按 id 查找
  为方便你阅读，每个占位符后面都添加了「显示：真实文字」的注释。
-->
<script setup lang="ts">
import {
	defineMessages,
	injectNotificationManager,
	Slider,
	Toggle,
	useSavable,
	useVIntl,
} from '@modrinth/ui'
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { onBeforeUnmount, onMounted } from 'vue'

import { useAppSettings } from '@/composables/use-app-settings.ts'
import {
	QUICK_INSTANCE_LIMIT_MAX,
	useQuickInstanceLimit,
} from '@/composables/use-quick-instance-limit.ts'
import { injectAppSettingsModal } from '@/features/settings/app-settings-modal/context'
import { type GlobalSyncedOptions, set_global_synced_option } from '@/helpers/instance.ts'
import {
	type AppSettings,
	appSettingsKeys,
	appSettingsQueryOptions,
	get,
	set,
} from '@/helpers/settings.ts'
import { globalSyncedOptionsQueryOptions, syncedOptionsKeys } from '@/helpers/synced-options'
import { screenshotKeys } from '@/pages/instance/query-options.ts'

const appSettings = useAppSettings()
const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const settingsModal = injectAppSettingsModal()
const quickInstances = useQuickInstanceLimit()
const queryClient = useQueryClient()

const messages = defineMessages({
	instancePagesTitle: {
		id: 'app.features-settings.instance-pages.title',
		defaultMessage: 'Instances',
	},
	showWorldsTabTitle: {
		id: 'app.features-settings.show-worlds-tab.title',
		defaultMessage: 'Show Worlds tab in instances',
	},
	showFilesTabTitle: {
		id: 'app.features-settings.show-files-tab.title',
		defaultMessage: 'Show Files tab in instances',
	},
	showScreenshotsTabTitle: {
		id: 'app.features-settings.show-screenshots-tab.title',
		defaultMessage: 'Show Screenshots tab in instances',
	},
	sidebarTitle: {
		id: 'app.features-settings.sidebar.title',
		defaultMessage: 'Sidebar',
	},
	showAllScreenshotsTitle: {
		id: 'app.features-settings.show-all-screenshots.title',
		defaultMessage: 'Show all screenshots in sidebar',
	},
	showAllScreenshotsDescription: {
		id: 'app.features-settings.show-all-screenshots.description',
		defaultMessage:
			'Show a button in the left sidebar to view screenshots from all your instances.',
	},
	showSkinSelectorTitle: {
		id: 'app.features-settings.show-skin-selector.title',
		defaultMessage: 'Show skin selector in sidebar',
	},
	showSkinSelectorDescription: {
		id: 'app.features-settings.show-skin-selector.description',
		defaultMessage: 'Show a button in the left sidebar to open the skin selector.',
	},
	quickInstancesTitle: {
		id: 'app.features-settings.quick-instances.title',
		defaultMessage: 'Quick instances in sidebar',
	},
	quickInstancesDescription: {
		id: 'app.features-settings.quick-instances.description',
		defaultMessage:
			'Choose the maximum number of recent instances shown in the left sidebar. You can also adjust this by dragging the divider.',
	},
	playPageTitle: {
		id: 'app.features-settings.play-page.title',
		defaultMessage: 'Play page',
	},
	showJumpInTitle: {
		id: 'app.features-settings.show-jump-in.title',
		defaultMessage: 'Show Jump in section',
	},
	showJumpInDescription: {
		id: 'app.features-settings.show-jump-in.description',
		defaultMessage: 'Show recently played worlds and instances at the top of the Play page.',
	},
})

type FeaturesSettingsState = {
	showFilesTab: boolean
	showWorldsTab: boolean
	showScreenshotsTab: boolean
	showAllScreenshots: boolean
	showSkinSelector: boolean
	quickInstanceCount: number
	showJumpIn: boolean
}

const settingsQuery = useQuery(appSettingsQueryOptions())
const globalOptionsQuery = useQuery(globalSyncedOptionsQueryOptions())
await Promise.all([settingsQuery.suspense(), globalOptionsQuery.suspense()])

function getFeaturesSettingsState(
	settings: AppSettings,
	globalSyncedOptions: GlobalSyncedOptions,
): FeaturesSettingsState {
	return {
		showFilesTab: settings.show_files_tab_in_instances,
		showWorldsTab: settings.show_worlds_tab_in_instances,
		showScreenshotsTab: settings.show_screenshots_tab_in_instances,
		showAllScreenshots: globalSyncedOptions.screenshots,
		showSkinSelector: settings.show_skin_selector_in_sidebar,
		quickInstanceCount: quickInstances.limit.value ?? QUICK_INSTANCE_LIMIT_MAX,
		showJumpIn: settings.show_jump_in,
	}
}

const settingsMutation = useMutation({
	mutationKey: appSettingsKeys.update,
	scope: { id: 'app-settings' },
	mutationFn: async ({
		value,
		updateQuickInstanceCount,
	}: {
		value: FeaturesSettingsState
		updateQuickInstanceCount: boolean
	}) => {

		const latestSettings = await get()
		const nextSettings: AppSettings = {
			...latestSettings,
			show_files_tab_in_instances: value.showFilesTab,
			show_worlds_tab_in_instances: value.showWorldsTab,
			show_screenshots_tab_in_instances: value.showScreenshotsTab,
			show_skin_selector_in_sidebar: value.showSkinSelector,
			show_jump_in: value.showJumpIn,
		}

		const screenshotsChanged =
			value.showAllScreenshots !== globalOptionsQuery.data.value!.screenshots
		const [, updatedGlobalSyncedOptions] = await Promise.all([
			set(nextSettings),
			screenshotsChanged
				? set_global_synced_option('screenshots', value.showAllScreenshots)
				: Promise.resolve(globalOptionsQuery.data.value!),
		])
		queryClient.setQueryData(appSettingsKeys.all, nextSettings)
		queryClient.setQueryData(syncedOptionsKeys.global, updatedGlobalSyncedOptions)
		if (screenshotsChanged) {
			await queryClient.invalidateQueries({ queryKey: screenshotKeys.all })
		}
		appSettings.showFilesTabInInstances = value.showFilesTab
		appSettings.showWorldsTabInInstances = value.showWorldsTab
		appSettings.showScreenshotsTabInInstances = value.showScreenshotsTab
		appSettings.showSkinSelectorInSidebar = value.showSkinSelector
		appSettings.showJumpIn = value.showJumpIn

		if (updateQuickInstanceCount) {
			quickInstances.setLimit(value.quickInstanceCount)
		}
	},
	onMutate: () =>
		Promise.all([
			queryClient.cancelQueries({ queryKey: appSettingsKeys.all }),
			queryClient.cancelQueries({ queryKey: syncedOptionsKeys.global }),
		]),
	onError: handleError,
	onSettled: () =>
		Promise.all([
			queryClient.invalidateQueries({ queryKey: appSettingsKeys.all }),
			queryClient.invalidateQueries({ queryKey: syncedOptionsKeys.global }),
		]),
})

const { saved, current, changes, saving, hasChanges, reset, save } = useSavable<FeaturesSettingsState>(
	() => getFeaturesSettingsState(settingsQuery.data.value!, globalOptionsQuery.data.value!),
	(changedValues) => {
		// 返回 void，避免类型层面的 Promise<unknown> 与 Promise<void> 冲突
		settingsMutation
			.mutateAsync({
				value: { ...current.value },
				updateQuickInstanceCount: changedValues.quickInstanceCount !== undefined,
			})
			.catch(() => {})
	},
)

async function saveFeaturesSettings(): Promise<void> {
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
		save: saveFeaturesSettings,
	})
})

onBeforeUnmount(() => {
	settingsModal.registerUnsavedChangesController(null)
})
</script>

<template>
	<section class="mt-0">
		<h2 class="m-0 text-xl font-semibold text-contrast">
			{{ formatMessage(messages.instancePagesTitle) }} <!-- 显示：实例 -->
		</h2>
		<div class="mt-4 flex flex-col gap-6">
			<div class="flex items-center justify-between gap-4">
				<div>
					<h3 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.showWorldsTabTitle) }} <!-- 显示：在实例中显示世界栏 -->
					</h3>
				</div>
				<Toggle
					id="show-worlds-tab-in-instances"
					v-model="current.showWorldsTab"
					:aria-label="formatMessage(messages.showWorldsTabTitle)"
				/>
			</div>

			<div class="flex items-center justify-between gap-4">
				<div>
					<h3 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.showFilesTabTitle) }} <!-- 显示：在实例中显示文件栏 -->
					</h3>
				</div>
				<Toggle
					id="show-files-tab-in-instances"
					v-model="current.showFilesTab"
					:aria-label="formatMessage(messages.showFilesTabTitle)"
				/>
			</div>

			<div class="flex items-center justify-between gap-4">
				<div>
					<h3 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.showScreenshotsTabTitle) }} <!-- 显示：在实例中显示截图栏 -->
					</h3>
				</div>
				<Toggle
					id="show-screenshots-tab-in-instances"
					v-model="current.showScreenshotsTab"
					:aria-label="formatMessage(messages.showScreenshotsTabTitle)"
				/>
			</div>
		</div>
	</section>

	<section class="mt-8 border-0 border-t border-solid border-surface-4 pt-6">
		<h2 class="m-0 text-xl font-semibold text-contrast">
			{{ formatMessage(messages.sidebarTitle) }} <!-- 显示：侧边栏 -->
		</h2>
		<div class="mt-4 flex flex-col gap-6">
			<div class="flex items-center justify-between gap-4">
				<div>
					<h3 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.showAllScreenshotsTitle) }} <!-- 显示：在侧边栏显示所有截图 -->
					</h3>
					<p class="m-0 mt-1">
						{{ formatMessage(messages.showAllScreenshotsDescription) }} <!-- 显示：在左侧边栏显示一个按钮来查看所有实例的截图。 -->
					</p>
				</div>
				<Toggle
					id="show-all-screenshots"
					v-model="current.showAllScreenshots"
					:aria-label="formatMessage(messages.showAllScreenshotsTitle)"
				/>
			</div>

			<div class="flex items-center justify-between gap-4">
				<div>
					<h3 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.showSkinSelectorTitle) }} <!-- 显示：在侧边栏显示皮肤选择器 -->
					</h3>
					<p class="m-0 mt-1">{{ formatMessage(messages.showSkinSelectorDescription) }} <!-- 显示：在左侧边栏显示一个按钮以打开皮肤选择器。 --></p>
				</div>
				<Toggle
					id="show-skin-selector-in-sidebar"
					v-model="current.showSkinSelector"
					:aria-label="formatMessage(messages.showSkinSelectorTitle)"
				/>
			</div>

			<div class="flex flex-col gap-2.5">
				<h3 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.quickInstancesTitle) }} <!-- 显示：侧边栏中的快速实例 -->
				</h3>
				<Slider
					id="quick-instances-in-sidebar"
					v-model="current.quickInstanceCount"
					:aria-label="formatMessage(messages.quickInstancesTitle)"
					:min="0"
					:max="QUICK_INSTANCE_LIMIT_MAX"
					:step="1"
				/>
				<p class="m-0 leading-tight">
					{{ formatMessage(messages.quickInstancesDescription) }} <!-- 显示：选择左侧边栏显示的最近实例的最大数量。您也可以通过拖动分隔符来调整它。 -->
				</p>
			</div>
		</div>
	</section>

	<section class="mt-8 border-0 border-t border-solid border-surface-4 pt-6">
		<h2 class="m-0 text-xl font-semibold text-contrast">
			{{ formatMessage(messages.playPageTitle) }} <!-- 显示：游玩界面 -->
		</h2>
		<div class="mt-4 flex items-center justify-between gap-4">
			<div>
				<h3 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.showJumpInTitle) }} <!-- 显示：在节中显示跳转 -->
				</h3>
				<p class="m-0 mt-1">{{ formatMessage(messages.showJumpInDescription) }} <!-- 显示：在游戏页面顶部显示最近播放的世界和实例。 --></p>
			</div>
			<Toggle
				id="show-jump-in-section"
				v-model="current.showJumpIn"
				:aria-label="formatMessage(messages.showJumpInTitle)"
			/>
		</div>
	</section>
</template>

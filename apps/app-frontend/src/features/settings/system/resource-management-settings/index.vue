<script setup>
import { BoxIcon, FolderSearchIcon } from '@modrinth/assets'
import { Combobox, defineMessages, IconButton, Input, Slider, useVIntl } from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'

import { setPref } from '@/composables/use-prefs'

import { get, set } from '@/helpers/settings.ts'

import ContentStorageSettings from './content-storage.vue'

const { formatMessage } = useVIntl()
const settings = ref(await get())

// Minecraft 下载源
const DOWNLOAD_SOURCE_KEY = 'aircore-download-source'
const downloadSourceOptions = [
	{ value: 'official' },
	{ value: 'bmclapi' },
]
const downloadSource = ref(localStorage.getItem(DOWNLOAD_SOURCE_KEY) ?? 'official')
const downloadSourceLabels = computed(() => ({
	official: formatMessage(messages.downloadSourceOfficial),
	bmclapi: formatMessage(messages.downloadSourceMirror),
}))
const downloadSourceChoices = computed(() =>
	downloadSourceOptions.map((option) => ({
		value: option.value,
		label: downloadSourceLabels.value[option.value] ?? option.value,
	})),
)
watch(downloadSource, async (value) => {
	setPref(DOWNLOAD_SOURCE_KEY, value)
	await invoke('plugin:utils|set_download_source', { source: value }).catch(() => {})
})

const messages = defineMessages({
	downloadSourceTitle: { id: 'app.downloads.sourceTitle', defaultMessage: 'Minecraft download source' },
	downloadSourceDescription: {
		id: 'app.downloads.sourceDescription',
		defaultMessage:
			'The official source is slow from mainland China. Mirror downloads version manifests, libraries and assets; modpacks and mods are not affected.',
	},
	downloadSourceOfficial: {
		id: 'app.downloads.sourceOfficial',
		defaultMessage: 'Official (Mojang, slow in China)',
	},
	downloadSourceMirror: {
		id: 'app.downloads.sourceMirror',
		defaultMessage: 'Mirror (BMCLAPI)',
	},
	concurrentHint: {
		id: 'app.downloads.concurrentHint',
		defaultMessage:
			'Auto-tuned between 8 and 96 based on bandwidth; this is the initial value.',
	},
	appDirectoryTitle: {
		id: 'app.settings.resource-management.app-directory.title',
		defaultMessage: 'App directory',
	},
	appDirectoryDescription: {
		id: 'app.settings.resource-management.app-directory.description',
		defaultMessage:
			'Where AirCore stores instances and other files. Changes take effect after restarting the app.',
	},
	selectAppDirectory: {
		id: 'app.settings.resource-management.app-directory.select',
		defaultMessage: 'Select a new app directory',
	},
	browseAppDirectory: {
		id: 'app.settings.resource-management.app-directory.browse',
		defaultMessage: 'Browse for an app directory',
	},
	maximumConcurrentDownloadsTitle: {
		id: 'app.settings.resource-management.maximum-concurrent-downloads.title',
		defaultMessage: 'Maximum concurrent downloads',
	},
	maximumConcurrentDownloadsDescription: {
		id: 'app.settings.resource-management.maximum-concurrent-downloads.description',
		defaultMessage:
			'Number of files the app can download at once. Lower this if downloads are unreliable on your connection. Requires an app restart.',
	},
	maximumConcurrentWritesTitle: {
		id: 'app.settings.resource-management.maximum-concurrent-writes.title',
		defaultMessage: 'Maximum concurrent writes',
	},
	maximumConcurrentWritesDescription: {
		id: 'app.settings.resource-management.maximum-concurrent-writes.description',
		defaultMessage:
			'Number of files the app can write to disk at once. Lower this if you frequently encounter I/O errors. Requires an app restart.',
	},
})

watch(
	settings,
	async () => {
		const setSettings = JSON.parse(JSON.stringify(settings.value))

		if (!setSettings.custom_dir) {
			setSettings.custom_dir = null
		}

		await set(setSettings)
	},
	{ deep: true },
)

async function findLauncherDir() {
	const newDir = await open({
		multiple: false,
		directory: true,
		title: formatMessage(messages.selectAppDirectory),
	})

	if (newDir) {
		settings.value.custom_dir = newDir
	}
}
</script>

<template>
	<section class="mb-6 border-0 border-b border-solid border-divider pb-6">
		<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.downloadSourceTitle) }}</h2>
		<p class="m-0 mt-1 text-secondary">{{ formatMessage(messages.downloadSourceDescription) }}</p>
		<div class="mt-3 w-[16rem]">
			<Combobox
				v-model="downloadSource"
				:options="downloadSourceChoices"
				trigger-type="base"
			/>
		</div>
	</section>

	<div class="flex flex-col gap-6">
		<ContentStorageSettings />
		<div class="flex flex-col gap-2.5">
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.appDirectoryTitle) }} <!-- 显示：应用目录 -->
			</h2>
			<Input
				id="appDir"
				v-model="settings.custom_dir"
				:icon="BoxIcon"
				type="text"
				wrapper-class="w-full"
			>
				<template #right>
					<IconButton
						v-tooltip="formatMessage(messages.browseAppDirectory)"
						:label="formatMessage(messages.browseAppDirectory)"
						class="ml-1.5"
						@click="findLauncherDir"
					>
						<FolderSearchIcon aria-hidden="true" />
					</IconButton>
				</template>
			</Input>
			<p class="m-0 leading-tight text-secondary">
				{{ formatMessage(messages.appDirectoryDescription) }} <!-- 显示：AirCore存储实例和其他文件的位置。更改会在重启程序后生效。 -->
			</p>
		</div>

		<div class="flex flex-col gap-2.5">
			<h2 class="m-0 text-lg font-semibold text-contrast mt-4">
				{{ formatMessage(messages.maximumConcurrentDownloadsTitle) }} <!-- 显示：最大并发下载量 -->
			</h2>
			<Slider
				id="max-downloads"
				v-model="settings.max_concurrent_downloads"
				:min="1"
				:max="32"
				:step="1"
			/>
			<p class="m-0 leading-tight text-secondary">
				{{ formatMessage(messages.maximumConcurrentDownloadsDescription) }} {{ formatMessage(messages.concurrentHint) }}
			</p>
		</div>

		<div class="flex flex-col gap-2.5">
			<h2 class="mt-0 m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.maximumConcurrentWritesTitle) }} <!-- 显示：最大并发写入数 -->
			</h2>
			<Slider
				id="max-writes"
				v-model="settings.max_concurrent_writes"
				:min="1"
				:max="50"
				:step="1"
			/>
			<p class="m-0 leading-tight text-secondary">
				{{ formatMessage(messages.maximumConcurrentWritesDescription) }} <!-- 显示：应用可同时写入磁盘的文件数。如果经常出现 I/O 错误，请降低该值。需要重启应用程序。 -->
			</p>
		</div>
	</div>
</template>

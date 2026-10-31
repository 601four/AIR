<script setup>
import { BoxIcon, FolderSearchIcon } from '@modrinth/assets'
import {
	Button,
	Combobox,
	defineMessages,
	IconButton,
	injectNotificationManager,
	Input,
	Slider,
	useVIntl,
} from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'


import { get, set } from '@/helpers/settings.ts'

import ContentStorageSettings from './content-storage.vue'

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const settings = ref(await get())

// Minecraft 下载源
const downloadSourceOptions = [
	{ value: 'official' },
	{ value: 'bmclapi' },
]
const downloadSource = ref(settings.value.download_source ?? 'official')
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
	settings.value.download_source = value
	await set(settings.value).catch(() => {})
	await invoke('plugin:utils|set_download_source', { source: value }).catch(() => {})
})

// Modrinth 资源源
const modrinthMirror = ref(settings.value.modrinth_mirror === true)
const modrinthSourceChoices = computed(() => [
	{ value: false, label: formatMessage(messages.modrinthSourceOfficial) },
	{ value: true, label: formatMessage(messages.modrinthSourceMirror) },
])
watch(modrinthMirror, async (value) => {
	settings.value.modrinth_mirror = value
	await set(settings.value).catch(() => {})
	await invoke('plugin:utils|set_modrinth_mirror', { enabled: value }).catch(() => {})
})

// 加载器下载源（Fabric / Quilt / Forge / NeoForge）
const loaderMode = ref(settings.value.loader_mode ?? 'cn')
const loaderMirror = ref(settings.value.loader_prefix ?? '')
const loaderModeChoices = computed(() => [
	{ value: 'official', label: formatMessage(messages.loaderSourceOfficial) },
	{ value: 'cn', label: formatMessage(messages.loaderSourceCn) },
	{ value: 'custom', label: formatMessage(messages.loaderSourceCustom) },
])
watch([loaderMode, loaderMirror], async () => {
	const prefix = loaderMode.value === 'custom' ? loaderMirror.value.trim() : ''
	settings.value.loader_mode = loaderMode.value
	settings.value.loader_prefix = prefix || null
	await set(settings.value).catch(() => {})
	await invoke('plugin:utils|set_loader_source', {
		mode: loaderMode.value,
		prefix: prefix || null,
	}).catch(() => {})
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
	modrinthSourceTitle: {
		id: 'app.downloads.modrinthSourceTitle',
		defaultMessage: 'Modrinth download source',
	},
	modrinthSourceDescription: {
		id: 'app.downloads.modrinthSourceDescription',
		defaultMessage: 'Where Modrinth files are downloaded from. Project lists and search always use the official source.',
	},
	modrinthSourceOfficial: {
		id: 'app.downloads.modrinthSourceOfficial',
		defaultMessage: 'Official',
	},
	modrinthSourceMirror: {
		id: 'app.downloads.modrinthSourceMirror',
		defaultMessage: 'MCIM mirror',
	},
	loaderSourceTitle: {
		id: 'app.downloads.loaderSourceTitle',
		defaultMessage: 'Mod loader download source',
	},
	loaderSourceDescription: {
		id: 'app.downloads.loaderSourceDescription',
		defaultMessage:
			'Where Fabric / Quilt / Forge / NeoForge libraries are downloaded from. Use a mirror prefix that passes Maven paths through, e.g. https://mirror.example.com/maven',
	},
	loaderSourceOfficial: { id: 'app.downloads.loaderSourceOfficial', defaultMessage: 'Official (direct)' },
	loaderSourceCn: {
		id: 'app.downloads.loaderSourceCn',
		defaultMessage: 'China acceleration (Maven Central via Aliyun)',
	},
	loaderSourceCustom: { id: 'app.downloads.loaderSourceCustom', defaultMessage: 'Custom mirror' },
	loaderSourcePlaceholder: {
		id: 'app.downloads.loaderSourcePlaceholder',
		defaultMessage: 'https://mirror.example.com/maven',
	},
	dataDirTitle: { id: 'app.dataDir.title', defaultMessage: 'Account data directory' },
	dataDirDescription: {
		id: 'app.dataDir.description',
		defaultMessage:
			'Where the database, plugins and logs are stored. Changing it copies the existing data to the new folder and takes effect after a restart.',
	},
	dataDirSetFailed: {
		id: 'app.dataDir.setFailed',
		defaultMessage: 'Failed to change the data directory',
	},
	dataDirBrowse: { id: 'app.dataDir.browse', defaultMessage: 'Choose folder' },
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

// 账号数据目录（app.db / 插件 / 日志）
const dataDirInfo = ref({ default: '', configured: null, effective: '' })
const dataDirDisplay = ref('')
let dataDirSaveTimer

async function loadDataDir() {
	try {
		dataDirInfo.value = await invoke('plugin:utils|get_data_dir')
		dataDirDisplay.value = dataDirInfo.value.effective
	} catch (error) {
		console.warn('[data-dir] load failed', error)
	}
}

// 输入后自动保存（和「应用目录」一样，改完重启生效）
watch(dataDirDisplay, (value) => {
	if (dataDirSaveTimer) clearTimeout(dataDirSaveTimer)
	const trimmed = String(value ?? '').trim()
	if (!trimmed || trimmed === dataDirInfo.value.effective) return
	if (!/^[A-Za-z]:[\\/]|^\\\\/.test(trimmed)) return
	dataDirSaveTimer = setTimeout(async () => {
		try {
			await invoke('plugin:utils|set_data_dir', { path: trimmed })
			dataDirInfo.value = { ...dataDirInfo.value, configured: trimmed, effective: trimmed }
		} catch (error) {
			console.error('[data-dir] set failed', error)
			addNotification({
				type: 'error',
				title: formatMessage(messages.dataDirSetFailed),
				text: String(error),
			})
		}
	}, 600)
})

async function chooseDataDir() {
	const picked = await open({
		multiple: false,
		directory: true,
		title: formatMessage(messages.dataDirBrowse),
	})
	if (!picked) return
	dataDirDisplay.value = picked
}

loadDataDir()

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

	<section class="mb-6 border-0 border-b border-solid border-divider pb-6">
		<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.modrinthSourceTitle) }}</h2>
		<p class="m-0 mt-1 text-secondary">{{ formatMessage(messages.modrinthSourceDescription) }}</p>
		<div class="mt-3 w-[16rem]">
			<Combobox v-model="modrinthMirror" :options="modrinthSourceChoices" trigger-type="base" />
		</div>
	</section>

	<section class="mb-6 border-0 border-b border-solid border-divider pb-6">
		<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.loaderSourceTitle) }}</h2>
		<p class="m-0 mt-1 text-secondary">{{ formatMessage(messages.loaderSourceDescription) }}</p>
		<div class="mt-3 flex flex-col gap-2">
			<div class="w-[16rem]">
				<Combobox v-model="loaderMode" :options="loaderModeChoices" trigger-type="base" />
			</div>
			<Input
				v-if="loaderMode === 'custom'"
				id="loaderMirror"
				v-model="loaderMirror"
				type="text"
				:placeholder="formatMessage(messages.loaderSourcePlaceholder)"
				wrapper-class="w-full"
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
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.dataDirTitle) }}
			</h2>
			<Input id="dataDir" v-model="dataDirDisplay" :icon="BoxIcon" type="text" wrapper-class="w-full">
				<template #right>
					<IconButton
						v-tooltip="formatMessage(messages.dataDirBrowse)"
						:label="formatMessage(messages.dataDirBrowse)"
						class="ml-1.5"
						@click="chooseDataDir"
					>
						<FolderSearchIcon aria-hidden="true" />
					</IconButton>
				</template>
			</Input>
			<p class="m-0 leading-tight text-secondary">
				{{ formatMessage(messages.dataDirDescription) }}
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

<script setup lang="ts">
import { defineMessages, Toggle, useVIntl } from '@modrinth/ui'
import { ref, watch } from 'vue'

import { optInAnalytics, optOutAnalytics } from '@/helpers/analytics'
import { get, set } from '@/helpers/settings.ts'

const { formatMessage } = useVIntl()
const settings = ref(await get())

const messages = defineMessages({
	telemetryTitle: {
		id: 'app.settings.privacy.telemetry.title',
		defaultMessage: 'Telemetry',
	},
	telemetryDescription: {
		id: 'app.settings.privacy.telemetry.description',
		defaultMessage:
			'AirCore collects anonymized analytics and usage data to improve our user experience and customize your experience. By disabling this option, you opt out and your data will no longer be collected.',
	},
	discordRichPresenceTitle: {
		id: 'app.settings.privacy.discord-rich-presence.title',
		defaultMessage: 'Discord Rich Presence',
	},
	discordRichPresenceDescription: {
		id: 'app.settings.privacy.discord-rich-presence.description',
		defaultMessage:
			'Show AirCore as your current activity on Discord. This does not affect Rich Presence added to instances by mods. Requires an app restart.',
	},
})

watch(
	settings,
	async () => {
		if (settings.value.telemetry) {
			optInAnalytics()
		} else {
			optOutAnalytics()
		}

		await set(settings.value)
	},
	{ deep: true },
)
</script>

<template>
	<div class="mt-8 first:mt-0 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.telemetryTitle) }} <!-- 显示：遥测 -->
			</h2>
			<p class="m-0 mt-1">
				{{ formatMessage(messages.telemetryDescription) }} <!-- 显示：AirCore 收集匿名分析和使用数据，以提升用户体验并为您提供个性化服务。若禁用此选项，即表示您选择退出，您的数据将不再被收集。 -->
			</p>
		</div>
		<Toggle id="opt-out-analytics" v-model="settings.telemetry" />
	</div>

	<div class="mt-4 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.discordRichPresenceTitle) }} <!-- 显示：Discord Rich Presence -->
			</h2>
			<p class="m-0 mt-1">
				{{ formatMessage(messages.discordRichPresenceDescription) }} <!-- 显示：在 Discord 上将 AirCore 显示为您当前的活动。这不会影响模组为游戏实例添加的 Rich Presence。需要重启应用生效。 -->
			</p>
		</div>
		<Toggle id="disable-discord-rpc" v-model="settings.discord_rpc" />
	</div>
</template>

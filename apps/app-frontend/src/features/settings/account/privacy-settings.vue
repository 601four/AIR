<script setup lang="ts">
import { Settings2Icon } from '@modrinth/assets'
import {
	Button,
	defineMessages,
	injectNotificationManager,
	injectPageContext,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { ref, watch } from 'vue'

import { open_ads_consent_preferences } from '@/helpers/ads.js'
import { optInAnalytics, optOutAnalytics } from '@/helpers/analytics'
import { get, set } from '@/helpers/settings.ts'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const { adConsentAvailable } = injectPageContext()
const settings = ref(await get())

const messages = defineMessages({
	adsConsentTitle: {
		id: 'app.ads-consent.title',
		defaultMessage: 'Your privacy and how ads support AirCore',
	},
	adsConsentIntro: {
		id: 'app.settings.privacy.ads-consent.intro',
		defaultMessage:
			'Ads make AirCore possible and fund creator payouts. Our partners may store or access cookies in the app to personalize ads and measure performance. You can opt out or manage your preferences below.',
	},
	adsConsentManage: {
		id: 'app.ads-consent.manage',
		defaultMessage: 'Manage preferences',
	},
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

async function manageAdsPreferences() {
	await open_ads_consent_preferences().catch(handleError)
}

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
	<div v-if="adConsentAvailable">
		<h2 class="m-0 text-lg font-semibold text-contrast">
			{{ formatMessage(messages.adsConsentTitle) }} <!-- 显示：您的隐私以及广告如何支持 AirCore -->
		</h2>
		<div class="mt-2 flex flex-col gap-2.5 items-start">
			<Button @click="manageAdsPreferences">
				<Settings2Icon aria-hidden="true" />
				{{ formatMessage(messages.adsConsentManage) }} <!-- 显示：管理偏好设置 -->
			</Button>
			<div>
				{{ formatMessage(messages.adsConsentIntro) }} <!-- 显示：广告使 AirCore 得以运营，并为创作者报酬提供资金支持。我们的合作伙伴可能会在应用中存储或访问 Cookie，以便实现广告个性化和效果衡量。您可以在下方选择退出或管理您的偏好设置。 -->
			</div>
		</div>
	</div>

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

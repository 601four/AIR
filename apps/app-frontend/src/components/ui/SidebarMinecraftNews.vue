<template>
	<div
		v-if="sidebarNewsEnabled && items.length > 0"
		class="border-0 border-b-[1px] border-[--brand-gradient-border] border-solid"
	>
		<div class="flex flex-col gap-2 p-4">
			<h3 class="m-0 text-base font-medium text-primary">
				{{ formatMessage(messages.title) }}
			</h3>
			<button
				v-for="item in items"
				:key="item.id"
				class="group flex w-full flex-col gap-2 rounded-lg border-0 bg-transparent p-2 text-left transition-colors hover:bg-surface-3"
				@click="openNews(item.link)"
			>
				<img
					v-if="item.image"
					:src="item.image"
					alt=""
					class="h-[88px] w-full rounded-md object-cover"
					loading="lazy"
				/>
				<span class="line-clamp-2 text-sm text-contrast group-hover:text-brand">
					{{ item.title }}
				</span>
				<span class="text-xs text-secondary">
					<template v-if="item.category">{{ item.category }}</template>
					<template v-if="item.category && item.date"> · </template>
					<template v-if="item.date">{{ item.date }}</template>
				</span>
			</button>
		</div>
	</div>
</template>

<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import { onMounted } from 'vue'

import { useMinecraftNews } from '@/composables/use-minecraft-news'
import { useSidebarNews } from '@/composables/use-sidebar-news'

const { formatMessage } = useVIntl()
const { sidebarNewsEnabled } = useSidebarNews()
const { items, loadMinecraftNews } = useMinecraftNews()

const messages = defineMessages({
	title: { id: 'app.sidebar.minecraft-news.title', defaultMessage: 'Minecraft news' },
})

onMounted(() => {
	void loadMinecraftNews()
})

async function openNews(link: string) {
	try {
		await openUrl(link)
	} catch (error) {
		console.warn('[minecraft-news] open failed', error)
		window.open(link, '_blank')
	}
}
</script>

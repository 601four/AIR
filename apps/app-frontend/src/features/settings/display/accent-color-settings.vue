<!-- 主题强调色：经典绿 / 天空蓝 / 樱花粉 / 自定义 -->
<template>
	<div class="flex flex-col gap-3 pb-6">
		<div>
			<h3 class="m-0 text-base font-semibold text-contrast">{{ formatMessage(messages.title) }}</h3>
			<p class="m-0 mt-1 text-secondary">{{ formatMessage(messages.description) }}</p>
		</div>

		<div class="flex flex-wrap items-center gap-2">
			<button
				v-for="option in accentOptions"
				:key="option.id"
				class="inline-flex items-center gap-2 whitespace-nowrap rounded-lg border border-solid px-3 py-2 text-sm transition-colors duration-150"
				:class="
					accentColor === option.color
						? 'border-brand bg-surface-4 text-contrast'
						: 'border-surface-4 bg-surface-3 text-secondary hover:bg-surface-4 hover:text-contrast'
				"
				@click="setAccent(option.color)"
			>
				<span
					class="h-4 w-4 shrink-0 rounded-full ring-2 ring-transparent transition-all duration-150"
					:class="{ 'ring-brand/40': accentColor === option.color }"
					:style="{ backgroundColor: option.color }"
				/>
				{{ option.label }}
				<CheckIcon
					class="h-3.5 w-3.5 shrink-0 text-brand transition-all duration-150"
					:class="accentColor === option.color ? 'scale-100 opacity-100' : 'scale-0 opacity-0'"
				/>
			</button>

			<div
				class="relative inline-flex cursor-pointer items-center gap-2 whitespace-nowrap rounded-lg border border-solid px-3 py-2 text-sm transition-colors duration-150"
				:class="
					isCustom
						? 'border-brand bg-surface-4 text-contrast'
						: 'border-surface-4 bg-surface-3 text-secondary hover:bg-surface-4 hover:text-contrast'
				"
				@click="openCustomPicker"
			>
				<span
					class="h-4 w-4 shrink-0 rounded-full ring-2 ring-transparent transition-all duration-150"
					:class="{ 'ring-brand/40': isCustom }"
					:style="{ backgroundColor: customColor }"
				/>
				<span>{{ formatMessage(messages.custom) }}</span>
				<CheckIcon
					class="h-3.5 w-3.5 shrink-0 text-brand transition-all duration-150"
					:class="isCustom ? 'scale-100 opacity-100' : 'scale-0 opacity-0'"
				/>
				<!-- 隐藏的 ColorPicker：点整个按钮时代为点它的触发器 -->
				<div ref="pickerAnchor" class="pointer-events-none absolute left-2 top-2 h-0 w-0 overflow-hidden opacity-0">
					<ColorPicker v-model="customColor" label="自定义强调色" size="sm" />
				</div>
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
import { CheckIcon } from '@modrinth/assets'
import { ColorPicker, defineMessages, useVIntl } from '@modrinth/ui'

import { computed, ref, watch } from 'vue'

import { ACCENT_OPTIONS, useAccentColor } from '@/composables/use-accent-color'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: { id: 'app.accent.title', defaultMessage: 'Accent color' },
	description: {
		id: 'app.accent.description',
		defaultMessage:
			"Changes the launcher's accent color (buttons, selections, loading bar, sidebar gradient). The dark / light / OLED theme itself stays the same.",
	},
	custom: { id: 'app.accent.custom', defaultMessage: 'Custom' },
	customLabel: { id: 'app.accent.customLabel', defaultMessage: 'Custom accent color' },
})

const { accentColor, isCustom, setAccent } = useAccentColor()

const ACCENT_LABELS = {
	green: { id: 'app.accent.preset.green', defaultMessage: 'Classic green' },
	blue: { id: 'app.accent.preset.blue', defaultMessage: 'Sky blue' },
	pink: { id: 'app.accent.preset.pink', defaultMessage: 'Cherry pink' },
}
const accentOptions = computed(() =>
	ACCENT_OPTIONS.map((option) => ({
		...option,
		label: ACCENT_LABELS[option.id] ? formatMessage(ACCENT_LABELS[option.id]) : option.name,
	})),
)

const customColor = ref(accentColor.value)
const pickerAnchor = ref<HTMLElement | null>(null)

function openCustomPicker() {
	const trigger = pickerAnchor.value?.querySelector('button')
	trigger?.click()
}

watch(customColor, (value) => {
	if (value) setAccent(value)
})
</script>

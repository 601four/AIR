<template>
	<div v-if="error" class="rounded-xl border border-solid border-surface-4 bg-surface-3 p-4">
		<p class="m-0 font-semibold text-contrast">插件面板出错</p>
		<p class="m-0 mt-1 text-sm text-secondary">{{ error }}</p>
		<Button class="mt-3" @click="retry">重试</Button>
	</div>
	<slot v-else />
</template>

<script setup lang="ts">
import { Button } from '@modrinth/ui'
import { onErrorCaptured, ref } from 'vue'

const error = ref<string | null>(null)
const props = defineProps<{ pluginId?: string }>()

onErrorCaptured((err) => {
	error.value = err instanceof Error ? err.message : String(err)
	console.warn(`[plugins] ${props.pluginId ?? ''} 渲染出错`, err)
	return false
})

function retry() {
	error.value = null
}
</script>

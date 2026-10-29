<template>
	<div v-if="error" class="rounded-xl border border-solid border-red-500/40 bg-red-500/10 p-4">
		<p class="m-0 font-semibold text-contrast">这个设置页面加载失败</p>
		<pre class="mt-2 whitespace-pre-wrap break-all text-sm text-secondary">{{ error }}</pre>
	</div>
	<Suspense v-else>
		<component :is="component" v-if="component" />
		<template #fallback>
			<p class="m-0 text-secondary">加载中…</p>
		</template>
	</Suspense>
</template>

<script setup lang="ts">
import { onErrorCaptured, ref, watch } from 'vue'

const props = defineProps<{ component?: unknown }>()
const error = ref<string | null>(null)

watch(
	() => props.component,
	() => {
		error.value = null
	},
)

onErrorCaptured((err) => {
	error.value = err instanceof Error ? err.stack || err.message : String(err)
	console.error('[settings-tab] 渲染失败', err)
	return false
})
</script>

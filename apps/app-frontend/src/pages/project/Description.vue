<template>
	<Card>
		<div class="mb-3 flex items-center justify-end gap-2">
			<IconButton
				:label="translated ? formatMessage(messages.showOriginal) : formatMessage(messages.translate)"
				type="quiet"
				size="sm"
				:loading="translating"
				@click="toggleTranslation"
			>
				<EyeOffIcon v-if="translated" />
				<LanguagesIcon v-else />
			</IconButton>
		</div>
		<ProjectPageDescription :description="displayBody" />
	</Card>
</template>

<script setup>
import { EyeOffIcon, LanguagesIcon } from '@modrinth/assets'
import {
	Card,
	defineMessages,
	IconButton,
	ProjectPageDescription,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'

const props = defineProps({
	project: {
		type: Object,
		default: () => ({}),
	},
})

const { formatMessage, locale } = useVIntl()
const { addNotification } = injectNotificationManager()

const messages = defineMessages({
	translate: { id: 'app.project.translate', defaultMessage: 'Translate' },
	showOriginal: { id: 'app.project.showOriginal', defaultMessage: 'Show original' },
	translating: { id: 'app.project.translating', defaultMessage: 'Translating…' },
	translateFailed: { id: 'app.project.translateFailed', defaultMessage: 'Translation failed' },
})

const translated = ref(false)
const translating = ref(false)
const translatedBody = ref('')

/** 目标语言 = 当前界面语言（中文 / 英文） */
const targetLocale = computed(() => (locale.value === 'zh-CN' ? 'zh-CN' : 'en-US'))

const displayBody = computed(() => {
	if (translated.value && translatedBody.value) return translatedBody.value
	return props.project?.body ?? ''
})

function cacheKey() {
	const id = props.project?.id ?? 'unknown'
	const len = (props.project?.body ?? '').length
	return `aircore-desc-translation:${id}:${len}:${targetLocale.value}`
}

/** 把长文本按段落切成小块，避免单次请求超限 */
function splitChunks(text, max = 1500) {
	const paragraphs = text.split(/\n{2,}/)
	const chunks = []
	let current = ''
	for (const paragraph of paragraphs) {
		if ((current + paragraph).length > max) {
			if (current) chunks.push(current)
			current = paragraph
		} else {
			current = current ? current + '\n\n' + paragraph : paragraph
		}
	}
	if (current) chunks.push(current)
	return chunks
}

async function translateBody(body) {
	const cached = localStorage.getItem(cacheKey())
	if (cached) return cached

	const chunks = splitChunks(body)
	const results = []
	for (const chunk of chunks) {
		const result = await invoke('plugin:utils|translate_text', {
			text: chunk,
			target: targetLocale.value,
		})
		results.push(result)
	}
	const joined = results.join('\n\n')
	localStorage.setItem(cacheKey(), joined)
	return joined
}

async function toggleTranslation() {
	if (translated.value) {
		translated.value = false
		return
	}
	const body = props.project?.body ?? ''
	if (!body.trim()) return
	translating.value = true
	try {
		translatedBody.value = await translateBody(body)
		translated.value = true
	} catch (error) {
		console.error('[translate] failed', error)
		addNotification({
			type: 'error',
			title: formatMessage(messages.translateFailed),
			text: String(error),
		})
	} finally {
		translating.value = false
	}
}
</script>

<script>
export default {
	name: 'Description',
}
</script>

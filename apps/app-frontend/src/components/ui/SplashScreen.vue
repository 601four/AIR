<template>
	<Transition name="splash-fade" @after-leave="onAfterLeave">
		<div v-if="!doneLoading" class="splash-screen" :class="`${theme.active}-mode`">
			<div class="app-logo-wrapper" data-tauri-drag-region>
				<span class="app-logo-text">AirCore</span>
				<ProgressBar class="loading-bar" :progress="Math.min(loadingProgress, 100)" />
				<span v-if="message">{{ message }}</span>
			</div>
			<div class="gradient-bg" data-tauri-drag-region></div>
			<div class="cube-bg"></div>
			<div class="base-bg"></div>
		</div>
	</Transition>
</template>

<script setup>
import { injectLoadingState } from '@modrinth/ui'
import { onMounted, ref, watch } from 'vue'

import ProgressBar from '@/components/ui/ProgressBar.vue'
import { useAppEvent } from '@/composables/use-app-event'
import { useTheme } from '@/composables/use-theme.ts'
import { debugStartup } from '@/helpers/startup-debug'

const theme = useTheme()

const doneLoading = ref(false)
const loadingProgress = ref(0)
const message = ref()

const MIN_DISPLAY_MS = 500
const mountedAt = Date.now()

const loading = injectLoadingState()
onMounted(() => debugStartup('Splash mounted'))

function onAfterLeave() {
	debugStartup('Splash fade completed', { displayedMs: Date.now() - mountedAt })
	loading.setEnabled(true)
}

watch(
	[loading.barEnabled, loading.pending],
	([barEnabled, pending]) => {
		debugStartup('Splash loading state changed', { barEnabled, pending })
		if (barEnabled) {
			return
		}

		if (pending) {
			loadingProgress.value = 0
			fakeLoadingIncrease()
			return
		}

		const elapsed = Date.now() - mountedAt
		const delay = Math.max(0, MIN_DISPLAY_MS - elapsed)
		debugStartup('Splash dismissal scheduled', { delayMs: delay, displayedMs: elapsed })

		setTimeout(() => {
			if (loading.pending.value) {
				debugStartup('Splash dismissal deferred: new loading work')
				return
			}
			doneLoading.value = true
			debugStartup('Splash fade started', { displayedMs: Date.now() - mountedAt })
		}, delay)
	},
	{ immediate: true },
)

function fakeLoadingIncrease() {
	if (loadingProgress.value < 95) {
		setTimeout(() => {
			loadingProgress.value += 2
			fakeLoadingIncrease()
		}, 5)
	}
}

useAppEvent('loading', (e) => {
	if (e.event.type === 'directory_move') {
		loadingProgress.value = 100 * (e.fraction ?? 1)
		message.value = 'Updating app directory...'
	}
})
</script>


.app-logo-text {
	font-size: 52px;
	font-weight: 800;
	letter-spacing: -0.02em;
	color: var(--color-contrast);
	user-select: none;
}
<style scoped lang="scss">
.splash-screen {
	position: fixed;
	inset: 0;
	z-index: 10000;

	--splash-cube-image: url('@/assets/loading/cube-white.png');

	&.light-mode {
		--splash-cube-image: url('@/assets/loading/cube-white-light.png');
	}
}

.splash-fade-leave-active {
	transition: opacity 0.3s ease-in-out;
}

.splash-fade-leave-to {
	opacity: 0;
}

.app-logo-wrapper {
	position: absolute;
	height: 100vh;
	width: 100%;

	display: flex;
	flex-direction: column;
	justify-content: center;
	align-items: center;

	gap: 1rem;
	color: var(--color-contrast);

	z-index: 9998;
}

.app-logo {
	height: 2.25rem;
	width: fit-content;
}

.loading-bar {
	max-width: 20rem;
}

.gradient-bg {
	position: absolute;
	height: 100vh;
	width: 100vw;
	background:
		linear-gradient(
			180deg,
			color-mix(in srgb, var(--color-brand) 22%, transparent) 0%,
			color-mix(in srgb, var(--color-brand) 8%, transparent) 55%,
			color-mix(in srgb, var(--color-brand) 18%, transparent) 100%
		),
		linear-gradient(
			0deg,
			color-mix(in srgb, var(--color-brand) 10%, transparent),
			color-mix(in srgb, var(--color-brand) 10%, transparent)
		);
	z-index: 9997;
}

.cube-bg {
	position: absolute;

	left: 50%;
	top: 50%;
	transform: translate(-50%, -50%);

	width: 180vw;
	height: 180vh;
	background-color: var(--color-bg);

	z-index: 9996;

	&::after {
		content: '';
		position: absolute;
		inset: 0;
		/* 纯白图作为遮罩，颜色来自强调色 */
		background-color: var(--color-brand);
		-webkit-mask: var(--splash-cube-image) center / contain no-repeat;
		mask: var(--splash-cube-image) center / contain no-repeat;
		opacity: var(--splash-cube-opacity, 0.28);
	}
}

.base-bg {
	position: absolute;
	top: 0;
	left: 0;
	width: 100%;
	height: 100%;
	background: var(--color-bg);
	z-index: 9995;
}
</style>

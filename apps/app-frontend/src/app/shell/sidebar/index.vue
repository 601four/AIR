<script setup lang="ts">
import { defineMessages, useScrollIndicator, useVIntl } from '@modrinth/ui'
import PluginErrorBoundary from '@/components/ui/PluginErrorBoundary.vue'
import { pluginRegistry } from '@/plugin-host'
import { dispatchSandboxEvent, sandboxTrees } from '@/plugin-host'
import { renderUiTree, type UiNode } from '@/plugins/ui-renderer'
import { OverlayScrollbars } from 'overlayscrollbars'
import { computed, onMounted, onScopeDispose, ref, watch } from 'vue'

import { useTheme } from '@/composables/use-theme'
import AccountsCard from '@/features/minecraft-accounts/accounts-card.vue'
import type { MinecraftAccountsActions } from '@/features/minecraft-accounts/context'
import OnboardingChecklist from '@/features/onboarding/checklist.vue'
import type { ModrinthCredentials } from '@/helpers/mr_auth'
import { injectOnboardingChecklist } from '@/providers/onboarding-checklist'


const props = defineProps<{
	credentials: ModrinthCredentials | null | undefined
	hasPlus: boolean
	showAd: boolean
	visible: boolean
	requestSignIn: () => void
}>()

const emit = defineEmits<{
	'create-instance': []
	'login-modrinth': []
}>()

const { formatMessage } = useVIntl()
const appTheme = useTheme()
const { hasLoggedIntoMinecraft, hasLoggedIntoModrinth, showChecklist } = injectOnboardingChecklist()
const accounts = ref<InstanceType<typeof AccountsCard> | null>(null)
const scrollContainer = ref<HTMLElement | null>(null)
const scrollViewport = ref<HTMLElement | null>(null)
const upgradeHeight = ref(0)
const promotionPaddingStyle = computed(() =>
	props.showAd ? { paddingBottom: `calc(1rem + ${upgradeHeight.value}px)` } : undefined,
)
const { showTopFade, showBottomFade, checkScrollState } = useScrollIndicator(scrollViewport)

let removeScrollbarListener: (() => void) | undefined

onMounted(() => {
	if (!scrollContainer.value) return
	const scrollbars = OverlayScrollbars(scrollContainer.value)
	if (!scrollbars) return
	scrollViewport.value = scrollbars.elements().viewport
	removeScrollbarListener = scrollbars.on('updated', checkScrollState)
})

onScopeDispose(() => removeScrollbarListener?.())

const minecraftAccounts = computed<MinecraftAccountsActions | null>(() => {
	const card = accounts.value
	if (!card) return null
	return {
		loginDisabled: card.loginDisabled,
		refreshValues: card.refreshValues,
		setEquippedSkin: card.setEquippedSkin,
		setLoginDisabled: card.setLoginDisabled,
		login: card.login,
	}
})

function showAddFriendModal() {
}

defineExpose({ minecraftAccounts, showAddFriendModal })

const scrollbarOptions = Object.freeze({
	overflow: {
		x: 'hidden',
		y: 'scroll',
	},
})

const contentVisible = ref(props.visible)
let contentTimer: ReturnType<typeof setTimeout> | undefined
watch(
	() => props.visible,
	(isVisible) => {
		if (contentTimer) clearTimeout(contentTimer)
		if (isVisible) {
			contentVisible.value = true
		} else {
			contentTimer = setTimeout(() => {
				contentVisible.value = false
			}, 150)
		}
	},
)

const ENABLE_SANDBOX_SLOTS = true

const sandboxSidebarSections = computed(() => {
	if (!ENABLE_SANDBOX_SLOTS) return []
	try {
		return Array.from(sandboxTrees.entries())
			.filter(([, trees]) => (trees.sidebar?.length ?? 0) > 0)
			.map(([pluginId, trees]) => [pluginId, trees] as const)
	} catch {
		return []
	}
})

function renderSandboxSidebar(pluginId: string, trees: { sidebar?: unknown[] }) {
	try {
		return renderUiTree((trees.sidebar ?? []) as UiNode[], (eventId, payload) =>
			dispatchSandboxEvent(pluginId, eventId, payload),
		)
	} catch {
		return []
	}
}

const messages = defineMessages({
	playingAs: {
		id: 'app.sidebar.playing-as',
		defaultMessage: 'Playing as',
	},
})
</script>

<template>
	<div
		class="app-sidebar mt-px relative flex h-[calc(100vh-var(--top-bar-height))] w-[300px] shrink-0 flex-col overflow-visible border-0 border-l-[1px] border-solid border-[--brand-gradient-border] before:pointer-events-none before:absolute before:inset-y-0 before:-left-8 before:w-8 before:content-['']"
		:class="{
			'before:shadow-[-15px_0_15px_-15px_rgba(0,0,0,0.1)_inset]': appTheme.advancedRendering,
		}"
	>
		<div class="relative flex min-h-0 grow shrink flex-col">
			<div
				ref="scrollContainer"
				v-overlay-scrollbars="scrollbarOptions"
				class="app-sidebar-scrollable relative min-h-0 grow shrink"
				data-overlayscrollbars-initialize
			>
				<OnboardingChecklist
					@create-instance="emit('create-instance')"
					@login-minecraft="accounts?.login()"
					@login-modrinth="emit('login-modrinth')"
				/>
				<div
					id="sidebar-teleport-target"
					class="sidebar-teleport-content empty:hidden"
				></div>
				<div class="sidebar-default-content hidden" :class="{ 'sidebar-enabled': contentVisible }">
					<div
						v-show="hasLoggedIntoMinecraft"
						class="p-4 border-0 border-b-[1px] border-[--brand-gradient-border] border-solid"
					>
						<h3 class="text-base text-primary font-medium m-0">
							{{ formatMessage(messages.playingAs) }}
						</h3>
						<Suspense>
							<AccountsCard ref="accounts" />
						</Suspense>
					</div>
					<div
						v-for="[pluginId, trees] in sandboxSidebarSections"
						:key="pluginId"
						class="border-0 border-b-[1px] border-[--brand-gradient-border] border-solid"
					>
						<div class="flex flex-col gap-2 p-4">
							<component
								:is="node"
								v-for="(node, index) in renderSandboxSidebar(pluginId, trees)"
								:key="index"
							/>
						</div>
					</div>
					<div
						v-for="section in pluginRegistry.sidebarSections"
						:key="section.id"
						class="border-0 border-b-[1px] border-[--brand-gradient-border] border-solid"
					>
						<PluginErrorBoundary :plugin-id="section.pluginId">
							<component :is="section.component" />
						</PluginErrorBoundary>
					</div>
				</div>
			</div>
			<div
				class="sidebar-scroll-fade pointer-events-none absolute inset-x-0 top-0 z-10 h-8 rotate-180 transition-opacity duration-200 motion-reduce:transition-none"
				:class="showTopFade ? 'opacity-100' : 'opacity-0'"
				aria-hidden="true"
			/>
			<div
				class="sidebar-scroll-fade pointer-events-none absolute inset-x-0 bottom-0 z-10 h-8 transition-opacity duration-200 motion-reduce:transition-none"
				:class="showAd || showBottomFade ? 'opacity-100' : 'opacity-0'"
				aria-hidden="true"
			/>
		</div>
	</div>
</template>

<style scoped>
.app-sidebar {
  background: color-mix(in srgb, var(--color-brand) 10%, var(--color-raised-bg));
	--color-button-bg: var(--brand-gradient-button);
	--surface-4: var(--brand-gradient-button);
	--color-button-bg-hover: var(--brand-gradient-border);
	--surface-5: var(--brand-gradient-border);
	--color-divider: var(--brand-gradient-border);
	--color-divider-dark: var(--brand-gradient-border);
}

.sidebar-teleport-content:empty + .sidebar-default-content.sidebar-enabled {
	display: contents;
}

.sidebar-scroll-fade {
  /* 如果背景色是深灰，直接给它改成透明或者深灰 */
  background: transparent; /* 或者 background: var(--color-bg); */
  display: none; /* 如果你根本不想要这个效果，直接隐藏最省事 */
}
</style>

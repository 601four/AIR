<script setup lang="ts">
import { PlayIcon, PlusIcon } from '@modrinth/assets'
import { Button, ContextMenu, defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import { computed, inject, ref } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'

import LibrarySection from '@/components/ui/library/index.vue'
import { libraryScrollTop } from '@/components/ui/library/view-state'
import { useAppSettings } from '@/composables/use-app-settings.ts'
import { traceStartupStep } from '@/helpers/startup-debug'
import { instanceListQueryOptions } from '@/pages/instance/query-options'
import { injectOnboardingChecklist } from '@/providers/onboarding-checklist'
import { useRootBreadcrumb } from '@/shared/breadcrumbs'

defineOptions({
	name: 'InstanceLibraryPage',
})

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const { hasCreatedInstance, isReady } = injectOnboardingChecklist()
const showCreationModal = inject<() => void>('showCreationModal')
const pageOptions = ref<InstanceType<typeof ContextMenu>>()
const appSettings = useAppSettings()
onBeforeRouteLeave(() => {
	libraryScrollTop.value = document.querySelector('.app-viewport')?.scrollTop ?? 0
})

const messages = defineMessages({
	emptyNone: { id: 'app.library.empty.none', defaultMessage: 'No instances yet' },
	emptyCreate: { id: 'app.library.empty.create', defaultMessage: 'New instance' },
	home: {
		id: 'app.navigation.home',
		defaultMessage: 'Home',
	},
	library: {
		id: 'app.navigation.library',
		defaultMessage: 'Instances',
	},
	newInstance: {
		id: 'app.library.context-menu.create-instance',
		defaultMessage: 'New instance',
	},
	libraryActionsLabel: {
		id: 'app.library.actions.label',
		defaultMessage: 'Library actions',
	},
})

useRootBreadcrumb({
	slot: 'root',
	id: 'library',
	label: formatMessage(messages.library),
	to: '/library',
	visual: { type: 'icon', component: PlayIcon },
})

const instancesQuery = useQuery(instanceListQueryOptions())
const instances = computed(() => instancesQuery.data.value ?? [])
if (hasCreatedInstance.value) {
	await traceStartupStep('Load library instances', () => instancesQuery.suspense()).catch(
		handleError,
	)
}

function openPageContextMenu(event: MouseEvent) {
	if (
		!(event.target instanceof HTMLElement) ||
		!event.target.hasAttribute('data-library-page-background')
	) {
		return
	}

	event.preventDefault()
	event.stopPropagation()
	pageOptions.value?.open(event, [
		{
			id: 'new_instance',
			label: formatMessage(messages.newInstance),
			icon: PlusIcon,
			action: () => showCreationModal?.(),
		},
	])
}
</script>

<template>
	<!-- 实例库页面：无论有没有实例都显示实例库本身的布局；
	     空的时候不切换到欢迎页，而是在中间显示「新建实例」 -->
	<div
		v-if="isReady"
		data-library-page-background
		class="relative flex min-h-full flex-col gap-3 p-6"
		@contextmenu="openPageContextMenu"
	>
		<LibrarySection :instances="instances" />

		<div
			v-if="instances.length === 0"
			class="pointer-events-none absolute inset-0 flex items-center justify-center"
		>
			<div class="pointer-events-auto flex flex-col items-center gap-3 text-center">
				<p class="m-0 text-secondary">{{ formatMessage(messages.emptyNone) }}</p>
				<Button type="colored" color="brand" @click="showCreationModal?.()">
					<PlusIcon />
					{{ formatMessage(messages.emptyCreate) }}
				</Button>
			</div>
		</div>

		<ContextMenu ref="pageOptions" :label="formatMessage(messages.libraryActionsLabel)" />
	</div>
</template>

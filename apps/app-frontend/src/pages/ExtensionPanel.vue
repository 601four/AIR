<template>
	<div class="p-6">
		<div v-if="sandboxEntry && appSettings.devMode" class="mb-3 flex justify-end">
			<Button type="outlined" @click="reloadSandbox">{{ formatMessage(messages.reload) }}</Button>
		</div>
		<div v-if="sandboxEntry" class="flex flex-col gap-3">
			<div v-for="(node, index) in sandboxPage" :key="index">
				<component :is="node" />
			</div>
			<div v-if="sandboxPage.length === 0" class="text-secondary">
				{{ formatMessage(messages.notRendered) }}
			</div>
		</div>
		<PluginErrorBoundary v-else-if="panel" :plugin-id="panel.pluginId">
			<component :is="panel.component" />
		</PluginErrorBoundary>
		<Admonition
			v-else-if="extensionDisabled"
			type="warning"
			:header="formatMessage(messages.disabledTitle)"
			:body="formatMessage(messages.disabledBody)"
		/>
		<Admonition
			v-else-if="route.name === 'ExtensionPanel' && id"
			type="warning"
			:header="formatMessage(messages.notFoundTitle)"
			:body="formatMessage(messages.notFoundBody, { id })"
		/>
	</div>
</template>

<script setup lang="ts">
import { Admonition, Button, defineMessages, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'
import { useRoute } from 'vue-router'

import { useAppSettings } from '@/composables/use-app-settings'

import PluginErrorBoundary from '@/components/ui/PluginErrorBoundary.vue'
import { renderUiTree, type UiNode } from '@/plugins/ui-renderer'
import {
	bumpSandboxToken,
	dispatchSandboxEvent,
	getSandboxModel,
	pluginRegistry,
	pluginState,
	sandboxTrees,
	setSandboxValue,
} from '@/plugin-host'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	reload: { id: 'app.extensions.reload', defaultMessage: 'Reload plugin page' },
	notRendered: {
		id: 'app.extensions.not-rendered',
		defaultMessage: 'Plugin page has not rendered yet. If this persists, check the Plugin logs page.',
	},
	disabledTitle: { id: 'app.extensions.disabled-title', defaultMessage: 'Plugin disabled' },
	disabledBody: {
		id: 'app.extensions.disabled-body',
		defaultMessage: 'Re-enable it in Settings → Plugins, or uninstall and reinstall.',
	},
	notFoundTitle: { id: 'app.extensions.not-found-title', defaultMessage: 'Plugin not found' },
	notFoundBody: { id: 'app.extensions.not-found-body', defaultMessage: 'No panel named {id}' },
})

const route = useRoute()
const appSettings = useAppSettings()
const id = computed(() => String(route.params.id ?? ''))
const extensionDisabled = computed(() => {
	const extension = pluginState.extensions.find((item) => item.id === id.value)
	return !!extension && extension.enabled === false
})

const sandboxEntry = computed(() => {
	const extension = pluginState.extensions.find((item) => item.id === id.value)
	const entry = extension?.entry ?? ''
	return entry.endsWith('.html') ? entry : null
})

const sandboxPermissions = computed(
	() => pluginState.extensions.find((item) => item.id === id.value)?.permissions ?? [],
)

function reloadSandbox() {
	console.info('[sandbox] 手动重载:', id.value)
	bumpSandboxToken(id.value)
}

const sandboxPage = computed(() =>
	renderUiTree(
		(sandboxTrees.get(id.value)?.page ?? []) as UiNode[],
		(eventId, payload) => {
			// 表单值记住，重渲染不丢
			if (typeof payload === 'string') setSandboxValue(id.value, eventId, payload)
			dispatchSandboxEvent(id.value, eventId, payload)
		},
		getSandboxModel(id.value),
	) as unknown[],
)

const panel = computed(
	() =>
		pluginRegistry.panels.find((item) => item.id === id.value) ??
		pluginRegistry.navItems.find((item) => item.id === id.value),
)
</script>

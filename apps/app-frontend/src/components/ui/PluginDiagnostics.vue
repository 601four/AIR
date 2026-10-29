<template>
	<div class="flex flex-col gap-3">
		<Admonition
			v-if="pluginState.error"
			type="critical"
			header="插件加载出错"
			:body="pluginState.error"
		/>
		<Admonition
			v-if="pluginState.extensions.length === 0"
			type="warning"
			header="没找到任何插件"
			body="请把插件放到 %APPDATA%\\com.aircore.launcher\\extensions\\<id>\\，其中包含 manifest.json 与 index.js"
		/>
		<div v-else class="flex flex-col gap-2">
			<h2 class="m-0 text-lg font-semibold text-contrast">已发现的插件</h2>
			<div
				v-for="extension in pluginState.extensions"
				:key="extension.id"
				class="rounded-xl border border-solid border-surface-4 bg-surface-3 p-3 font-mono text-sm text-secondary"
			>
				{{ extension.id }} → {{ extension.entry ?? 'index.js' }}
			</div>
		</div>
		<div class="flex flex-col gap-2">
			<h2 class="m-0 text-lg font-semibold text-contrast">已注册面板</h2>
			<div v-if="pluginRegistry.panels.length === 0" class="text-sm text-secondary">
				没有面板被注册（插件脚本可能没执行）
			</div>
			<div
				v-for="panel in pluginRegistry.panels"
				:key="panel.id"
				class="rounded-xl border border-solid border-surface-4 bg-surface-3 p-3 font-mono text-sm text-secondary"
			>
				{{ panel.id }} — {{ panel.title }}
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
import { Admonition } from '@modrinth/ui'

import { pluginRegistry, pluginState } from '@/plugin-host'
</script>

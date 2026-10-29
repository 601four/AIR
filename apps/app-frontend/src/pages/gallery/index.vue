<!-- 插件日志：查看插件加载、注册与 API 调用 -->
<template>
	<div class="flex flex-col gap-4 p-6">
		<div class="flex items-center justify-between gap-4">
			<div>
				<h1 class="m-0 text-2xl font-bold text-contrast">插件日志</h1>
				<p class="m-0 mt-1 text-secondary">
					共 {{ pluginLogs.length }} 条记录 · 已发现 {{ pluginState.extensions.length }} 个插件 · 已注册
					{{ pluginRegistry.panels.length + pluginRegistry.navItems.length }} 个面板
				</p>
			</div>
			<Button type="outlined" @click="pluginLogs.splice(0, pluginLogs.length)">清空</Button>
		</div>

		<Admonition
			v-if="pluginState.error"
			type="critical"
			header="插件错误"
			:body="pluginState.error"
		/>

		<div v-if="pluginLogs.length === 0" class="text-secondary">还没有记录。</div>

		<div v-else class="flex flex-col gap-1 font-mono text-sm">
			<div
				v-for="(entry, index) in pluginLogs"
				:key="index"
				class="flex gap-3 rounded-lg border border-solid border-surface-4 bg-surface-3 px-3 py-2"
			>
				<span class="shrink-0 text-secondary">{{ entry.at }}</span>
				<span
					class="w-12 shrink-0 uppercase"
					:class="{
						'text-brand-red': entry.level === 'error',
						'text-brand-orange': entry.level === 'warn',
						'text-brand-blue': entry.level === 'info',
					}"
				>
					{{ entry.level }}
				</span>
				<span v-if="entry.pluginId" class="shrink-0 text-brand">{{ entry.pluginId }}</span>
				<span class="text-contrast">{{ entry.message }}</span>
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
import { Admonition, Button } from '@modrinth/ui'

import { pluginLogs, pluginRegistry, pluginState } from '@/plugin-host'
</script>

<script setup lang="ts">
import { SpinnerIcon } from '@modrinth/assets'
import { Button, commonMessages, defineMessages, Input, Slider, Toggle, useVIntl } from '@modrinth/ui'

import { computed, ref, watch } from 'vue'

import { recommendMemoryMb, useMemorySettings } from '@/composables/use-memory-settings.ts'
import useMemorySlider from '@/composables/useMemorySlider'
import { parseEnvVars, serializeEnvVars } from '@/helpers/settings'

import { useDefaultInstanceSettings } from '../use-default-instance-settings'
import JavaInstallations from './java-installations.vue'

const { formatMessage } = useVIntl()
const { settings, settingsQuery } = useDefaultInstanceSettings(
	(value) => ({
		memory: { ...value.memory },
		launchArgs: value.extra_launch_args.join(' '),
		envVars: serializeEnvVars(value.custom_env_vars),
	}),
	(value) => ({
		memory: { ...value.memory },
		extra_launch_args: value.launchArgs.trim().split(/\s+/).filter(Boolean),
		custom_env_vars: parseEnvVars(value.envVars),
	}),
)
const { maxMemory, snapPoints, memoryQuery } = useMemorySlider()

// 自动内存分配
const { autoMemory, setAutoMemory, prelaunchOptimize, setPrelaunchOptimize } = useMemorySettings()

const optimizeResult = ref(null)
async function optimizeNow() {
	const { runMemoryOptimize } = await import('@/composables/use-memory-settings')
	optimizeResult.value = await runMemoryOptimize()
}
const autoMemoryEnabled = computed(() => autoMemory.value)
const recommendedMemory = computed(() => recommendMemoryMb(maxMemory.value))
watch(
	[recommendedMemory, autoMemoryEnabled],
	([recommended, enabled]) => {
		if (enabled && settings.value) {
			settings.value.memory.maximum = recommended
		}
	},
	{ immediate: true },
)
function onAutoMemoryToggle(value: boolean) {
	setAutoMemory(value)
}

function onPrelaunchToggle(value: boolean) {
	setPrelaunchOptimize(value)
}

const messages = defineMessages({
	autoMemoryLabel: { id: 'app.memory.auto', defaultMessage: 'Automatic memory allocation' },
	autoMemoryHint: {
		id: 'app.memory.autoHint',
		defaultMessage: '(Recommended from physical memory, currently {mb} MB)',
	},
	optimizeNow: { id: 'app.memory.optimizeNow', defaultMessage: 'Optimize now' },
	optimizeResult: {
		id: 'app.memory.optimizeResult',
		defaultMessage:
			'Released {freed} MB · available {available} MB · trimmed {processes} processes',
	},
	prelaunchLabel: {
		id: 'app.memory.prelaunch',
		defaultMessage: 'Optimize memory before launch',
	},
	prelaunchHint: {
		id: 'app.memory.prelaunchHint',
		defaultMessage: 'Reclaims idle physical memory from running programs before launch to free up more memory for the game',
	},
	javaInstallationsTitle: {
		id: 'app.settings.tabs.java-installations',
		defaultMessage: 'Java installations',
	},
	memoryAllocationTitle: {
		id: 'app.settings.default-instance-options.memory-allocation.title',
		defaultMessage: 'Memory allocation',
	},
	memoryAllocationDescription: {
		id: 'app.settings.default-instance-options.memory-allocation.description',
		defaultMessage: 'Maximum memory available to each instance.',
	},
	javaArgumentsTitle: {
		id: 'app.settings.default-instance-options.java-arguments.title',
		defaultMessage: 'Java arguments',
	},
	javaArgumentsPlaceholder: {
		id: 'app.settings.default-instance-options.java-arguments.placeholder',
		defaultMessage: 'Enter Java arguments...',
	},
	javaArgumentsDescription: {
		id: 'app.settings.default-instance-options.java-arguments.description',
		defaultMessage: 'Arguments passed to Java when launching an instance.',
	},
	environmentVariablesTitle: {
		id: 'app.settings.default-instance-options.environment-variables.title',
		defaultMessage: 'Environment variables',
	},
	environmentVariablesPlaceholder: {
		id: 'app.settings.default-instance-options.environment-variables.placeholder',
		defaultMessage: 'Enter environment variables...',
	},
	environmentVariablesDescription: {
		id: 'app.settings.default-instance-options.environment-variables.description',
		defaultMessage: 'Environment variables set when launching an instance.',
	},
})
</script>

<template>
	<div class="flex flex-col gap-6">
		<section class="flex flex-col gap-4">
			<h2 class="m-0 text-xl font-semibold text-contrast">
				{{ formatMessage(messages.javaInstallationsTitle) }} <!-- 显示：Java 管理 -->
			</h2>
			<JavaInstallations />
		</section>
		<div class="flex flex-col gap-6 border-0 border-t border-solid border-surface-5 pt-6">
			<template v-if="settings">
				<div class="flex flex-col gap-2.5">
					<h2 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.memoryAllocationTitle) }} <!-- 显示：内存分配 -->
					</h2>
					<label class="flex items-center gap-3">
						<Toggle :model-value="autoMemoryEnabled" @update:model-value="onAutoMemoryToggle" />
						<span class="text-contrast">{{ formatMessage(messages.autoMemoryLabel) }}</span>
						<span class="text-sm text-secondary">
							{{ formatMessage(messages.autoMemoryHint, { mb: recommendedMemory }) }}
						</span>
					</label>
					<Slider
						v-if="maxMemory"
						:disabled="autoMemoryEnabled"
						id="max-memory"
						v-model="settings.memory.maximum"
						:aria-label="formatMessage(messages.memoryAllocationTitle)"
						:min="512"
						:max="maxMemory"
						:step="64"
						:snap-points="snapPoints"
						:snap-range="512"
						min-label="512 MB"
						:max-label="`${Number((maxMemory / 1024).toFixed(1))} GB`"
						unit="MB"
					/>
					<Button v-else-if="memoryQuery.isError.value" @click="memoryQuery.refetch()">
						{{ formatMessage(commonMessages.refreshButton) }}
					</Button>
					<div
						v-else
						role="status"
						:aria-label="formatMessage(commonMessages.loadingLabel)"
						class="h-10 animate-pulse rounded-lg bg-surface-3"
					/>
					<p class="m-0 mt-1 leading-tight">
						{{ formatMessage(messages.memoryAllocationDescription) }} <!-- 显示：每个实例的最大可用内存。 -->
					</p>

					<label class="mt-1 flex items-center gap-3">
						<Toggle :model-value="prelaunchOptimize" @update:model-value="onPrelaunchToggle" />
						<span class="text-contrast">{{ formatMessage(messages.prelaunchLabel) }}</span>
						<span class="text-sm text-secondary">
							{{ formatMessage(messages.prelaunchHint) }}
						</span>
					</label>
					<div class="mt-1 flex items-center gap-3">
						<Button type="outlined" size="sm" @click="optimizeNow">
							{{ formatMessage(messages.optimizeNow) }}
						</Button>
						<span v-if="optimizeResult" class="text-sm text-secondary">
							{{
								formatMessage(messages.optimizeResult, {
									freed: optimizeResult.freed_mb,
									available: optimizeResult.available_after_mb,
									processes: optimizeResult.trimmed_processes,
								})
							}}
						</span>
					</div>
				</div>
				<div class="flex flex-col gap-2.5">
					<h2 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.javaArgumentsTitle) }} <!-- 显示：Java 参数 -->
					</h2>
					<Input
						id="java-args"
						v-model="settings.launchArgs"
						:aria-label="formatMessage(messages.javaArgumentsTitle)"
						autocomplete="off"
						type="text"
						:placeholder="formatMessage(messages.javaArgumentsPlaceholder)"
						wrapper-class="w-full"
					/>
					<p class="m-0 leading-tight">
						{{ formatMessage(messages.javaArgumentsDescription) }} <!-- 显示：启动实例时传递给 Java 的参数。 -->
					</p>
				</div>
				<div class="flex flex-col gap-2.5">
					<h2 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.environmentVariablesTitle) }} <!-- 显示：环境变量 -->
					</h2>
					<Input
						id="env-vars"
						v-model="settings.envVars"
						:aria-label="formatMessage(messages.environmentVariablesTitle)"
						autocomplete="off"
						type="text"
						:placeholder="formatMessage(messages.environmentVariablesPlaceholder)"
						wrapper-class="w-full"
					/>
					<p class="m-0 leading-tight">
						{{ formatMessage(messages.environmentVariablesDescription) }} <!-- 显示：启动实例时设置的环境变量。 -->
					</p>
				</div>
			</template>
			<Button v-else-if="settingsQuery.isError.value" @click="settingsQuery.refetch()">
				{{ formatMessage(commonMessages.refreshButton) }}
			</Button>
			<div v-else class="flex items-center gap-2 text-secondary">
				<SpinnerIcon class="size-5 animate-spin" aria-hidden="true" />
				{{ formatMessage(commonMessages.loadingLabel) }}
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
import { ChevronDownIcon, PlayIcon, SearchIcon, StopCircleIcon } from '@modrinth/assets'
import {
	Button,
	defineMessages,
	Input,
	NewModal,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'

dayjs.extend(relativeTime)
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

import WelcomeScreen from '@/components/ui/WelcomeScreen.vue'
import { useAppEvent } from '@/composables/use-app-event'
import { get_default_user, users } from '@/helpers/auth'
import { getInstanceIconUrl } from '@/helpers/instance'
import { kill, run as runInstance } from '@/helpers/instance'
import { get_all } from '@/helpers/process'
import { traceStartupStep } from '@/helpers/startup-debug'
import { instanceListQueryOptions } from '@/pages/instance/query-options'
import { injectOnboardingChecklist } from '@/providers/onboarding-checklist'
import { useRootBreadcrumb } from '@/shared/breadcrumbs'

defineOptions({
	name: 'HomePage',
})

const { formatMessage } = useVIntl()
const { handleError, addNotification } = injectNotificationManager()
const { hasCreatedInstance, isReady } = injectOnboardingChecklist()
const router = useRouter()

const messages = defineMessages({
	home: { id: 'app.navigation.home', defaultMessage: 'Home' },
	selectInstance: { id: 'app.home.select-instance', defaultMessage: 'Select instance' },
	searchInstances: { id: 'app.home.search-instances', defaultMessage: 'Search instances' },
	launch: { id: 'app.home.launch', defaultMessage: 'Play' },
	viewAll: { id: 'app.home.view-all', defaultMessage: 'View all instances' },
	neverPlayed: { id: 'app.home.never-played', defaultMessage: 'Never played' },
	launchFailed: { id: 'app.home.launch-failed', defaultMessage: 'Failed to launch' },
	stopFailed: { id: 'app.home.stop-failed', defaultMessage: 'Failed to stop' },
	justNow: { id: 'app.home.time.just-now', defaultMessage: 'just now' },
	minutesAgo: { id: 'app.home.time.minutes', defaultMessage: '{count} minutes ago' },
	hoursAgo: { id: 'app.home.time.hours', defaultMessage: '{count} hours ago' },
	daysAgo: { id: 'app.home.time.days', defaultMessage: '{count} days ago' },
	lastPlayed: { id: 'app.home.last-played', defaultMessage: 'Last played {when}' },
	cancel: { id: 'app.home.cancel', defaultMessage: 'Cancel' },
	instanceCount: { id: 'app.home.instance-count', defaultMessage: '{count} instances' },
	stop: { id: 'app.home.stop', defaultMessage: 'Stop game' },
	installing: { id: 'app.home.installing', defaultMessage: 'Installing…' },
	greetingWithName: { id: 'app.home.greeting.with-name', defaultMessage: '{greeting}, {name}' },
	greetingDawn: { id: 'app.home.greeting.dawn', defaultMessage: 'Good night' },
	greetingMorning: { id: 'app.home.greeting.morning', defaultMessage: 'Good morning' },
	greetingNoon: { id: 'app.home.greeting.noon', defaultMessage: 'Good afternoon' },
	greetingAfternoon: { id: 'app.home.greeting.afternoon', defaultMessage: 'Good afternoon' },
	greetingEvening: { id: 'app.home.greeting.evening', defaultMessage: 'Good evening' },
})

useRootBreadcrumb({
	slot: 'root',
	id: 'home',
	label: formatMessage(messages.home),
	to: '/',
	visual: { type: 'icon', component: PlayIcon },
})

const instancesQuery = useQuery(instanceListQueryOptions())
const instances = computed(() => instancesQuery.data.value ?? [])
if (hasCreatedInstance.value) {
	await traceStartupStep('Load library instances', () => instancesQuery.suspense()).catch(
		handleError,
	)
}

const sortedInstances = computed(() =>
	instances.value
		.slice()
		.sort((a, b) => dayjs(b.last_played ?? b.created).diff(dayjs(a.last_played ?? a.created))),
)

const accountsQuery = useQuery({
	queryKey: ['auth', 'accounts'],
	queryFn: async () => ({ list: await users(), defaultId: await get_default_user() }),
})

const accountName = computed(() => {
	const data = accountsQuery.data.value
	if (!data?.list?.length) return null
	const current = data.list.find((account) => account.id === data.defaultId) ?? data.list[0]
	return current?.profile?.name ?? null
})

const greeting = computed(() => {
	const hour = new Date().getHours()
	if (hour < 5) return formatMessage(messages.greetingDawn)
	if (hour < 11) return formatMessage(messages.greetingMorning)
	if (hour < 13) return formatMessage(messages.greetingNoon)
	if (hour < 18) return formatMessage(messages.greetingAfternoon)
	return formatMessage(messages.greetingEvening)
})

const greetingText = computed(() =>
	accountName.value
		? formatMessage(messages.greetingWithName, {
				greeting: greeting.value,
				name: accountName.value,
			})
		: greeting.value,
)

// 运行中实例
const processesQuery = useQuery({
	queryKey: ['processes', 'list'],
	queryFn: () => get_all(),
})
const runningInstances = computed(() =>
	(processesQuery.data.value ?? []).map((process) => process.instance_id),
)
const isInstalling = computed(() => {
	const stage = selectedInstance.value?.install_stage
	if (!stage) return false
	return stage.includes('installing')
})

const isRunning = computed(
	() => !!selectedInstance.value && runningInstances.value.includes(selectedInstance.value.id),
)

const selectedId = ref<string | null>(null)
const pickerModal = ref()
const searchQuery = ref('')

const selectedInstance = computed(() => {
	const list = sortedInstances.value
	if (list.length === 0) return null
	return list.find((instance) => instance.id === selectedId.value) ?? list[0]
})

const filteredInstances = computed(() => {
	const query = searchQuery.value.trim().toLowerCase()
	if (!query) return sortedInstances.value
	return sortedInstances.value.filter((instance) => instance.name.toLowerCase().includes(query))
})

const iconSrc = computed(() =>
	selectedInstance.value ? getInstanceIconUrl(selectedInstance.value.icon_path) : '',
)

const lastPlayedLabel = computed(() => {
	const lastPlayed = selectedInstance.value?.last_played
	if (!lastPlayed) return formatMessage(messages.neverPlayed)
	const played = dayjs(lastPlayed)
	const minutes = dayjs().diff(played, 'minute')
	if (minutes < 1) return formatMessage(messages.justNow)
	if (minutes < 60) return formatMessage(messages.minutesAgo, { count: minutes })
	const hours = Math.floor(minutes / 60)
	if (hours < 24) return formatMessage(messages.hoursAgo, { count: hours })
	return formatMessage(messages.daysAgo, { count: Math.floor(hours / 24) })
})


function openPicker() {
	searchQuery.value = ''
	pickerModal.value?.show()
}

function choose(instance) {
	selectedId.value = instance.id
	pickerModal.value?.hide()
}

// 正在启动 / 结束中的实例（按 id 记录，允许同时存在多个）
const pendingInstances = ref(new Set())

const actionPending = computed(
	() => !!selectedInstance.value && pendingInstances.value.has(selectedInstance.value.id),
)

function markPending(instanceId, pending) {
	const next = new Set(pendingInstances.value)
	if (pending) next.add(instanceId)
	else next.delete(instanceId)
	pendingInstances.value = next
}

async function refreshRunning() {
	await processesQuery.refetch()
}

// 进程列表变化时，刷新“是否运行中”
useAppEvent('process', () => {
	void refreshRunning()
})

// 实例安装状态变化时，刷新列表（按钮从「安装中」变回「开始游戏」）
useAppEvent('instance', () => {
	void instancesQuery.refetch()
})

async function launch() {
	const instance = selectedInstance.value
	if (!instance || pendingInstances.value.has(instance.id)) return
	markPending(instance.id, true)
	try {
		await runInstance(instance.id)
		await refreshRunning()
	} catch (error) {
		addNotification({ type: 'error', title: formatMessage(messages.launchFailed), text: String(error) })
	} finally {
		markPending(instance.id, false)
		await refreshRunning()
	}
}

async function stop() {
	const instance = selectedInstance.value
	if (!instance || pendingInstances.value.has(instance.id)) return
	markPending(instance.id, true)
	try {
		await kill(instance.id)
	} catch (error) {
		addNotification({ type: 'error', title: formatMessage(messages.stopFailed), text: String(error) })
	} finally {
		markPending(instance.id, false)
		await refreshRunning()
	}
}
</script>

<template>
  <WelcomeScreen v-if="isReady && instances.length === 0" />
  <div v-else-if="isReady" class="flex min-h-full flex-col items-center justify-center p-6">

    <!-- 用一个统一的宽度容器包裹所有内容，确保内部元素 100% 对齐 -->
    <div class="flex w-full max-w-3xl flex-col">

      <!-- 问候语：居中 -->
      <div class="text-center mb-6">
        <h1 class="m-0 text-3xl font-bold text-contrast">{{ greetingText }}</h1>
      </div>

      <!-- 实例选择按钮行：与下方卡片的外边缘左对齐 -->
      <div class="flex items-center justify-start gap-3 mb-3">
        <Button type="outlined" class="max-w-72 justify-between" @click="openPicker">
          <span class="flex min-w-0 items-center gap-2">
            <img v-if="iconSrc" :src="iconSrc" alt="" class="h-5 w-5 rounded-md object-cover" />
            <span class="truncate">{{ selectedInstance?.name }}</span>
          </span>
          <ChevronDownIcon class="ml-2 h-4 w-4 shrink-0" />
        </Button>
        <Button type="quiet" size="sm" @click="router.push('/library')">
          {{ formatMessage(messages.viewAll) }}
        </Button>
      </div>

      <!-- 当前实例卡片 -->
      <div
          v-if="selectedInstance"
          class="flex w-full items-center gap-4 rounded-2xl border border-solid border-surface-4 bg-surface-3 p-4"
      >
        <img
            v-if="iconSrc"
            :src="iconSrc"
            alt=""
            class="h-14 w-14 shrink-0 rounded-xl border border-solid border-surface-4 object-cover"
        />
        <div class="flex min-w-0 flex-1 flex-col gap-1">
          <span class="truncate text-xl font-bold text-contrast">{{ selectedInstance.name }}</span>
          <div class="flex items-center gap-4 text-secondary">
            <span>{{ selectedInstance.loader }} {{ selectedInstance.game_version }}</span>
            <span>{{ lastPlayedLabel }}</span>
          </div>
        </div>
        <Button
            v-if="isInstalling"
            type="base"
            size="lg"
            class="shrink-0"
            disabled
        >
            {{ formatMessage(messages.installing) }}
        </Button>
        <Button
            v-else-if="!isRunning"
            type="colored"
            color="brand"
            size="lg"
            class="shrink-0"
            :disabled="actionPending"
            @click="launch"
        >
          <PlayIcon />
          {{ formatMessage(messages.launch) }}
        </Button>
        <Button v-else type="colored" color="red" size="lg" class="shrink-0 !text-white" :disabled="actionPending" @click="stop">
          <StopCircleIcon />
          {{ formatMessage(messages.stop) }}
        </Button>
      </div>

      <!-- 实例选择弹窗：搜索 + 列表 -->
      <NewModal ref="pickerModal" :header="formatMessage(messages.selectInstance)">
        <div class="-mt-2 flex flex-col gap-3">
          <Input
              v-model="searchQuery"
              :icon="SearchIcon"
              type="text"
              :placeholder="formatMessage(messages.searchInstances)"
              wrapper-class="w-full"
              clearable
          />
          <div class="flex max-h-[50vh] flex-col gap-1 overflow-y-auto">
            <button
                v-for="instance in filteredInstances"
                :key="instance.id"
                class="flex w-full cursor-pointer items-center gap-3 rounded-xl border border-solid border-transparent bg-transparent p-2 text-left hover:bg-surface-3"
                :class="{ '!border-brand !bg-surface-3': instance.id === selectedInstance?.id }"
                @click="choose(instance)"
            >
              <img
                  v-if="getInstanceIconUrl(instance.icon_path)"
                  :src="getInstanceIconUrl(instance.icon_path)"
                  alt=""
                  class="h-9 w-9 shrink-0 rounded-lg object-cover"
              />
              <div class="flex min-w-0 flex-col">
                <span class="truncate font-semibold text-contrast">{{ instance.name }}</span>
                <span class="text-sm text-secondary">
                  {{ instance.loader }} {{ instance.game_version }}
                </span>
              </div>
            </button>
          </div>
          <p class="m-0 text-sm text-secondary">
            {{ formatMessage(messages.instanceCount, { count: filteredInstances.length }) }}
          </p>
        </div>
        <template #actions>
          <Button type="outlined" @click="pickerModal?.hide()">
            {{ formatMessage(messages.cancel) }}
          </Button>
        </template>
      </NewModal>

    </div>
  </div>
</template>
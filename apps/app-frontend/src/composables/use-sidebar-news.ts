import { computed, ref } from 'vue'

import { setPref } from './use-prefs'

const KEY = 'aircore-sidebar-news'

function readFlag(): boolean {
	try {
		const raw = window.localStorage.getItem(KEY)
		if (raw === null) return true
		return raw === '1'
	} catch {
		return true
	}
}

const enabled = ref<boolean>(readFlag())

export function useSidebarNews() {
	const setEnabled = (value: boolean) => {
		enabled.value = value
		setPref(KEY, value ? '1' : '0')
	}

	return {
		sidebarNewsEnabled: computed(() => enabled.value),
		setSidebarNewsEnabled: setEnabled,
	}
}

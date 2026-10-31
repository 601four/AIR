import type { CfManualFile } from './curseforge-pack'

interface PendingCfNotice {
	title: string
	manual: CfManualFile[]
}

const pending = new Map<string, PendingCfNotice>()

/** 安装开始时不弹提示，等任务结束后再提示缺了哪些模组 */
export function registerCurseForgeNotice(jobId: string, notice: PendingCfNotice) {
	if (notice.manual.length === 0) return
	pending.set(jobId, notice)
}

export function takeCurseForgeNotice(jobId: string): PendingCfNotice | null {
	const notice = pending.get(jobId)
	pending.delete(jobId)
	return notice ?? null
}

const tempPacks = new Map<string, string>()

/** 记住这个任务用的临时 .mrpack，结束后删掉 */
export function registerCurseForgeTempPack(jobId: string, path: string) {
	tempPacks.set(jobId, path)
}

/** 任务结束后删除临时包 */
export async function cleanupCurseForgeTempPack(jobId: string) {
	const path = tempPacks.get(jobId)
	tempPacks.delete(jobId)
	if (!path) return
	const { invoke } = await import('@tauri-apps/api/core')
	await invoke('plugin:utils|cf_cleanup_temp_pack', { path }).catch(() => {})
}

/** 安装任务创建失败时立即删除 */
export async function cleanupTempPackByPath(path: string) {
	const { invoke } = await import('@tauri-apps/api/core')
	await invoke('plugin:utils|cf_cleanup_temp_pack', { path }).catch(() => {})
}

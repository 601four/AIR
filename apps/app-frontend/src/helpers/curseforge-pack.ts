import { invoke } from '@tauri-apps/api/core'
import { confirm } from '@tauri-apps/plugin-dialog'

import {
	cleanupTempPackByPath,
	registerCurseForgeNotice,
	registerCurseForgeTempPack,
} from './curseforge-notices'
import {
	install_create_modpack_instance,
	installJobInstanceId,
	type InstallJobSnapshot,
} from './install'

export interface CfPackInfo {
	title: string
	version: string | null
	game_version: string
	loader: string
	loader_version: string | null
	required_files: number
	optional_files: number
}

export interface CfManualFile {
	file_name: string
	display_name: string
	project_id: number
	file_id: number
	reason: string
}

export interface CfPrepareResult {
	mrpack_path: string
	title: string
	manual_files: CfManualFile[]
	optional_skipped: number
}

export async function readCurseForgePack(zipPath: string): Promise<CfPackInfo> {
	return await invoke<CfPackInfo>('plugin:utils|cf_pack_info', { zipPath })
}

/** 等实例真正写入数据库后再跳转，避免 unknown instance */
export async function waitForInstanceAvailable(
	instanceId: string,
	timeoutMs = 180000,
): Promise<boolean> {
	const { get } = await import('@/helpers/instance')
	const deadline = Date.now() + timeoutMs
	while (Date.now() < deadline) {
		const instance = await get(instanceId).catch(() => null)
		if (instance) return true
		await new Promise((resolve) => setTimeout(resolve, 500))
	}
	return false
}

/**
 * CurseForge 整合包安装：
 *   1. 解析 manifest，询问可选模组
 *   2. 转成临时 .mrpack（含 sha1 校验，上游安装器支持 sha1 回退）
 *   3. 交给上游安装任务：建实例、装游戏、装加载器、下载模组、解压 overrides
 */
export async function installCurseForgePack(zipPath: string): Promise<{
	job: InstallJobSnapshot
	prepared: CfPrepareResult
}> {
	const info = await readCurseForgePack(zipPath)

	let includeOptional = false
	if (info.optional_files > 0) {
		includeOptional = await confirm(
			`这个整合包包含 ${info.optional_files} 个可选模组，是否一并安装？`,
			{ title: info.title, kind: 'info' },
		).catch(() => false)
	}

	const prepared = await invoke<CfPrepareResult>('plugin:utils|cf_pack_to_mrpack', {
		zipPath,
		includeOptional,
	})
	let job: InstallJobSnapshot
	try {
		job = await install_create_modpack_instance(
			{
				type: 'fromFile',
				path: prepared.mrpack_path,
			},
			{ name: prepared.title },
		)
	} catch (error) {
		await cleanupTempPackByPath(prepared.mrpack_path)
		throw error
	}
	registerCurseForgeTempPack(job.job_id, prepared.mrpack_path)
	registerCurseForgeNotice(job.job_id, {
		title: prepared.title,
		manual: prepared.manual_files,
	})
	return { job, prepared }
}

export { installJobInstanceId }

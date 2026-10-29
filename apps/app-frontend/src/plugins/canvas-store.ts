import { reactive } from 'vue'

/** 插件画布指令（宿主负责执行） */
export type CanvasOp =
	| { op: 'clear'; color?: string }
	| { op: 'rect'; x: number; y: number; w: number; h: number; color: string }
	| { op: 'circle'; x: number; y: number; r: number; color: string; stroke?: string; strokeWidth?: number }
	| { op: 'text'; x: number; y: number; text: string; color?: string; size?: number }
	| { op: 'image'; image: string; x: number; y: number; w: number; h: number; smooth?: boolean }

const ops = new Map<string, CanvasOp[]>()
const images = new Map<string, HTMLImageElement>()

export const canvasVersion = reactive({ value: 0 })

export function setCanvasOps(canvasId: string, next: CanvasOp[]) {
	// 预加载图片，加载完逐个触发重绘
	for (const item of next) {
		if (item.op !== 'image') continue
		if (images.has(item.image)) continue
		const img = new Image()
		img.src = item.image
		img.decode().then(() => {
			images.set(item.image, img)
			canvasVersion.value += 1
		}).catch(() => {})
	}
	ops.set(canvasId, next)
	canvasVersion.value += 1
}

export function getCanvasOps(canvasId: string): CanvasOp[] {
	return ops.get(canvasId) ?? []
}

export function getCanvasImage(src: string): HTMLImageElement | null {
	return images.get(src) ?? null
}

export function clearCanvas(canvasId: string) {
	ops.delete(canvasId)
	canvasVersion.value += 1
}

<template>
	<div
		ref="containerEl"
		class="relative w-full overflow-hidden rounded-xl border border-solid border-surface-4 bg-surface-2"
		:style="{ height: `${height}px`, touchAction: 'none' }"
	>
		<canvas
			ref="canvasEl"
			class="absolute inset-0 h-full w-full"
			@pointerdown="onPointerDown"
			@pointermove="onPointerMove"
			@pointerup="onPointerUp"
			@pointercancel="onPointerUp"
			@wheel.prevent="onWheel"
		></canvas>
	</div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'

import { canvasVersion, getCanvasImage, getCanvasOps } from '@/plugins/canvas-store'

const props = withDefaults(
	defineProps<{
		id?: string
		height?: number
		interactive?: boolean
	}>(),
	{
		id: 'canvas',
		height: 480,
		interactive: true,
	},
)

const emit = defineEmits<{
	ptr: [{ canvasId: string; type: string; x: number; y: number; button?: number; deltaY?: number; width: number; height: number }]
}>()

const containerEl = ref<HTMLDivElement | null>(null)
const canvasEl = ref<HTMLCanvasElement | null>(null)
let resizeObserver: ResizeObserver | null = null

function draw() {
	const canvas = canvasEl.value
	const container = containerEl.value
	if (!canvas || !container) return
	const width = container.clientWidth
	const height = container.clientHeight
	if (width <= 0 || height <= 0) return
	if (canvas.width !== width || canvas.height !== height) {
		canvas.width = width
		canvas.height = height
	}
	const ctx = canvas.getContext('2d')
	if (!ctx) return

	ctx.clearRect(0, 0, width, height)
	ctx.fillStyle = '#242430'
	ctx.fillRect(0, 0, width, height)

	for (const op of getCanvasOps(props.id)) {
		if (op.op === 'clear') {
			ctx.fillStyle = op.color ?? '#242430'
			ctx.fillRect(0, 0, width, height)
		} else if (op.op === 'rect') {
			ctx.fillStyle = op.color
			ctx.fillRect(op.x, op.y, op.w, op.h)
		} else if (op.op === 'circle') {
			ctx.beginPath()
			ctx.fillStyle = op.color
			ctx.arc(op.x, op.y, op.r, 0, Math.PI * 2)
			ctx.fill()
			if (op.stroke) {
				ctx.strokeStyle = op.stroke
				ctx.lineWidth = op.strokeWidth ?? 1
				ctx.stroke()
			}
		} else if (op.op === 'text') {
			ctx.fillStyle = op.color ?? '#ffffff'
			ctx.font = `${op.size ?? 14}px sans-serif`
			ctx.fillText(op.text, op.x, op.y)
		} else if (op.op === 'image') {
			const img = getCanvasImage(op.image)
			if (img) {
				ctx.imageSmoothingEnabled = op.smooth ?? false
				ctx.drawImage(img, op.x, op.y, op.w, op.h)
			}
		}
	}
}

function payload(type: string, event: PointerEvent | WheelEvent) {
	const container = containerEl.value
	if (!container) return null
	const rect = container.getBoundingClientRect()
	return {
		canvasId: props.id,
		type,
		x: Math.round(event.clientX - rect.left),
		y: Math.round(event.clientY - rect.top),
		button: 'button' in event ? event.button : 0,
		deltaY: 'deltaY' in event ? event.deltaY : 0,
		width: rect.width,
		height: rect.height,
	}
}

function onPointerDown(event: PointerEvent) {
	if (!props.interactive) return
	;(event.target as HTMLElement).setPointerCapture?.(event.pointerId)
	const data = payload('down', event)
	if (data) emit('ptr', data)
}

function onPointerMove(event: PointerEvent) {
	if (!props.interactive) return
	const data = payload('move', event)
	if (data) emit('ptr', data)
}

function onPointerUp(event: PointerEvent) {
	if (!props.interactive) return
	const data = payload('up', event)
	if (data) emit('ptr', data)
}

function onWheel(event: WheelEvent) {
	if (!props.interactive) return
	const data = payload('wheel', event)
	if (data) emit('ptr', data)
}

onMounted(() => {
	draw()
	if (containerEl.value) {
		resizeObserver = new ResizeObserver(() => draw())
		resizeObserver.observe(containerEl.value)
	}
})

onBeforeUnmount(() => {
	resizeObserver?.disconnect()
})

watch(() => canvasVersion.value, draw)
</script>

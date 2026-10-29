import { Admonition, Button, Combobox, EmptyState, Toggle } from '@modrinth/ui'
import { h } from 'vue'

import PluginCanvasNode from '@/components/ui/PluginCanvasNode.vue'

/** UI 协议节点：插件只描述，宿主负责用自己的组件渲染 */
export interface UiNode {
	type: string
	value?: string
	text?: string
	label?: string
	id?: string
	size?: 'small' | 'normal' | 'title'
	kind?: string
	body?: string
	title?: string
	children?: UiNode[]
	columns?: number
	style?: 'primary' | 'outlined' | 'default' | 'transparent' | 'colored'
	placeholder?: string
	/** image 节点宽度（px），不填则随容器 */
	width?: number
	/** image 节点高度（px） */
	height?: number
	/** select 选项：字符串或 { value, label } */
	options?: (string | { value: string; label: string })[]
	/** toggle 开关 */
	checked?: boolean
	/** slider 范围 */
	min?: number
	max?: number
	step?: number
	/** canvas 节点：是否接收鼠标/轮滚事件 */
	interactive?: boolean
}

type Emit = (id: string, payload?: unknown) => void

export function renderUiTree(
	nodes: UiNode[] | undefined,
	emit: Emit,
	model: Record<string, string> = {},
): unknown[] {
	if (!Array.isArray(nodes)) return []
	return nodes.map((node, index) => renderNode(node, emit, index, model))
}

function renderNode(
	node: UiNode,
	emit: Emit,
	key: number,
	model: Record<string, string> = {},
): unknown {
	if (!node || typeof node !== 'object') return null

	switch (node.type) {
		case 'text':
			return h('p', { key, class: node.size === 'title' ? 'text-lg font-semibold text-contrast m-0' : 'text-secondary m-0' }, node.value ?? '')
		case 'title':
			return h('h2', { key, class: 'm-0 text-2xl font-bold text-contrast' }, node.value ?? '')
		case 'button':
			return h('div', { key, class: 'flex' }, [
				h(
					Button,
					{
						// 两种按钮形式都支持：
						//   style: 'primary' / 'colored'   → 实心品牌色按钮
						//   style: 'outlined' / 'transparent' → 透明描边按钮
						//   不填 → 默认灰色按钮
						type:
							node.style === 'outlined' || node.style === 'transparent'
								? 'outlined'
								: node.style === 'primary' || node.style === 'colored'
									? 'colored'
									: 'default',
						color:
							node.style === 'primary' || node.style === 'colored' ? 'brand' : undefined,
						// 边栏等窄容器里用小号按钮，避免看起来像主要操作
						size: node.size === 'small' ? 'sm' : node.size === 'title' ? 'lg' : 'md',
						onClick: () => {
							const raw = node.id ?? ''
							const [id, ...rest] = raw.split(':')
							emit(id, rest.length ? rest.join(':') : undefined)
						},
					},
					() => node.label ?? node.value ?? '按钮',
				),
			])
		case 'card':
			return h(
				'div',
				{ key, class: 'flex flex-col gap-3 rounded-2xl border border-solid border-surface-4 bg-surface-3 p-4' },
				renderUiTree(node.children, emit, model),
			)
		case 'row':
			return h('div', { key, class: 'flex flex-wrap items-center gap-3' }, renderUiTree(node.children, emit, model))
		case 'grid':
			return h(
				'div',
				{
					key,
					class: 'grid gap-3',
					style: { gridTemplateColumns: `repeat(${Math.max(1, node.columns ?? 2)}, minmax(0, 1fr))` },
				},
				renderUiTree(node.children, emit, model),
			)
		case 'admonition':
			return h(Admonition, {
				key,
				type: (node.kind as 'info' | 'warning' | 'critical' | 'success') ?? 'info',
				header: node.title ?? '',
				body: node.body ?? node.value ?? '',
			})
		case 'empty':
			return h(EmptyState, { key, heading: node.title ?? '什么都没有', description: node.body ?? '' })
		case 'input':
			return h('input', {
				key,
				class:
					'w-full rounded-lg border border-solid border-surface-4 bg-surface-2 px-3 py-2 text-contrast outline-none',
				placeholder: node.placeholder ?? '',
				value: model[node.id ?? ''] ?? '',
				onInput: (event: Event) => {
					const target = event.target as HTMLInputElement
					emit(node.id ?? '', target.value)
				},
			})
		case 'divider':
			return h('hr', { key, class: 'my-2 border-0 border-t border-solid border-divider' })
		case 'spacer':
			return h('div', { key, class: 'h-4' })
		case 'image':
			return h('img', {
				key,
				src: node.value,
				alt: node.text ?? '',
				class: 'rounded-xl',
				style: {
					width: node.width ? `${node.width}px` : 'auto',
					height: node.height ? `${node.height}px` : 'auto',
					maxWidth: '100%',
				},
			})
		case 'select': {
			const options = (node.options ?? []).map((option) =>
				typeof option === 'string' ? { value: option, label: option } : option,
			)
			const current = model[node.id ?? ''] ?? node.value ?? ''
			return h(Combobox, {
				key,
				modelValue: current,
				'options': options,
				triggerType: 'base',
				'onUpdate:modelValue': (value: string) => emit(node.id ?? '', value),
			})
		}
		case 'toggle':
			return h(
				'label',
				{ key, class: 'flex items-center justify-between gap-3 rounded-lg border border-solid border-surface-4 bg-surface-2 px-3 py-2' },
				[
					h('span', { class: 'text-secondary' }, node.label ?? node.value ?? ''),
					h(Toggle, {
						modelValue: (model[node.id ?? ''] ?? String(node.checked ?? false)) === 'true',
						'onUpdate:modelValue': (value: boolean) => emit(node.id ?? '', String(value)),
					}),
				],
			)
		case 'slider':
			return h('div', { key, class: 'flex flex-col gap-1' }, [
				h('span', { class: 'text-sm text-secondary' }, `${node.label ?? ''} ${model[node.id ?? ''] ?? node.value ?? node.min ?? 0}`),
				h('input', {
					type: 'range',
					class: 'w-full',
					min: node.min ?? 0,
					max: node.max ?? 100,
					step: node.step ?? 1,
					value: model[node.id ?? ''] ?? node.value ?? node.min ?? 0,
					onInput: (event: Event) => {
						emit(node.id ?? '', (event.target as HTMLInputElement).value)
					},
				}),
			])
		case 'canvas':
			return h(PluginCanvasNode, {
				key,
				id: node.id ?? 'canvas',
				height: node.height ?? 480,
				interactive: node.interactive !== false,
				onPtr: (data: unknown) => emit('canvas-input', data),
			})
		default:
			return h('div', { key, class: 'text-sm text-secondary' }, `未知元素: ${node.type}`)
	}
}

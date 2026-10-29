import { ref } from 'vue'

/** 侧栏展开/收起动画进行中（用于冻结依赖容器宽度的虚拟列表） */
export const sidebarAnimating = ref(false)

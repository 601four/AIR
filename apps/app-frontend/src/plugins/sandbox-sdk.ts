/**
 * 注入到插件 iframe 里的 SDK。
 * 插件只能通过它与宿主通信，拿不到 Tauri / Vue。
 */
export const SANDBOX_SDK = String.raw`
(function () {
  if (window.AirCore) return
  const pending = new Map()
  const listeners = new Map()
  let reqSeq = 0

  const api = {
    version: 1,

    /**
     * 渲染界面。
     *   render([...])                           → 插件自己的页面
     *   render({ slot: 'sidebar', tree: [...] }) → 右侧栏分区
     *   render({ slot: 'settings', title, tree }) → 设置里的标签页
     *   render({ slot: 'page', tree })           → 等同于传数组
     */
    render(payload) {
      const message = Array.isArray(payload)
        ? { __aircore: true, kind: 'ui', slot: 'page', tree: payload }
        : {
            __aircore: true,
            kind: 'ui',
            slot: payload?.slot || 'page',
            title: payload?.title,
            tree: payload?.tree || [],
          }
      parent.postMessage(message, '*')
    },

    /** 调用宿主能力（返回 Promise） */
    call(method, params) {
      const reqId = ++reqSeq
      return new Promise((resolve, reject) => {
        pending.set(reqId, { resolve, reject })
        parent.postMessage({ __aircore: true, kind: 'api', reqId, method, params }, '*')
      })
    },

    /** 监听宿主事件 */
    on(event, handler) {
      if (!listeners.has(event)) listeners.set(event, new Set())
      listeners.get(event).add(handler)
    },

    /** 读取 zip 内的图片，返回 data URL（可直接给 Image / canvas 用） */
    image(path) {
      return api.call('asset.image', { path })
    },

    /** 复制文本到剪贴板 */
    copy(text) {
      return api.call('clipboard.write', { text })
    },

    /** 用系统浏览器打开链接（仅 http/https） */
    openUrl(url) {
      return api.call('open.url', { url })
    },

    notify(message, type) {
      return api.call('notify', { message, type })
    },

    /** 网络请求（需 manifest permissions 包含 "network"）。
     *  返回 { status, body }，body 为文本。 */
    fetch(url, options) {
      return api.call('net.fetch', {
        url,
        method: options?.method,
        headers: options?.headers,
        body: options?.body,
        timeoutMs: options?.timeoutMs,
      })
    },

    /** 读取 zip 内的资源文件，返回 Uint8Array（用于 wasm / 图片 / 字体） */
    async asset(path) {
      const base64 = await api.call('asset.read', { path })
      const binary = atob(base64 || '')
      const bytes = new Uint8Array(binary.length)
      for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i)
      return bytes
    },

    /** 请求并解析 JSON */
    async fetchJson(url, options) {
      const response = await api.call('net.fetch', {
        url,
        method: options?.method,
        headers: options?.headers,
        body: options?.body,
        timeoutMs: options?.timeoutMs,
      })
      if (response.status < 200 || response.status >= 300) {
        throw new Error('HTTP ' + response.status)
      }
      return JSON.parse(response.body)
    },
  }

  // 把插件内的未捕获错误回传给宿主，方便在「插件日志」里看到
  window.addEventListener('error', (event) => {
    parent.postMessage(
      {
        __aircore: true,
        kind: 'plugin.error',
        message: String(event.message || 'script error'),
        stack: event.error && event.error.stack ? String(event.error.stack) : '',
      },
      '*',
    )
  })
  window.addEventListener('unhandledrejection', (event) => {
    const reason = event.reason
    parent.postMessage(
      {
        __aircore: true,
        kind: 'plugin.error',
        message: reason && reason.message ? String(reason.message) : String(reason),
        stack: reason && reason.stack ? String(reason.stack) : '',
      },
      '*',
    )
  })

  window.addEventListener('message', (event) => {
    const data = event.data
    if (!data || data.__aircore !== true) return

    if (data.kind === 'api.response') {
      const entry = pending.get(data.reqId)
      if (!entry) return
      pending.delete(data.reqId)
      if (data.error) entry.reject(new Error(data.error))
      else entry.resolve(data.result)
      return
    }

    if (data.kind === 'event') {
      for (const handler of listeners.get(data.id) ?? []) {
        try {
          handler(data.payload)
        } catch (error) {
          console.error('[plugin] event handler failed', error)
        }
      }
      return
    }

    if (data.kind === 'host.event') {
      for (const handler of listeners.get(data.name) ?? []) {
        try {
          handler(data.payload)
        } catch (error) {
          console.error('[plugin] host event failed', error)
        }
      }
    }
  })

  window.AirCore = api
  parent.postMessage({ __aircore: true, kind: 'ready' }, '*')
})()
`

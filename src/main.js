import './style.css'

// 判断是否运行在 Tauri 环境
const isTauri = typeof window !== 'undefined' && '__TAURI_IPC__' in window

let invoke = null
if (isTauri) {
  const { invoke: _invoke } = await import('@tauri-apps/api/tauri')
  invoke = _invoke
}

// ---------------- 状态 ----------------
const state = {
  folder: '',
  categories: [],
  activeCategory: 'all', // 'all' | 'fav' | 分类名
  search: '',
  favorites: [], // [{ name, path, kind, icon, category }]
  autoLoad: true,
  theme: 'light', // light | dark
  clickNormal: 'single', // 正常按钮启动方式：single | double
  clickWeak: 'single', // 灰色弱化按钮启动方式：single | double
}

// 主题先用本地镜像立即应用（避免启动瞬间闪白）；权威值稍后从用户配置读取
try {
  document.documentElement.dataset.theme = localStorage.getItem('launcher-theme') === 'dark' ? 'dark' : 'light'
} catch {}

// ---------------- DOM ----------------
const els = {
  nav: document.getElementById('nav'),
  pathText: document.getElementById('path-text'),
  pathDisplay: document.getElementById('path-display'),
  browseBtn: document.getElementById('browse-btn'),
  settingsBtn: document.getElementById('settings-btn'),
  refreshBtn: document.getElementById('refresh-btn'),
  searchBox: document.getElementById('search-box'),
  searchInput: document.getElementById('search-input'),
  content: document.getElementById('content'),
  grid: document.getElementById('grid'),
  status: document.getElementById('status'),
  empty: document.getElementById('empty'),
  sectionTitle: document.getElementById('section-title'),
  sidebar: document.getElementById('sidebar'),
  collapseBtn: document.getElementById('collapse-btn'),
  settingsPage: document.getElementById('settings-page'),
  settingsBack: document.getElementById('settings-back'),
  autoLoadToggle: document.getElementById('auto-load-toggle'),
  configPathText: document.getElementById('config-path-text'),
  dataDirText: document.getElementById('data-dir-text'),
  openConfigLoc: document.getElementById('open-config-loc'),
  openDataLoc: document.getElementById('open-data-loc'),
  settingsFooter: document.getElementById('settings-footer'),
  contextMenu: document.getElementById('context-menu'),
  hintToast: document.getElementById('hint-toast'),
  themeSeg: document.getElementById('theme-seg'),
  clickNormalSeg: document.getElementById('click-normal-seg'),
  clickWeakSeg: document.getElementById('click-weak-seg'),
  clearCacheBtn: document.getElementById('clear-cache-btn'),
  cacheDesc: document.getElementById('cache-desc'),
  modalMask: document.getElementById('modal-mask'),
  modalTitle: document.getElementById('modal-title'),
  modalBody: document.getElementById('modal-body'),
  modalOk: document.getElementById('modal-ok'),
  modalCancel: document.getElementById('modal-cancel'),
}

// 诊断计数（供自动化验证读取，不影响功能）
window.__launcherStats = { applies: 0, bgVerifies: 0, bgUpdated: 0 }

// ---------------- 演示数据（浏览器预览用；均为中性示例名） ----------------
const DEMO = [
  {
    name: '办公',
    path: 'D:/软件库/办公',
    apps: [
      { name: '设计套件', path: '', items: [
        { name: '图像处理', path: '', ext: 'lnk', kind: 'app', noise: false },
        { name: '视频剪辑', path: '', ext: 'lnk', kind: 'app', noise: false },
        { name: '特效合成', path: '', ext: 'lnk', kind: 'app', noise: false },
      ]},
      { name: '三维设计', path: '', items: [
        { name: '三维建模工具', path: '', ext: 'lnk', kind: 'app', noise: false },
      ]},
      { name: '开发工具', path: '', items: [
        { name: '集成开发环境', path: '', ext: 'lnk', kind: 'app', noise: false },
        { name: '代码编辑器', path: '', ext: 'lnk', kind: 'app', noise: false },
        { name: '命令行终端', path: '', ext: 'lnk', kind: 'app', noise: false },
      ]},
      { name: '办公套件', path: '', items: [
        { name: '文档办公', path: '', ext: 'lnk', kind: 'app', noise: false },
        { name: '配置工具', path: '', ext: 'lnk', kind: 'app', noise: true },
      ]},
    ],
    loose_items: [],
  },
  {
    name: '工具',
    path: 'D:/软件库/工具',
    apps: [
      { name: '文件搜索', path: '', items: [
        { name: '搜索工具', path: '', ext: 'lnk', kind: 'app', noise: false },
        { name: '搜索栏插件', path: '', ext: 'lnk', kind: 'app', noise: false },
      ]},
      { name: '快速启动', path: '', items: [
        { name: '快速启动工具', path: '', ext: 'lnk', kind: 'app', noise: false },
        { name: '卸载程序', path: '', ext: 'lnk', kind: 'app', noise: true },
      ]},
      { name: '系统增强', path: '', items: [
        { name: '系统增强工具', path: '', ext: 'lnk', kind: 'app', noise: false },
      ]},
      { name: '录屏工具', path: '', items: [
        { name: '录屏软件', path: '', ext: 'lnk', kind: 'app', noise: false },
      ]},
    ],
    loose_items: [],
  },
  {
    name: '游戏',
    path: 'D:/软件库/游戏',
    apps: [
      { name: '沙盒建造', path: '', items: [
        { name: '游戏启动器', path: '', ext: 'lnk', kind: 'app', noise: false },
        { name: '社区启动器', path: '', ext: 'lnk', kind: 'app', noise: false },
      ]},
      { name: '示例游戏', path: '', items: [
        { name: '启动示例游戏', path: '', ext: 'lnk', kind: 'app', noise: false },
        { name: '卸载示例游戏', path: '', ext: 'lnk', kind: 'app', noise: true },
      ]},
    ],
    loose_items: [
      { name: '游戏平台', path: '', ext: 'url', kind: 'url', noise: false },
      { name: '独立游戏', path: '', ext: 'url', kind: 'url', noise: false },
    ],
  },
  {
    name: '软件',
    path: 'D:/软件库/软件',
    apps: [
      { name: '浏览器', path: '', items: [
        { name: '浏览器主程序', path: '', ext: 'lnk', kind: 'app', noise: false },
      ]},
      { name: '音乐播放器', path: '', items: [
        { name: '音乐播放器', path: '', ext: 'lnk', kind: 'app', noise: false },
        { name: '卸载程序', path: '', ext: 'lnk', kind: 'app', noise: true },
      ]},
      { name: '即时通讯', path: '', items: [
        { name: '通讯客户端', path: '', ext: 'lnk', kind: 'app', noise: false },
      ]},
    ],
    loose_items: [],
  },
]

// ---------------- 图标映射 ----------------
const ICON_BY_KIND = { app: '🚀', folder: '📁', url: '🔗', script: '⚙️', file: '📄' }
const CATEGORY_ICONS = { 办公: '💼', 工具: '🛠️', 游戏: '🎮', 游戏工具: '🎲', 软件: '📦', 快速通道: '⚡', 学习: '📚', '还未安装的应用（待办）': '📋' }

function itemIcon(name, kind) {
  const n = name.toLowerCase()
  // 通用词形 → 示意图标
  const map = [
    ['卸载', '🗑️'], ['uninstall', '🗑️'],
    ['图像', '🎨'], ['视频', '🎬'], ['特效', '✨'], ['三维', '🧊'],
    ['代码', '💻'], ['开发', '💡'], ['终端', '⌨️'], ['文档', '📝'], ['配置', '⚙️'],
    ['搜索', '🔍'], ['快速', '⚡'], ['系统', '🛠️'], ['录屏', '🎥'],
    ['启动器', '🎮'], ['游戏', '🎮'], ['浏览器', '🌐'], ['音乐', '🎵'], ['通讯', '💬'],
  ]
  for (const [k, e] of map) if (n.includes(k)) return e
  return ICON_BY_KIND[kind] || '📄'
}

function catIcon(name) {
  return CATEGORY_ICONS[name] || '📁'
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]))
}

// ---------------- 收藏辅助 ----------------
function isFav(name, path) {
  return state.favorites.some((f) => f.name === name && f.path === path)
}

async function toggleFavorite(item) {
  if (!isTauri) {
    // 预览模式：仅内存操作
    const idx = state.favorites.findIndex((f) => f.name === item.name && f.path === item.path)
    if (idx >= 0) state.favorites.splice(idx, 1)
    else state.favorites.push(item)
    afterFavChange(item)
    return
  }
  try {
    let favs
    if (isFav(item.name, item.path)) {
      favs = await invoke('cmd_remove_favorite', { name: item.name, path: item.path })
      setStatus(`已取消收藏「${item.name}」`, 'ok')
    } else {
      favs = await invoke('cmd_add_favorite', { item })
      setStatus(`已收藏「${item.name}」`, 'ok')
    }
    state.favorites = favs || []
    afterFavChange(item)
  } catch (e) {
    setStatus(`收藏操作失败：${e}`, 'error')
  }
}

function afterFavChange() {
  renderNav()
  renderGrid()
}

// ---------------- 渲染侧栏 ----------------
function renderNav() {
  const cats = state.categories
  els.nav.innerHTML = ''

  // 「全部」入口
  const all = document.createElement('div')
  all.className = 'nav-item' + (state.activeCategory === 'all' ? ' active' : '')
  all.dataset.cat = 'all'
  all.innerHTML = `<span class="nav-icon">🏠</span><span class="nav-label">全部</span><span class="nav-count">${totalCount()}</span>`
  all.addEventListener('click', () => setCategory('all'))
  els.nav.appendChild(all)

  // 「收藏」入口
  const fav = document.createElement('div')
  fav.className = 'nav-item' + (state.activeCategory === 'fav' ? ' active' : '')
  fav.dataset.cat = 'fav'
  fav.innerHTML = `<span class="nav-icon">⭐</span><span class="nav-label">收藏</span><span class="nav-count">${state.favorites.length}</span>`
  fav.addEventListener('click', () => setCategory('fav'))
  els.nav.appendChild(fav)

  // 分隔
  const sep = document.createElement('div')
  sep.className = 'nav-sep'
  els.nav.appendChild(sep)

  for (const cat of cats) {
    const count = cat.apps.reduce((s, a) => s + a.items.length, 0) + cat.loose_items.length
    const item = document.createElement('div')
    item.className = 'nav-item' + (state.activeCategory === cat.name ? ' active' : '')
    item.dataset.cat = cat.name
    item.innerHTML = `<span class="nav-icon">${catIcon(cat.name)}</span><span class="nav-label" title="${escapeHtml(cat.name)}">${escapeHtml(cat.name)}</span><span class="nav-count">${count}</span>`
    item.addEventListener('click', () => setCategory(cat.name))
    els.nav.appendChild(item)
  }
}

function totalCount() {
  return state.categories.reduce((s, c) => s + c.apps.reduce((x, a) => x + a.items.length, 0) + c.loose_items.length, 0)
}

function setCategory(name) {
  state.activeCategory = name
  showContentView()
  renderNav()
  renderGrid()
}

// ---------------- 视图切换 ----------------
function showContentView() {
  els.content.style.display = ''
  els.settingsPage.style.display = 'none'
  els.searchBox.style.display = ''
  els.refreshBtn.style.display = ''
  els.pathDisplay.style.display = ''
}

function showSettingsView() {
  els.content.style.display = 'none'
  els.settingsPage.style.display = 'block'
  els.searchBox.style.display = 'none'
  els.refreshBtn.style.display = 'none'
  els.pathDisplay.style.display = 'none'
  // 清除侧栏选中态
  els.nav.querySelectorAll('.nav-item').forEach((n) => n.classList.remove('active'))
  void populateSettings()
}

// ---------------- 设置页 ----------------
let appInfo = null

// ---------------- 主题 / 分段控件 ----------------
/// 应用主题并写入本地镜像（镜像用于下次启动时避免闪白）
function applyTheme(theme) {
  state.theme = theme === 'dark' ? 'dark' : 'light'
  document.documentElement.dataset.theme = state.theme
  try {
    localStorage.setItem('launcher-theme', state.theme)
  } catch {}
  if (els.themeSeg) setSeg(els.themeSeg, state.theme)
}

/// 设置分段控件的选中项
function setSeg(seg, val) {
  if (!seg) return
  seg.querySelectorAll('button').forEach((b) => b.classList.toggle('on', b.dataset.val === val))
}

/// 绑定分段控件点击
function bindSeg(seg, onSelect) {
  if (!seg) return
  seg.querySelectorAll('button').forEach((b) => {
    b.addEventListener('click', () => onSelect(b.dataset.val))
  })
}

/// 填充设置页（配置文件路径、缓存路径、版本、开关状态、各项偏好）
async function populateSettings() {
  if (!isTauri) {
    els.configPathText.textContent = '（预览模式）launcher-config.json（与 exe 同目录）'
    els.dataDirText.textContent = '（预览模式）webview-data'
    els.settingsFooter.textContent = 'Folder Launchpad · 浏览器预览模式'
    els.autoLoadToggle.checked = state.autoLoad
    setSeg(els.themeSeg, state.theme)
    setSeg(els.clickNormalSeg, state.clickNormal)
    setSeg(els.clickWeakSeg, state.clickWeak)
    return
  }
  try {
    appInfo = await invoke('cmd_get_app_info')
    els.configPathText.textContent = appInfo.configPath || '（未获取到）'
    els.configPathText.title = appInfo.configPath || ''
    els.dataDirText.textContent = appInfo.dataDir || '（未获取到）'
    els.dataDirText.title = appInfo.dataDir || ''
    els.settingsFooter.textContent = `Folder Launchpad v${appInfo.version} · 免安装（数据保存在程序所在目录）`
    const s = await invoke('cmd_get_settings')
    state.autoLoad = s.auto_load !== false
    els.autoLoadToggle.checked = state.autoLoad
    state.clickNormal = s.click_normal === 'double' ? 'double' : 'single'
    state.clickWeak = s.click_weak === 'double' ? 'double' : 'single'
    applyTheme(s.theme === 'dark' ? 'dark' : 'light')
    setSeg(els.clickNormalSeg, state.clickNormal)
    setSeg(els.clickWeakSeg, state.clickWeak)
  } catch (e) {
    console.warn('读取设置信息失败', e)
  }
}

/// 在资源管理器中打开位置（file 为文件则定位选中，否则打开文件夹）
async function openLocation(path, isFile) {
  if (!isTauri || !path) return
  try {
    if (isFile) {
      await invoke('cmd_reveal_file', { path })
    } else {
      await invoke('cmd_open_folder', { path })
    }
    setStatus('已在资源管理器中打开', 'ok')
  } catch (e) {
    setStatus(`打开位置失败：${e}`, 'error')
  }
}

// ---------------- 渲染网格 ----------------
function renderGrid() {
  els.grid.innerHTML = ''
  cancelPendingLaunch() // 列表重绘时收起「再次点击」待启动状态

  // 收藏页：单独渲染（同一文件夹卡片下的多个收藏项合并为一张卡）
  if (state.activeCategory === 'fav') {
    els.sectionTitle.textContent = '收藏'
    if (state.favorites.length === 0) {
      els.grid.innerHTML = `<div class="fav-empty"><div class="big">⭐</div><p>还没有收藏任何项。在卡片或条目上右键即可收藏。</p></div>`
      els.empty.style.display = 'none'
      return
    }

    // 扫描数据索引：判断收藏项属于哪张文件夹卡片 / 给旧收藏兜底补图标
    const appByPath = new Map()
    const looseByPath = new Map()
    for (const cat of state.categories) {
      for (const app of cat.apps) appByPath.set(app.path.toLowerCase(), { app, cat })
      for (const it of cat.loose_items) if (it.path) looseByPath.set(it.path.toLowerCase(), it)
    }

    // 分组：所属文件夹卡片相同的收藏项 → 合并进同一张卡（组按首次收藏出现的位置排序）
    const groups = []
    const groupIdx = new Map()
    for (const f of state.favorites) {
      const p = f.path || ''
      const cut = Math.max(p.lastIndexOf('\\'), p.lastIndexOf('/'))
      const parent = cut > 0 ? p.slice(0, cut) : ''
      const hit = parent ? appByPath.get(parent.toLowerCase()) : null
      const key = hit ? 'app:' + hit.app.path.toLowerCase() : 'item:' + (p || f.name)
      let g = groupIdx.get(key)
      if (!g) {
        g = { app: hit ? hit.app : null, cat: hit ? hit.cat : null, favs: [] }
        groupIdx.set(key, g)
        groups.push(g)
      }
      g.favs.push(f)
    }

    for (const g of groups) {
      if (g.app) {
        // 文件夹卡片样式：芯片 = 该卡片下已收藏的启动项（顺序跟随主视图）
        const card = document.createElement('div')
        card.className = 'app-card is-fav'
        card.title = `${g.app.name}（${g.cat ? g.cat.name : '收藏'}）`
        let favIcon = ''
        for (const f of g.favs) { if (f.icon) { favIcon = f.icon; break } }
        const iconHtml = g.app.icon
          ? `<img src="${g.app.icon}" alt="" />`
          : (favIcon ? `<img src="${favIcon}" alt="" />` : itemIcon(g.app.name, 'app'))
        const ordered = g.app.items.filter((it) => g.favs.some((f) => f.name === it.name && f.path === it.path))
        for (const f of g.favs) {
          if (!ordered.some((it) => it.name === f.name && it.path === f.path)) ordered.push(f)
        }
        const chips = ordered.map((it) => {
          const noiseCls = it.noise ? ' noise' : ''
          return `<span class="chip ${it.kind}${noiseCls} is-fav" data-path="${escapeHtml(it.path)}" data-name="${escapeHtml(it.name)}" title="${escapeHtml(it.path || it.name)}"><span class="dot"></span>${escapeHtml(it.name)}<span class="fav-badge">⭐</span></span>`
        }).join('')
        card.innerHTML = `
          <div class="app-head">
            <div class="app-icon">${iconHtml}</div>
            <div class="app-name" title="${escapeHtml(g.app.name)}">${escapeHtml(g.app.name)}</div>
          </div>
          <div class="app-items">${chips}</div>
        `
        card.querySelectorAll('.chip').forEach((chip) => {
          const itName = chip.dataset.name
          const itPath = chip.dataset.path
          const isNoise = chip.classList.contains('noise')
          bindLaunch(chip, isNoise, () => onOpen(itPath, itName, card))
          // 芯片自己处理点击，不冒泡给整卡（仅单收藏卡存在整卡绑定，避免父子状态互相覆盖）
          chip.addEventListener('click', (e) => e.stopPropagation())
          chip.addEventListener('contextmenu', (e) => {
            e.preventDefault()
            e.stopPropagation()
            const kind = (chip.className.match(/chip (\w+)/) || [])[1] || 'app'
            showContextMenu(e.clientX, e.clientY, { name: itName, path: itPath, kind, category: g.cat ? g.cat.name : '' }, g.app)
          })
        })
        // 卡片本体点击：与普通卡片一致（默认不响应）；仅当该卡片只收藏 1 项时，整卡可点击启动这一项
        if (g.favs.length === 1 && g.favs[0].path) {
          const only = g.favs[0]
          bindLaunch(card, false, () => onOpen(only.path, only.name, card))
        }
        // 卡片本体右键：对象为卡片对应文件夹
        card.addEventListener('contextmenu', (e) => {
          e.preventDefault()
          showContextMenu(e.clientX, e.clientY, { name: g.app.name, path: g.app.path, kind: 'folder', category: g.cat ? g.cat.name : '' }, g.app)
        })
        els.grid.appendChild(card)
      } else {
        // 单个收藏项（散装项 / 收藏的软件文件夹本身）：保持单卡样式
        const f = g.favs[0]
        const card = document.createElement('div')
        card.className = 'app-card is-fav'
        card.title = f.path || f.name
        let icon = f.icon
        if (!icon && f.path) {
          // 旧收藏兜底：从扫描数据里补图标
          const exact = appByPath.get(f.path.toLowerCase())
          const loose = looseByPath.get(f.path.toLowerCase())
          if (exact && exact.app.icon) icon = exact.app.icon
          else if (loose && loose.icon) icon = loose.icon
        }
        const iconHtml = icon ? `<img src="${icon}" alt="" />` : itemIcon(f.name, f.kind)
        card.innerHTML = `
          <div class="app-head">
            <div class="app-icon">${iconHtml}</div>
            <div class="app-name" title="${escapeHtml(f.name)}">${escapeHtml(f.name)}</div>
          </div>
          <div class="app-items">
            <span class="chip ${f.kind}"><span class="dot"></span>${escapeHtml(f.category || '收藏')}</span>
          </div>
        `
        bindLaunch(card, false, () => onOpen(f.path, f.name, card))
        card.addEventListener('contextmenu', (e) => {
          e.preventDefault()
          showContextMenu(e.clientX, e.clientY, f, null)
        })
        els.grid.appendChild(card)
      }
    }
    els.empty.style.display = 'none'
    return
  }

  const q = state.search.trim().toLowerCase()

  let cats = state.categories
  if (state.activeCategory !== 'all') {
    cats = cats.filter((c) => c.name === state.activeCategory)
  }

  els.sectionTitle.textContent = state.activeCategory === 'all' ? '全部软件' : state.activeCategory

  let total = 0

  for (const cat of cats) {
    // 软件卡片
    for (const app of cat.apps) {
      const items = app.items.filter((it) => !q || it.name.toLowerCase().includes(q))
      if (items.length === 0) continue
      total += items.length
      const card = document.createElement('div')
      card.className = 'app-card'
      card.title = `${app.name}（${cat.name}）`

      const iconHtml = app.icon
        ? `<img src="${app.icon}" alt="" />`
        : itemIcon(app.name, 'app')

      const chips = items.map((it) => {
        const noiseCls = it.noise ? ' noise' : ''
        const favCls = isFav(it.name, it.path) ? ' is-fav' : ''
        const star = isFav(it.name, it.path) ? '<span class="fav-badge">⭐</span>' : ''
        return `<span class="chip ${it.kind}${noiseCls}${favCls}" data-path="${escapeHtml(it.path)}" data-name="${escapeHtml(it.name)}" title="${escapeHtml(it.path || it.name)}"><span class="dot"></span>${escapeHtml(it.name)}${star}</span>`
      }).join('')

      card.innerHTML = `
        <div class="app-head">
          <div class="app-icon">${iconHtml}</div>
          <div class="app-name" title="${escapeHtml(app.name)}">${escapeHtml(app.name)}</div>
        </div>
        <div class="app-items">${chips}</div>
      `
      // 芯片：按「灰色弱化按钮 / 正常按钮」各自的点击设置启动
      card.querySelectorAll('.chip').forEach((chip) => {
        const itName = chip.dataset.name
        const itPath = chip.dataset.path
        const isNoise = chip.classList.contains('noise')
        bindLaunch(chip, isNoise, () => onOpen(itPath, itName, chip.closest('.app-card') || chip))
        chip.addEventListener('contextmenu', (e) => {
          e.preventDefault()
          e.stopPropagation()
          const kind = (chip.className.match(/chip (\w+)/) || [])[1] || 'app'
          showContextMenu(e.clientX, e.clientY, { name: itName, path: itPath, kind, category: cat.name }, app)
        })
      })
      // 右键卡片本体（软件选项区域之外）：操作对象为「卡片对应的文件夹」
      card.addEventListener('contextmenu', (e) => {
        e.preventDefault()
        showContextMenu(e.clientX, e.clientY, { name: app.name, path: app.path, kind: 'folder', category: cat.name }, app)
      })
      els.grid.appendChild(card)
    }

    // 散装启动项
    for (const it of cat.loose_items) {
      if (q && !it.name.toLowerCase().includes(q)) continue
      total++
      const favCls = isFav(it.name, it.path) ? ' is-fav' : ''
      const star = isFav(it.name, it.path) ? '<span class="fav-badge">⭐</span>' : ''
      const iconHtml = it.icon ? `<img src="${it.icon}" alt="" />` : itemIcon(it.name, it.kind)
      const node = document.createElement('div')
      node.className = 'loose-item' + (it.noise ? ' noise' : '') + favCls
      node.innerHTML = `<span class="loose-icon">${iconHtml}</span><span class="loose-name" title="${escapeHtml(it.name)}">${escapeHtml(it.name)}</span>${star}`
      bindLaunch(node, it.noise, () => onOpen(it.path, it.name, node))
      node.addEventListener('contextmenu', (e) => {
        e.preventDefault()
        showContextMenu(e.clientX, e.clientY, { name: it.name, path: it.path, kind: it.kind, category: cat.name, icon: it.icon || '' }, null)
      })
      els.grid.appendChild(node)
    }
  }

  if (total === 0) {
    els.empty.style.display = 'flex'
    els.empty.querySelector('p').textContent = q
      ? '没有匹配的项'
      : (state.categories.length === 0 ? '选择一个文件夹，加载你的软件库' : '该分类暂无内容')
  } else {
    els.empty.style.display = 'none'
  }
}

// ---------------- 右键菜单 ----------------
/// 定位菜单（防溢出）
function positionMenu(x, y) {
  const menu = els.contextMenu
  menu.style.display = 'block'
  const mw = menu.offsetWidth
  const mh = menu.offsetHeight
  const px = Math.min(x, window.innerWidth - mw - 8)
  const py = Math.min(y, window.innerHeight - mh - 8)
  menu.style.left = px + 'px'
  menu.style.top = py + 'px'
}

/// 菜单里的路径类操作（在资源管理器中打开 / 打开文件位置 / 属性）
async function runPathAction(cmd, path, okMsg) {
  if (!isTauri) {
    setStatusAnimated('预览模式：该操作在打包版中可用', 'ok')
    return
  }
  if (!path) {
    setStatusAnimated('该条目没有可用路径', 'error')
    return
  }
  try {
    await invoke(cmd, { path })
    setStatusAnimated(okMsg, 'ok')
  } catch (e) {
    setStatusAnimated(`操作失败：${e}`, 'error')
  }
}

/// 软件项 / 卡片共用的右键菜单
function showContextMenu(x, y, item, app) {
  const menu = els.contextMenu
  cancelPendingLaunch() // 右键视为取消「再次点击」待启动状态
  const faved = isFav(item.name, item.path)

  const iconHtml = (app && app.icon) ? app.icon : (item.icon || null)
  const favPayload = {
    name: item.name,
    path: item.path,
    kind: item.kind || 'app',
    icon: iconHtml || null,
    category: item.category || '',
  }

  menu.innerHTML = `
    <div class="ctx-item" data-act="open">▶ 打开</div>
    <div class="ctx-item" data-act="fav">${faved ? '⭐ 取消收藏' : '☆ 收藏'}</div>
    <div class="ctx-sep"></div>
    <div class="ctx-item" data-act="explorer" title="打开该文件夹；若是文件，则打开它所在的文件夹">📂 在资源管理器中打开</div>
    <div class="ctx-item" data-act="reveal" title="打开它上一级所在的文件夹（快捷方式也只看它自身的位置）">📍 打开文件位置</div>
    <div class="ctx-item" data-act="origin" title="打开快捷方式指向的原软件位置；非快捷方式则打开其自身位置">🎯 打开原文件位置</div>
    <div class="ctx-item" data-act="props">ℹ️ 属性</div>
    <div class="ctx-sep"></div>
    <div class="ctx-item" data-act="copy">📋 复制路径</div>
  `

  positionMenu(x, y)

  menu.querySelectorAll('.ctx-item').forEach((node) => {
    node.addEventListener('click', async () => {
      const act = node.dataset.act
      hideContextMenu()
      if (act === 'open') {
        // 文件夹对象（卡片本体右键）→ 在资源管理器中打开；软件项 → 启动
        if (favPayload.kind === 'folder') {
          await runPathAction('cmd_open_in_explorer', favPayload.path, `已打开「${favPayload.name}」`)
        } else {
          onOpen(favPayload.path, favPayload.name)
        }
      } else if (act === 'fav') {
        // 收藏卡片时优先用软件级图标
        await toggleFavorite({ ...favPayload, icon: favPayload.icon })
      } else if (act === 'explorer') {
        // #1 在资源管理器中打开：文件夹 → 进入本身；文件 → 打开所在文件夹
        await runPathAction('cmd_open_in_explorer', favPayload.path, '已在资源管理器中打开')
      } else if (act === 'reveal') {
        // #2 打开文件位置：打开它上一级所在的文件夹（不解析快捷方式）
        await runPathAction('cmd_open_file_location', favPayload.path, '已打开上一级所在位置')
      } else if (act === 'origin') {
        // #3 打开原文件位置：快捷方式 → 其指向的原软件位置；否则自身位置
        await runPathAction('cmd_open_original_location', favPayload.path, '已打开原文件位置')
      } else if (act === 'props') {
        await runPathAction('cmd_show_properties', favPayload.path, '已打开属性面板')
      } else if (act === 'copy') {
        try {
          await navigator.clipboard.writeText(favPayload.path)
          setStatus('已复制路径', 'ok')
        } catch {
          setStatus('复制失败', 'error')
        }
      }
    })
  })
}

/// 空白区域（卡片以外）的右键菜单：刷新 / 打开当前路径 / 打开总目录路径
function showEmptyMenu(x, y) {
  const menu = els.contextMenu
  cancelPendingLaunch()
  menu.innerHTML = `
    <div class="ctx-item" data-act="refresh">🔄 刷新</div>
    <div class="ctx-sep"></div>
    <div class="ctx-item" data-act="open-current">📂 打开当前路径</div>
    <div class="ctx-item" data-act="open-root">🏠 打开总目录路径</div>
  `

  positionMenu(x, y)

  menu.querySelectorAll('.ctx-item').forEach((node) => {
    node.addEventListener('click', async () => {
      const act = node.dataset.act
      hideContextMenu()
      if (act === 'refresh') {
        // 刷新当前打开的视图（保持当前分类，不回到「全部」）
        await refresh()
      } else if (act === 'open-current') {
        const p = currentFolderPath()
        if (!p) {
          setStatusAnimated('尚未选择文件夹', 'error')
          return
        }
        await runPathAction('cmd_open_folder', p, `已打开「${p}」`)
      } else if (act === 'open-root') {
        if (!state.folder) {
          setStatusAnimated('尚未选择文件夹', 'error')
          return
        }
        await runPathAction('cmd_open_folder', state.folder, `已打开「${state.folder}」`)
      }
    })
  })
}

/// 当前视图对应的文件夹路径：分类视图 → 该分类文件夹；其余（全部/收藏）→ 根目录
function currentFolderPath() {
  if (state.activeCategory !== 'all' && state.activeCategory !== 'fav') {
    const cat = state.categories.find((c) => c.name === state.activeCategory)
    if (cat && cat.path) return cat.path
  }
  return state.folder || ''
}

function hideContextMenu() {
  els.contextMenu.style.display = 'none'
}

document.addEventListener('click', hideContextMenu)
document.addEventListener('scroll', hideContextMenu, true)
window.addEventListener('resize', hideContextMenu)

// 全局右键：屏蔽 WebView2 原生菜单；空白区域弹出简化菜单
// （芯片 / 卡片 / 散装项 / 收藏卡在各自的 contextmenu 监听里已 preventDefault 并弹对应菜单）
document.addEventListener('contextmenu', (e) => {
  if (e.defaultPrevented) return
  e.preventDefault()
  if (e.target.closest('.context-menu') || e.target.closest('.modal-mask')) return
  showEmptyMenu(e.clientX, e.clientY)
})

// ---------------- 启动反馈 ----------------
const LAUNCH_FLASH_MS = 2600
const launchTimers = new WeakMap()
const prefersReducedMotion =
  typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches

/// 状态栏淡入：仅用于点击启动等即时反馈，避免扫描类调用导致全局闪烁
function setStatusAnimated(msg, type = 'ok') {
  setStatus(msg, type)
  if (!prefersReducedMotion && els.status.animate) {
    els.status.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 220, easing: 'ease-out' })
  }
}

/// 给被点击元素加「已启动」高亮；可重复触发（重启计时与动画）
function flashLaunched(el) {
  if (!el) return
  const prev = launchTimers.get(el)
  if (prev) clearTimeout(prev)
  el.classList.remove('just-launched')
  void el.offsetWidth // 强制重排，保证重新加类时动画从头播放
  el.classList.add('just-launched')
  launchTimers.set(el, setTimeout(() => {
    el.classList.remove('just-launched')
    launchTimers.delete(el)
  }, LAUNCH_FLASH_MS))
}

// ---------------- 启动点击方式（单击 / 双击） ----------------
const LAUNCH_GUARD_MS = 450
const lastLaunchAt = new WeakMap()
const DOUBLE_HINT_DELAY_MS = 300 // 快速两次点击的判定窗口：窗口内直接启动，不弹提示
const DOUBLE_GRACE_MS = 1900 // 提示出现后的宽限窗口：再次点击仍会启动
const HINT_VISIBLE_MS = 1500 // 小提示自动消失时间

/// 当前生效的点击模式（weak = 灰色弱化按钮，用独立设置）
function clickModeOf(weak) {
  return weak ? state.clickWeak : state.clickNormal
}

/// 双击模式下的「待启动」状态与小提示
let pendingLaunch = null
let hintHideTimer = null

/// 按「单击/双击」设置绑定启动：
/// - 单击模式：点一下即启动（快速双击只算一次，防连发）
/// - 双击模式：快速两次点击启动；若只点了一次，先弹「再次点击即可启动」小提示，
///   宽限窗口内再次点击同样会启动（提示出现前后行为一致，避免“点了没反应”）
function bindLaunch(el, weak, fire) {
  el.addEventListener('click', () => {
    if (clickModeOf(weak) === 'single') {
      guardedLaunch(el, fire)
      return
    }
    // 刚刚启动过的余波点击（如三连击的第三次）直接忽略
    if (Date.now() - (lastLaunchAt.get(el) || 0) < LAUNCH_GUARD_MS) return
    // 同一元素上的第二次点击 → 启动
    if (pendingLaunch && pendingLaunch.el === el) {
      cancelPendingLaunch()
      guardedLaunch(el, fire)
      return
    }
    // 第一次点击：等待快速第二次点击；超过窗口则弹顶栏提示并继续等待（宽限期内点击同样启动）
    cancelPendingLaunch()
    const p = { el }
    p.hintTimer = setTimeout(() => {
      if (pendingLaunch === p) showLaunchHint('再次点击即可启动')
    }, DOUBLE_HINT_DELAY_MS)
    p.endTimer = setTimeout(() => {
      if (pendingLaunch === p) cancelPendingLaunch()
    }, DOUBLE_GRACE_MS)
    pendingLaunch = p
  })
}

/// 取消待启动状态并收起小提示（不影响已发生的启动）
function cancelPendingLaunch() {
  if (pendingLaunch) {
    clearTimeout(pendingLaunch.hintTimer)
    clearTimeout(pendingLaunch.endTimer)
    pendingLaunch = null
  }
  hideLaunchHint()
}

/// 从顶栏下方居中弹出一个轻提示（如「再次点击即可启动」）
function showLaunchHint(text) {
  const el = els.hintToast
  if (!el) return
  el.textContent = text
  // 贴住顶栏下沿、水平对齐顶栏中心（顶栏尺寸变化时同样适用）
  const bar = document.querySelector('.topbar')
  if (bar) {
    const r = bar.getBoundingClientRect()
    el.style.left = Math.round(r.left + r.width / 2) + 'px'
    el.style.top = Math.round(r.bottom + 10) + 'px'
  }
  el.classList.add('show')
  clearTimeout(hintHideTimer)
  hintHideTimer = setTimeout(hideLaunchHint, HINT_VISIBLE_MS)
}

function hideLaunchHint() {
  clearTimeout(hintHideTimer)
  if (els.hintToast) els.hintToast.classList.remove('show')
}

/// 防抖：单击模式下快速双击会连发两次 click，这里保证短时间只触发一次
function guardedLaunch(el, fire) {
  const t = lastLaunchAt.get(el) || 0
  if (Date.now() - t < LAUNCH_GUARD_MS) return
  lastLaunchAt.set(el, Date.now())
  fire()
}

// ---------------- 动作 ----------------
async function onOpen(path, name, el = null) {
  if (!isTauri) {
    setStatusAnimated(`预览模式：将启动「${name}」`, 'ok')
    return
  }
  if (!path) {
    setStatusAnimated(`「${name}」没有可用的启动路径`, 'error')
    return
  }
  try {
    await invoke('cmd_open_item', { path })
    setStatusAnimated(`已启动「${name}」`, 'ok')
    flashLaunched(el) // 成功后才高亮
  } catch (e) {
    setStatusAnimated(`启动失败：${e}`, 'error')
  }
}

function updatePathDisplay() {
  els.pathText.textContent = state.folder || '尚未选择文件夹'
  els.pathDisplay.title = state.folder || '当前加载的文件夹'
}

/// 扫描结果的轻量签名：判断「后台校验结果」与「缓存」内容是否一致（不一致才更新界面）
function scanSignature(res) {
  if (!res || !Array.isArray(res.categories)) return 'null'
  const parts = [String(res.total_items)]
  for (const c of res.categories) {
    parts.push('C:' + c.name, 'a' + (c.apps || []).length, 'l' + (c.loose_items || []).length)
    for (const a of c.apps || []) {
      parts.push(a.name)
      for (const it of a.items || []) parts.push(it.path)
    }
    for (const it of c.loose_items || []) parts.push(it.path)
  }
  return parts.join('\u0001')
}

/// 把扫描结果应用到界面（首次扫描、缓存秒开、后台更新共用同一入口）
function applyScanResult(folder, res) {
  state.folder = folder
  state.categories = (res && res.categories) || []
  // 当前分类已消失（被改名/删除）时回退到「全部」，避免停留在空视图
  if (
    state.activeCategory !== 'all' &&
    state.activeCategory !== 'fav' &&
    !state.categories.some((c) => c.name === state.activeCategory)
  ) {
    state.activeCategory = 'all'
  }
  updatePathDisplay()
  setStatus(`已加载 ${res.total_items} 个启动项，${state.categories.length} 个分类`, 'ok')
  renderNav()
  renderGrid()
  window.__launcherStats.applies++
}

/// 缓存秒开后：后台静默重扫，内容有变化才更新界面（无变化时界面纹丝不动）
async function verifyScanInBackground(folder, cached) {
  window.__launcherStats.bgVerifies++
  try {
    const fresh = await invoke('cmd_scan_folder', { folder })
    if (state.folder !== folder) return // 校验期间已切换文件夹 → 丢弃旧结果，避免界面跳回
    if (scanSignature(fresh) !== scanSignature(cached)) {
      window.__launcherStats.bgUpdated++
      applyScanResult(folder, fresh)
    }
  } catch (e) {
    setStatus(`扫描失败：${e}（当前显示的是上次缓存）`, 'error')
  }
}

/// 扫描并加载指定文件夹
async function scanAndLoad(folder, { silent = false } = {}) {
  if (!isTauri) {
    state.folder = folder
    state.categories = DEMO
    state.activeCategory = 'all'
    updatePathDisplay()
    setStatus(`预览模式：已加载演示数据（Tauri 环境将扫描「${folder}」）`, 'ok')
    renderNav()
    renderGrid()
    return
  }
  try {
    if (!silent) setStatus(`正在扫描「${folder}」…`, 'ok')
    els.refreshBtn.classList.remove('spin')
    void els.refreshBtn.offsetWidth // 重排以重启动画
    els.refreshBtn.classList.add('spin') // 扫描期间持续旋转，作为加载指示
    const res = await invoke('cmd_scan_folder', { folder })
    await invoke('cmd_save_folder', { folder })
    applyScanResult(folder, res)
  } catch (e) {
    setStatus(`扫描失败：${e}`, 'error')
  } finally {
    els.refreshBtn.classList.remove('spin')
  }
}

/// 点击「选择文件夹」→ 弹出系统文件夹选择窗口
async function pickFolder() {
  if (!isTauri) {
    setStatus('预览模式：无法打开系统选择窗口（打包为 Tauri 应用后可用）', 'ok')
    return
  }
  try {
    const folder = await invoke('cmd_pick_folder')
    if (!folder) {
      setStatus('已取消选择', 'ok')
      return
    }
    state.activeCategory = 'all'
    showContentView()
    await scanAndLoad(folder)
  } catch (e) {
    setStatus(`选择文件夹失败：${e}`, 'error')
  }
}

/// 刷新：重新扫描当前文件夹（旋转动画由 scanAndLoad 统一管理）
async function refresh() {
  if (!state.folder) {
    setStatus('尚未选择文件夹', 'error')
    return
  }
  await scanAndLoad(state.folder)
}

function setStatus(msg, type = 'ok') {
  els.status.textContent = msg
  els.status.className = `status ${type}`
}

// ---------------- 事件 ----------------
els.browseBtn.addEventListener('click', pickFolder)
els.settingsBtn.addEventListener('click', showSettingsView)
els.settingsBack.addEventListener('click', () => {
  // 返回时保持当前视图（「全部」/「收藏」/具体分类），不强制跳回「全部」
  showContentView()
  renderNav()
  renderGrid()
})
els.refreshBtn.addEventListener('click', refresh)

// 设置页：自动加载开关
els.autoLoadToggle.addEventListener('change', async (e) => {
  const enabled = e.target.checked
  if (!isTauri) {
    state.autoLoad = enabled
    return
  }
  try {
    await invoke('cmd_set_auto_load', { enabled })
    state.autoLoad = enabled
    setStatus(enabled ? '已开启：启动时自动加载上次文件夹' : '已关闭：启动时不再自动加载', 'ok')
  } catch (err) {
    setStatus(`设置保存失败：${err}`, 'error')
    e.target.checked = !enabled
  }
})

// 设置页：主题（亮色 / 暗色，立即生效并持久化）
bindSeg(els.themeSeg, async (val) => {
  applyTheme(val)
  if (!isTauri) return
  try {
    await invoke('cmd_set_theme', { theme: state.theme })
  } catch (e) {
    setStatus(`主题保存失败：${e}`, 'error')
  }
})

// 设置页：启动点击次数（正常按钮 / 灰色按钮 两项独立设置）
bindSeg(els.clickNormalSeg, async (val) => {
  state.clickNormal = val
  setSeg(els.clickNormalSeg, val)
  if (!isTauri) return
  try {
    await invoke('cmd_set_click_mode', { target: 'normal', mode: val })
    setStatus(`正常按钮已设为：${val === 'double' ? '双击' : '单击'}启动`, 'ok')
  } catch (e) {
    setStatus(`设置保存失败：${e}`, 'error')
  }
})

bindSeg(els.clickWeakSeg, async (val) => {
  state.clickWeak = val
  setSeg(els.clickWeakSeg, val)
  if (!isTauri) return
  try {
    await invoke('cmd_set_click_mode', { target: 'weak', mode: val })
    setStatus(`灰色按钮已设为：${val === 'double' ? '双击' : '单击'}启动`, 'ok')
  } catch (e) {
    setStatus(`设置保存失败：${e}`, 'error')
  }
})

// 设置页：清理缓存（二次确认后才执行）
els.clearCacheBtn.addEventListener('click', async () => {
  const ok = await showConfirm({
    title: '确定要清理缓存吗？',
    body: '将删除：\n· 扫描缓存（下次启动会重新扫描，首屏稍慢一点）\n· 浏览器内核磁盘缓存（自动重建）\n\n你的配置、收藏和文件夹设置不受影响。',
    okText: '清理',
  })
  if (!ok) return
  if (!isTauri) {
    setStatus('预览模式：跳过清理', 'ok')
    return
  }
  els.clearCacheBtn.disabled = true
  try {
    const r = await invoke('cmd_clear_cache')
    const text = r.freed_bytes >= 1048576
      ? `${(r.freed_bytes / 1048576).toFixed(1)} MB`
      : `${Math.round(r.freed_bytes / 1024)} KB`
    const note = r.skipped > 0 ? `（${r.skipped} 项正在使用中，未清理）` : ''
    setStatus(`已清理缓存，释放 ${text}${note}`, 'ok')
    if (els.cacheDesc) els.cacheDesc.textContent = `上次清理释放 ${text}${note}，缓存会自动重建`
  } catch (e) {
    setStatus(`清理缓存失败：${e}`, 'error')
  } finally {
    els.clearCacheBtn.disabled = false
  }
})

// ---------------- 确认弹窗 ----------------
let modalResolve = null

function showConfirm({ title, body, okText = '确定' }) {
  els.modalTitle.textContent = title
  els.modalBody.textContent = body
  els.modalOk.textContent = okText
  els.modalMask.style.display = 'flex'
  return new Promise((resolve) => {
    modalResolve = resolve
  })
}

function closeConfirm(result) {
  els.modalMask.style.display = 'none'
  const r = modalResolve
  modalResolve = null
  if (r) r(result)
}

els.modalOk.addEventListener('click', () => closeConfirm(true))
els.modalCancel.addEventListener('click', () => closeConfirm(false))
els.modalMask.addEventListener('click', (e) => {
  if (e.target === els.modalMask) closeConfirm(false)
})
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && modalResolve) closeConfirm(false)
})

// 设置页：打开配置文件位置 / 缓存文件夹
els.openConfigLoc.addEventListener('click', () => {
  openLocation((appInfo && appInfo.configPath) || els.configPathText.textContent, true)
})
els.openDataLoc.addEventListener('click', () => {
  openLocation((appInfo && appInfo.dataDir) || els.dataDirText.textContent, false)
})

els.searchInput.addEventListener('input', (e) => {
  state.search = e.target.value
  renderGrid()
})

// 侧边栏收起/展开
let collapsed = localStorage.getItem('launcher-collapsed') === '1'
function applyCollapse() {
  els.sidebar.classList.toggle('collapsed', collapsed)
  els.collapseBtn.textContent = collapsed ? '▶' : '◀'
}
els.collapseBtn.addEventListener('click', () => {
  collapsed = !collapsed
  localStorage.setItem('launcher-collapsed', collapsed ? '1' : '0')
  applyCollapse()
})

// ---------------- 初始化 ----------------
async function init() {
  applyCollapse()
  updatePathDisplay()

  if (isTauri) {
    try {
      // 读取收藏
      const favs = await invoke('cmd_get_favorites')
      state.favorites = favs || []
    } catch (e) {
      console.warn('读取收藏失败', e)
    }
    try {
      // 读取用户偏好（自动加载 / 主题 / 启动点击次数）
      const s = await invoke('cmd_get_settings')
      state.autoLoad = s.auto_load !== false
      state.clickNormal = s.click_normal === 'double' ? 'double' : 'single'
      state.clickWeak = s.click_weak === 'double' ? 'double' : 'single'
      applyTheme(s.theme === 'dark' ? 'dark' : 'light')
    } catch (e) {
      console.warn('读取设置失败', e)
    }
    try {
      // 读取上次文件夹并按设置决定是否自动加载
      const saved = await invoke('cmd_get_saved_folder')
      if (saved && state.autoLoad) {
        state.folder = saved
        updatePathDisplay()
        // 优先用上次扫描的缓存「秒开」；随后后台静默校验，有变化才更新
        let cached = null
        try {
          cached = await invoke('cmd_load_scan_cache', { folder: saved })
        } catch (e) {
          console.warn('读取扫描缓存失败', e)
        }
        if (cached && Array.isArray(cached.categories)) {
          applyScanResult(saved, cached)
          // 首屏先稳定显示，再在空闲时后台校验（避免与首屏渲染争抢 CPU/IO）
          setTimeout(() => { void verifyScanInBackground(saved, cached) }, 1200)
        } else {
          await scanAndLoad(saved)
        }
      } else {
        setStatus('请点击左下角「选择文件夹」加载你的软件库', 'ok')
        renderNav()
        renderGrid()
      }
    } catch (e) {
      console.warn('读取配置失败', e)
      renderNav()
      renderGrid()
    }
  } else {
    state.folder = 'D:\\软件库'
    state.categories = DEMO
    updatePathDisplay()
    setStatus('浏览器预览模式（演示数据）。打包为 Tauri 后即可真实扫描启动。', 'ok')
    renderNav()
    renderGrid()
  }
}

init()

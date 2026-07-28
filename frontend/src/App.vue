<template>
  <!-- Dashboard background -->
  <div id="dashboard">
    <div class="wp-bg" :style="wpStyle"></div>
    <div class="wp-overlay"></div>

    <div class="dash-content">
      <!-- Header -->
      <div class="dash-header" :style="headerStyle">
        <img v-if="panelInfo.logo" class="dash-logo" :src="panelInfo.logo" alt="" />
        <button class="icon-btn net-btn" @click="toggleNet" :title="netMode === 'lan' ? t('switchToWan') : t('switchToLan')">
          <svg v-if="netMode === 'lan'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/>
          </svg>
          <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/>
            <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
          </svg>
        </button>
        <button class="icon-btn" @click="onSettingsClick">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="3"/>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06-.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
          </svg>
        </button>
      </div>

      <!-- Hero -->
      <div class="hero">
        <div class="hero-hostname" :style="hostnameStyle">{{ panelInfo.hostname || 'Helm' }}</div>
        <div class="hero-clock" :style="clockStyle" v-html="clockHtml"></div>
      </div>

      <!-- Apps -->
      <div class="apps-outer" :style="outerStyle" @contextmenu.prevent="onPanelContextMenu">
        <div class="apps-inner" :style="innerStyle">

          <!-- Sort bar -->
          <div v-if="sortMode" class="sort-bar">
            <button class="sort-btn" @click="exitSort(true)">💾 {{ t('saveSortBtn') }}</button>
            <button class="sort-btn" style="background:rgba(255,255,255,.6)" @click="exitSort(false)">✕ {{ t('cancelBtn') }}</button>
          </div>

          <!-- App grid -->
        <div class="apps-grid" :style="{ gap: dispSet.iconGap + 'px' }">
          <!-- 系统功能图标：固定在最前，不参与排序/编辑 -->
          <template v-if="featureSysInfo||featureProcess||featureSystemd||featureDocker">
            <!-- 本机信息：仪表盘 -->
            <div v-if="featureSysInfo" class="app-card" :style="{ width: (dispSet.iconSize + 14) + 'px' }" @click="requireAuth(() => sysInfoModal?.open())">
              <div class="app-icon-wrap" :style="iconWrapStyle">
                <div class="sys-ico-fill" style="background:linear-gradient(145deg,#7c3aed,#4f46e5)">
                  <svg width="44" height="44" viewBox="0 0 44 44" fill="none">
                    <path d="M8 30 A16 16 0 1 1 36 30" stroke="rgba(255,255,255,0.25)" stroke-width="3.5" stroke-linecap="round"/>
                    <line x1="22" y1="7"  x2="22" y2="10" stroke="white" stroke-width="2" stroke-linecap="round" opacity="0.6"/>
                    <line x1="10" y1="11" x2="12.2" y2="13.2" stroke="white" stroke-width="2" stroke-linecap="round" opacity="0.6"/>
                    <line x1="34" y1="11" x2="31.8" y2="13.2" stroke="white" stroke-width="2" stroke-linecap="round" opacity="0.6"/>
                    <path d="M8 30 A16 16 0 0 1 34.5 16" stroke="white" stroke-width="3.5" stroke-linecap="round" opacity="0.9"/>
                    <line x1="22" y1="22" x2="31" y2="14" stroke="white" stroke-width="2.2" stroke-linecap="round"/>
                    <circle cx="22" cy="22" r="3" fill="white" opacity="0.95"/>
                    <circle cx="22" cy="22" r="1.2" fill="rgba(100,80,200,0.8)"/>
                    <circle cx="14" cy="32" r="1.5" fill="rgba(255,255,255,0.5)"/>
                    <circle cx="22" cy="34" r="1.5" fill="rgba(255,255,255,0.5)"/>
                    <circle cx="30" cy="32" r="1.5" fill="rgba(255,255,255,0.5)"/>
                  </svg>
                </div>
              </div>
              <div v-show="showAppName" class="app-name" :style="appNameStyle">{{ t('lblSysInfo') }}</div>
            </div>
            <!-- 进程管理：心跳波形 -->
            <div v-if="featureProcess" class="app-card" :style="{ width: (dispSet.iconSize + 14) + 'px' }" @click="requireAuth(() => processModal?.open())">
              <div class="app-icon-wrap" :style="iconWrapStyle">
                <div class="sys-ico-fill" style="background:linear-gradient(145deg,#f97316,#b91c1c)">
                  <svg width="44" height="44" viewBox="0 0 44 44" fill="none">
                    <line x1="6" y1="16" x2="38" y2="16" stroke="rgba(255,255,255,0.15)" stroke-width="1"/>
                    <line x1="6" y1="22" x2="38" y2="22" stroke="rgba(255,255,255,0.15)" stroke-width="1"/>
                    <line x1="6" y1="28" x2="38" y2="28" stroke="rgba(255,255,255,0.15)" stroke-width="1"/>
                    <polyline points="6,22 12,22 15,12 18,32 21,18 24,26 27,22 30,14 33,22 38,22" stroke="white" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" fill="none" opacity="0.95"/>
                    <line x1="30" y1="10" x2="30" y2="34" stroke="rgba(255,255,255,0.35)" stroke-width="1.5" stroke-linecap="round"/>
                    <circle cx="30" cy="14" r="2" fill="rgba(255,200,100,0.9)"/>
                  </svg>
                </div>
              </div>
              <div v-show="showAppName" class="app-name" :style="appNameStyle">{{ t('lblProcess') }}</div>
            </div>
            <!-- Systemd：电源符号 -->
            <div v-if="featureSystemd" class="app-card" :style="{ width: (dispSet.iconSize + 14) + 'px' }" @click="requireAuth(() => systemdModal?.open())">
              <div class="app-icon-wrap" :style="iconWrapStyle">
                <div class="sys-ico-fill" style="background:linear-gradient(145deg,#0ea5e9,#1d4ed8)">
                  <svg width="44" height="44" viewBox="0 0 44 44" fill="none">
                    <path d="M14 12 A12 12 0 1 0 30 12" stroke="white" stroke-width="3.2" stroke-linecap="round" opacity="0.9"/>
                    <line x1="22" y1="8" x2="22" y2="22" stroke="white" stroke-width="3.2" stroke-linecap="round"/>
                    <circle cx="22" cy="26" r="1.5" fill="rgba(255,255,255,0.5)"/>
                    <circle cx="17" cy="30" r="1.2" fill="rgba(255,255,255,0.35)"/>
                    <circle cx="27" cy="30" r="1.2" fill="rgba(255,255,255,0.35)"/>
                  </svg>
                </div>
              </div>
              <div v-show="showAppName" class="app-name" :style="appNameStyle">{{ t('lblSystemd') }}</div>
            </div>
            <!-- Docker：鲸鱼 -->
            <div v-if="featureDocker" class="app-card" :style="{ width: (dispSet.iconSize + 14) + 'px' }" @click="requireAuth(() => dockerModal?.open())">
              <div class="app-icon-wrap" :style="iconWrapStyle">
                <div class="sys-ico-fill" style="background:linear-gradient(145deg,#22d3ee,#0369a1)">
                  <svg width="46" height="46" viewBox="0 0 46 46" fill="none">
                    <rect x="10" y="14" width="5.5" height="4.5" rx="1.2" fill="white" opacity="0.95"/>
                    <rect x="17" y="14" width="5.5" height="4.5" rx="1.2" fill="white" opacity="0.95"/>
                    <rect x="24" y="14" width="5.5" height="4.5" rx="1.2" fill="white" opacity="0.95"/>
                    <rect x="17" y="8.5" width="5.5" height="4.5" rx="1.2" fill="white" opacity="0.95"/>
                    <rect x="24" y="8.5" width="5.5" height="4.5" rx="1.2" fill="white" opacity="0.95"/>
                    <path d="M31.5 16.5 C35 16 37 17.5 37 19.5" stroke="white" stroke-width="1.8" stroke-linecap="round" opacity="0.9"/>
                    <circle cx="38" cy="17" r="2" fill="white" opacity="0.85"/>
                    <path d="M6 23 C6 21 8 20 11 20.5 L35 20.5 C38 20.5 40 22 40 24.5 C40 28 36 30 30 30 L12 30 C8 30 6 28 6 25 Z" fill="white" opacity="0.92"/>
                    <path d="M6 25 C4 25 2 27 3 30 C4 32 7 31 9 29" fill="white" opacity="0.92"/>
                    <circle cx="32" cy="24.5" r="1.8" fill="rgba(3,105,161,0.7)"/>
                    <circle cx="32.5" cy="24" r="0.6" fill="white"/>
                    <path d="M15 32 Q16 30 17 32 Q18 34 19 32" stroke="rgba(255,255,255,0.6)" stroke-width="1.4" fill="none" stroke-linecap="round"/>
                    <path d="M23 33 Q24 31 25 33 Q26 35 27 33" stroke="rgba(255,255,255,0.5)" stroke-width="1.3" fill="none" stroke-linecap="round"/>
                  </svg>
                </div>
              </div>
              <div v-show="showAppName" class="app-name" :style="appNameStyle">{{ t('lblDocker') }}</div>
            </div>
          </template>
            <div
              v-for="app in apps" :key="app.id"
              class="app-card"
              :class="{ 'sort-mode': sortMode, 'drag-over': dragOverId === app.id }"
              :style="{ width: (dispSet.iconSize + 14) + 'px' }"
              :draggable="sortMode"
              @click="onAppClick(app)"
              @contextmenu.prevent.stop="onAppContextMenu($event, app.id)"
              @dragstart="onDragStart($event, app.id)"
              @dragover.prevent="dragOverId = app.id"
              @dragleave="dragOverId = null"
              @drop="onDrop($event, app.id)"
            >
              <div class="app-icon-wrap" :style="iconWrapStyle">
                <template v-if="app.icon_type === 'image' && app.icon_image">
                  <img class="app-icon-img" :src="app.icon_image" :alt="app.title"
                    :style="{ borderRadius: iconBorderRadius }"
                    @error="e => { e.target.style.display='none'; e.target.nextElementSibling.style.display='flex' }" />
                  <div class="app-icon-txt" style="display:none" :style="iconTxtStyle">{{ (app.title || '?').substring(0, 2) }}</div>
                </template>
                <div v-else class="app-icon-txt" :style="iconTxtStyle">
                  {{ (app.icon_text && app.icon_text.trim()) ? app.icon_text.trim() : (app.title || '?').substring(0, 2) }}
                </div>
              </div>
              <div v-show="showAppName" class="app-name" :style="appNameStyle">{{ app.title }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- Components -->
  <LoginModal ref="loginModal" />
  <AppModal ref="appModal" @saved="loadApps" @deleted="loadApps" @toast="showToast" />
  <ContextMenu ref="ctxMenu" @edit="onCtxEdit" @delete="onCtxDelete" @add="onCtxAdd" @sort="onCtxSort" />
  <SettingsPanel
    ref="settingsPanel"
    :user="curUser"
    :panel-info="panelInfo"
    :pub-mode-value="pubMode"
    :net-mode-value="netMode"
    @net-mode-changed="v => netMode = v"
    :feature-sysinfo="featureSysInfo"
    :feature-process="featureProcess"
    :feature-systemd="featureSystemd"
    :feature-docker="featureDocker"
    @features-changed="onFeaturesChanged"
    :show-app-name="showAppName"
    @show-app-name-changed="v => showAppName = v"
    :desktop-disp="desktopDisp"
    :mobile-disp="mobileDisp"
    :disp-set="dispSet"
    :font-set="fontSet"
    :clk-cfg="clkCfg"
    :apps="apps"
    @toast="showToast"
    @logout="doLogout"
    @panel-updated="loadPanel"
    @clear-data="clearAllData"
  />
  <AppToast ref="toast" />
  <SystemInfoModal ref="sysInfoModal" />
  <ProcessModal    ref="processModal" />
  <SystemdModal    ref="systemdModal" />
  <DockerModal     ref="dockerModal" />
</template>

<script setup>
import { ref, reactive, computed, onMounted, onUnmounted } from 'vue'
import { useI18n, lang } from './composables/useI18n.js'
import { apiCall } from './composables/useApi.js'
import { useTheme, resolveFont, curThemeId, curTheme, WPS } from './composables/useTheme.js'
import { getLunar } from './composables/useLunar.js'
import LoginModal from './components/LoginModal.vue'
import AppModal from './components/AppModal.vue'
import ContextMenu from './components/ContextMenu.vue'
import SettingsPanel from './components/SettingsPanel.vue'
import SystemInfoModal from './components/SystemInfoModal.vue'
import ProcessModal from './components/ProcessModal.vue'
import DockerModal from './components/DockerModal.vue'
import SystemdModal from './components/SystemdModal.vue'
import AppToast from './components/AppToast.vue'

const { t } = useI18n()
useTheme()

// ── Refs ────────────────────────────────────────────────────────
const loginModal   = ref(null)
const appModal     = ref(null)
const ctxMenu      = ref(null)
const settingsPanel  = ref(null)
const sysInfoModal   = ref(null)
const processModal   = ref(null)
const dockerModal    = ref(null)
const systemdModal   = ref(null)
const toast        = ref(null)

// ── State ───────────────────────────────────────────────────────
const apps      = ref([])
const panelInfo = ref({})
const curUser   = ref(null)
const pubMode   = ref(false)
const netMode   = ref('lan')  // 'lan' | 'wan'
const featureSysInfo = ref(false)
const featureProcess = ref(false)
const featureSystemd = ref(false)
const featureDocker  = ref(false)
const showAppName    = ref(true)
const desktopDisp    = ref(null)  // 桌面端完整样式数据
const mobileDisp     = ref(null)  // 移动端完整样式数据
const clkCfg    = reactive({ show_time: true, show_date: true, show_weekday: true, show_lunar: false, show_seconds: false, show_year: false })
const clockHtml = ref('')
const dispSet   = reactive({ hostnameSize: 76, clockSize: 24, iconSize: 64, appNameSize: 14, iconRadius: 25, iconGap: 22, sidePadding: 52 })
const fontSet   = reactive({ hostname: 'system', clock: 'system', appname: 'system', ui: 'system' })
const sortMode  = ref(false)
const dragOverId = ref(null)

let clkTimer = null

// ── Computed ────────────────────────────────────────────────────
const fallbackGrad = computed(() => curTheme.value.grad || 'linear-gradient(135deg,#6b21a8 0%,#a855f7 40%,#ec4899 100%)')
const wallpaperUrl  = computed(() => panelInfo.value.wallpaper || curTheme.value.wp || '/default-wallpaper')
const wpStyle = computed(() => ({
  background: fallbackGrad.value,
  backgroundImage: wallpaperUrl.value ? `url(${wallpaperUrl.value})` : 'none',
  backgroundSize: 'cover',
  backgroundPosition: 'center',
}))

const iconBorderRadius = computed(() => Math.round((dispSet.iconRadius || 26) / 100 * dispSet.iconSize) + 'px')
const iconWrapStyle = computed(() => ({ width: dispSet.iconSize + 'px', height: dispSet.iconSize + 'px', borderRadius: iconBorderRadius.value }))
const iconTxtStyle  = computed(() => ({ fontSize: Math.round(dispSet.iconSize * 0.33) + 'px', borderRadius: iconBorderRadius.value }))
const appNameStyle  = computed(() => ({ fontSize: dispSet.appNameSize + 'px', maxWidth: (dispSet.iconSize + 10) + 'px', fontFamily: resolveFont(fontSet.appname) }))
const hostnameStyle = computed(() => ({ fontSize: dispSet.hostnameSize + 'px', fontFamily: resolveFont(fontSet.hostname) }))
const clockStyle    = computed(() => ({ fontSize: dispSet.clockSize + 'px', fontFamily: resolveFont(fontSet.clock) }))
const outerStyle    = computed(() => ({ padding: `0 0 52px` }))
const headerStyle   = computed(() => ({ padding: `22px ${dispSet.sidePadding}px 0` }))
const innerStyle    = computed(() => ({ padding: `0 ${dispSet.sidePadding}px` }))

// ── Clock ───────────────────────────────────────────────────────
function tick() {
  const now = new Date(), pad = n => String(n).padStart(2, '0')
  const parts = []

  // 左侧：年 / 农历 / 日期 / 星期
  const leftItems = []
  if (clkCfg.show_year) leftItems.push(`${now.getFullYear()}年`)
  if (clkCfg.show_lunar && lang.value === 'zh') {
    const l = getLunar(now)
    if (l) leftItems.push(`农历 ${l}`)
  }
  if (clkCfg.show_date) leftItems.push(`${now.getMonth() + 1}月${now.getDate()}日`)
  const days = t('weekdays')
  if (clkCfg.show_weekday) leftItems.push(days[now.getDay()])
  if (leftItems.length) parts.push(`<span>${leftItems.join(' &nbsp;')}</span>`)

  // 分隔符 + 右侧：时间
  if (clkCfg.show_time) {
    let tv = `${pad(now.getHours())}:${pad(now.getMinutes())}`
    if (clkCfg.show_seconds) tv += `:${pad(now.getSeconds())}`
    if (leftItems.length) parts.push(`<span class="div">|</span>`)
    parts.push(`<span>${tv}</span>`)
  }

  clockHtml.value = parts.join('')
}

function startClock() {
  if (clkTimer) clearInterval(clkTimer)
  tick(); clkTimer = setInterval(tick, 1000)
}

// ── Data ────────────────────────────────────────────────────────
async function loadPanel() {
  try {
    const info = await apiCall('/api/panel')
    panelInfo.value = info
    pubMode.value = info.public_mode || false
    netMode.value = info.network_mode || 'lan'
    featureSysInfo.value = info.feature_sysinfo || false
    featureProcess.value = info.feature_process || false
    featureSystemd.value = info.feature_systemd || false
    featureDocker.value  = info.feature_docker  || false
    showAppName.value    = info.show_app_name !== false // 默认true
    desktopDisp.value    = info.desktop || null
    mobileDisp.value     = info.mobile  || null
    if (info.clock) Object.assign(clkCfg, info.clock)
    const sl = info.language || localStorage.getItem('ep_lang') || 'zh'
    lang.value = sl; localStorage.setItem('ep_lang', sl)
    const th = (await import('./composables/useTheme.js')).THEMES.find(x => x.id === info.theme)
    if (th) { curThemeId.value = th.id; (await import('./composables/useTheme.js')).applyThemeCss(th) }
    Object.assign(dispSet, {
      hostnameSize: info.hostname_size || 56, clockSize: info.clock_size || 16,
      iconSize: info.icon_size || 78, appNameSize: info.app_name_size || 12,
      iconRadius: info.icon_radius || 26, iconGap: info.icon_gap || 22, sidePadding: info.side_padding || 52,
    })
    Object.assign(fontSet, {
      hostname: info.font_hostname || 'system', clock: info.font_clock || 'system',
      appname: info.font_appname || 'system', ui: info.font_ui || 'system',
    })
    tick()
  } catch (e) { console.error(e) }
}

async function loadApps() {
  try { apps.value = await apiCall('/api/apps') } catch (e) { console.error(e) }
}

// ── Auth ────────────────────────────────────────────────────────
function showToast(msg) { toast.value?.show(msg) }

async function requireAuth(cb) {
  // 内存里已有用户，直接放行
  if (curUser.value) { await cb(); return }
  // 内存没有，但 cookie 里可能有有效 token，先验证一次
  try {
    const auth = await apiCall('/api/checkauth')
    if (auth.logged_in) {
      curUser.value = auth
      await cb()
      return
    }
  } catch (e) { console.error('checkauth error:', e) }
  // cookie 也无效，才弹登录框
  loginModal.value?.open(t('loginRequired'), async (user) => { curUser.value = user; await cb() })
}

async function doLogout() {
  await apiCall('/api/logout', { method: 'POST' }).catch(() => {})
  curUser.value = null
  settingsPanel.value?.close()
  if (!pubMode.value) loginModal.value?.open(t('pleaseLogin'), (user) => { curUser.value = user })
  showToast(t('tLoggedOut'))
}

// ── Settings ────────────────────────────────────────────────────
async function onSettingsClick() {
  await requireAuth(async () => { await settingsPanel.value?.open() })
}

// ── App actions ─────────────────────────────────────────────────
function onAddApp() { requireAuth(() => appModal.value?.openAdd()) }
function onAppClick(app) {
  if (sortMode.value) return
  // 根据网络模式选地址，优先用对应模式地址，没有则 fallback 到另一个，再 fallback 旧 url 字段
  let url = ''
  if (netMode.value === 'lan') {
    url = app.url_lan || app.url_wan || app.url || ''
  } else {
    url = app.url_wan || app.url_lan || app.url || ''
  }
  if (!url) return
  app.open_type === 'current' ? (window.location.href = url) : window.open(url, '_blank')
}
function toggleNet() {
  netMode.value = netMode.value === 'lan' ? 'wan' : 'lan'
}
function onFeaturesChanged(f) {
  featureSysInfo.value = f.sysinfo
  featureProcess.value = f.process
  featureSystemd.value = f.systemd
  featureDocker.value  = f.docker
}
function onAppContextMenu(e, id) {
  requireAuth(() => ctxMenu.value?.show(e.clientX, e.clientY, id))
}
function onPanelContextMenu(e) {
  requireAuth(() => ctxMenu.value?.showPanel(e.clientX, e.clientY))
}

function onCtxEdit(id) {
  const app = apps.value.find(x => x.id === id)
  if (app) appModal.value?.openEdit(app)
}
async function onCtxDelete(id) {
  if (!confirm(t('confirmDelete'))) return
  try { await apiCall(`/api/apps/${id}`, { method: 'DELETE' }); await loadApps(); showToast(t('tDeleted')) }
  catch { showToast(t('tDeleteFailed')) }
}
function onCtxAdd()  { onAddApp() }
function onCtxSort() { onEnterSort() }

// ── Sort ────────────────────────────────────────────────────────
function onEnterSort() { requireAuth(() => { sortMode.value = true }) }

async function exitSort(save) {
  sortMode.value = false; dragOverId.value = null
  if (save) {
    try { await apiCall('/api/apps/reorder', { method: 'POST', body: JSON.stringify({ ids: apps.value.map(a => a.id) }) }); showToast(t('tSortSaved')) }
    catch { showToast('failed') }
  } else await loadApps()
}

function onDragStart(e, id) {
  if (!sortMode.value) { e.preventDefault(); return }
  e.dataTransfer.effectAllowed = 'move'
  e.dataTransfer.setData('text/plain', id)
}
function onDrop(e, targetId) {
  dragOverId.value = null
  const srcId = e.dataTransfer.getData('text/plain')
  if (!srcId || srcId === targetId) return
  const a = apps.value.findIndex(x => x.id === srcId), b = apps.value.findIndex(x => x.id === targetId)
  if (a < 0 || b < 0) return
  const [item] = apps.value.splice(a, 1); apps.value.splice(b, 0, item)
}

// ── Backup clear ─────────────────────────────────────────────────
async function clearAllData() {
  if (!confirm(t('confirmClear'))) return
  try {
    for (const app of apps.value) await apiCall(`/api/apps/${app.id}`, { method: 'DELETE' }).catch(() => {})
    await loadApps(); showToast(t('tCleared'))
  } catch { showToast(t('tFailed')) }
}

// ── Lifecycle ───────────────────────────────────────────────────
onMounted(async () => {
  await loadPanel()
  await loadApps()
  startClock()
  const auth = await apiCall('/api/checkauth').catch(() => ({ logged_in: false }))
  if (auth.logged_in) curUser.value = auth
  if (!pubMode.value && !curUser.value) {
    loginModal.value?.open(t('pleaseLogin'), (user) => { curUser.value = user })
  }
  document.addEventListener('click', (e) => {
    const ctx = document.getElementById('ctx-menu')
    if (ctx && !ctx.contains(e.target)) ctxMenu.value?.hide()
  })
})

onUnmounted(() => { if (clkTimer) clearInterval(clkTimer) })
</script>

<style scoped>
#dashboard { min-height: 100vh; position: relative; }
.wp-bg { position: fixed; inset: 0; z-index: 0; background-size: cover; background-position: center; transition: background .6s; }
.wp-overlay { position: fixed; inset: 0; z-index: 0; background: rgba(0,0,0,.22); }
.dash-content { position: relative; z-index: 1; min-height: 100vh; display: flex; flex-direction: column; }
.dash-header { display: flex; justify-content: flex-start; align-items: flex-start; }
.dash-logo { height: 34px; width: auto; border-radius: 8px; object-fit: contain; }
.net-btn { position: fixed; top: 22px; right: 100px; }
.sys-ico-fill { width:100%; height:100%; display:flex; align-items:center; justify-content:center; border-radius:inherit; }
.icon-btn { position: fixed; top: 22px; right: 52px; z-index: 10; width: 40px; height: 40px; border-radius: 12px; cursor: pointer; display: flex; align-items: center; justify-content: center; color: white; font-size: 17px; transition: all var(--tr); border: 1px solid rgba(255,255,255,.25); background: rgba(255,255,255,.15); backdrop-filter: blur(12px); }
.icon-btn:hover { background: rgba(255,255,255,.28); transform: translateY(-1px); }
.hero { text-align: center; padding: 32px 20px 40px; color: white; flex-shrink: 0; }
.hero-hostname { font-size: 56px; font-weight: 900; letter-spacing: -1.5px; line-height: 1.05; margin-bottom: 20px; text-shadow: 0 2px 24px rgba(0,0,0,.28); }
.hero-clock { font-size: 16px; display: flex; align-items: center; justify-content: center; gap: 8px; flex-wrap: wrap; opacity: .9; }
:deep(.hero-clock .div) { opacity: .35; }
.apps-outer { flex: 1; display: flex; justify-content: center; }
.apps-inner { width: 100%; max-width: 1100px; }
.group-row { display: flex; align-items: center; gap: 10px; margin-bottom: 18px; }
.group-lbl { color: rgba(255,255,255,.92); font-size: 15px; font-weight: 800; text-transform: uppercase; letter-spacing: 2px; text-shadow: 0 1px 6px rgba(0,0,0,.3); }
.group-tools { display: none; align-items: center; gap: 6px; }
.group-row:hover .group-tools { display: flex; }
.tool-btn { width: 30px; height: 30px; border-radius: 9px; border: 1.5px solid rgba(255,255,255,.4); background: rgba(255,255,255,.12); backdrop-filter: blur(8px); cursor: pointer; display: flex; align-items: center; justify-content: center; color: rgba(255,255,255,.9); transition: all var(--tr); }
.tool-btn:hover, .tool-btn.active { background: rgba(255,255,255,.28); border-color: white; }
.sort-bar { display: flex; margin-bottom: 14px; gap: 8px; flex-wrap: wrap; }
.sort-btn { padding: 8px 18px; background: rgba(255,255,255,.88); color: #1e1b2e; border: none; border-radius: 10px; font-size: 13px; font-weight: 700; cursor: pointer; display: flex; align-items: center; gap: 5px; box-shadow: 0 4px 14px rgba(0,0,0,.14); transition: all var(--tr); }
.sort-btn:hover { background: white; transform: translateY(-1px); }
.apps-grid { display: flex; flex-wrap: wrap; justify-content: flex-start; }
.app-card { display: flex; flex-direction: column; align-items: center; gap: 9px; cursor: pointer; user-select: none; }
.app-icon-wrap { overflow: hidden; transition: transform var(--tr), box-shadow var(--tr); box-shadow: 0 6px 20px rgba(0,0,0,.24), 0 1px 0 rgba(255,255,255,.2) inset; }
.app-card:not(.sort-mode):hover .app-icon-wrap { transform: translateY(-6px) scale(1.07); box-shadow: 0 16px 36px rgba(0,0,0,.32), 0 1px 0 rgba(255,255,255,.2) inset; }
.app-icon-img { width: 100%; height: 100%; object-fit: cover; display: block; }
.app-icon-txt { width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; background: var(--grad); font-weight: 800; color: white; text-shadow: 0 1px 4px rgba(0,0,0,.25); }
.app-name { font-size: 12px; color: rgba(255,255,255,.96); text-align: center; font-weight: 600; text-shadow: 0 1px 4px rgba(0,0,0,.4); overflow: hidden; text-overflow: ellipsis; white-space: normal; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; word-break: break-word; line-height: 1.35; min-height: 1.35em; }
.app-card.sort-mode { animation: wiggle .4s ease infinite alternate; }
.app-card.sort-mode .app-icon-wrap { border: 2.5px dashed rgba(255,255,255,.65); transform: none !important; box-shadow: none !important; }
.app-card.drag-over .app-icon-wrap { border-color: white; background: rgba(255,255,255,.12); }
@media(max-width:700px){
  .hero { padding: 22px 16px 30px; }
  .icon-btn { right: 18px; }
}
</style>

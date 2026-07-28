<template>
  <template v-if="visible">
    <div class="s-backdrop" @click="close"></div>
    <div class="s-panel">
      <div class="s-box" :style="{ fontFamily: resolveFont(form.fonts.ui) }">
        <button class="s-close-top" @click="close" title="关闭"></button>

        <!-- ── 移动端：抽屉遮罩 ── -->
        <div v-if="mobileDrawerOpen" class="mob-drawer-mask" @click="mobileDrawerOpen = false"></div>

        <!-- ── 移动端：滑入抽屉导航 ── -->
        <div class="mob-drawer" :class="{ open: mobileDrawerOpen }">
          <div class="mob-drawer-header">
            <div class="mob-drawer-ico">
              <img v-if="form.logoPreview || form.logo" :src="form.logoPreview || form.logo" style="width:100%;height:100%;object-fit:cover;border-radius:9px;" />
              <svg v-else width="18" height="18" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                <rect x="3" y="6" width="18" height="2.5" rx="1.25" fill="white"/>
                <rect x="3" y="11" width="13" height="2.5" rx="1.25" fill="white" fill-opacity="0.75"/>
                <rect x="3" y="16" width="15.5" height="2.5" rx="1.25" fill="white" fill-opacity="0.5"/>
              </svg>
            </div>
            <span class="mob-drawer-title">{{ t('sysSettings') }}</span>
          </div>
          <div class="mob-nav-list">
            <div v-for="tab in TABS" :key="tab.id" class="mob-ni" :class="{ active: activeTab === tab.id }"
              @click="activeTab = tab.id; mobileDrawerOpen = false">
              <span class="mob-ni-ico">{{ tab.icon }}</span>
              <span class="mob-ni-lbl">{{ t(tab.labelKey) }}</span>
            </div>
          </div>
        </div>

        <!-- ── 桌面端：固定侧边导航 ── -->
        <div class="s-nav">
          <div class="s-nav-header">
            <div class="s-nav-ico">
              <img v-if="form.logoPreview || form.logo" :src="form.logoPreview || form.logo" style="width:100%;height:100%;object-fit:cover;border-radius:9px;" />
              <svg v-else width="18" height="18" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                <rect x="3" y="6" width="18" height="2.5" rx="1.25" fill="white"/>
                <rect x="3" y="11" width="13" height="2.5" rx="1.25" fill="white" fill-opacity="0.75"/>
                <rect x="3" y="16" width="15.5" height="2.5" rx="1.25" fill="white" fill-opacity="0.5"/>
              </svg>
            </div>
            <span class="s-nav-t">{{ t('sysSettings') }}</span>
          </div>
          <div class="s-nav-list">
            <div v-for="tab in TABS" :key="tab.id" class="s-ni" :class="{ active: activeTab === tab.id }" @click="activeTab = tab.id">
              <span class="s-ico">{{ tab.icon }}</span>
              <span>{{ t(tab.labelKey) }}</span>
            </div>
          </div>
        </div>

        <!-- Content -->
        <div class="s-content">

          <!-- ── 移动端顶部导航栏 ── -->
          <div class="mob-topbar">
            <button class="mob-hamburger" @click="mobileDrawerOpen = true">
              <span></span><span></span><span></span>
            </button>
            <span class="mob-topbar-title">{{ t(TABS.find(t => t.id === activeTab)?.labelKey || 'sysSettings') }}</span>
          </div>

          <!-- ── 我的信息 ── -->
          <div v-if="activeTab === 'account'">
            <div class="s-title" v-show="!isMobile">{{ t('titleAccount') }}</div>
            <div class="s-card">
              <div class="s-row"><label class="s-lbl">{{ t('lblUsername') }}</label><input class="s-inp" :value="user?.username || ''" disabled /></div>
              <div class="s-row"><label class="s-lbl">{{ t('lblNickname') }}</label><input class="s-inp" v-model="form.nick" :placeholder="t('nickPlaceholder')" /></div>
              <button class="btn btn-p" @click="saveNick">{{ t('saveNickBtn') }}</button>
            </div>
            <div class="s-card">
              <div class="s-title" style="font-size:14px;margin-bottom:14px;padding-bottom:10px;border-bottom-width:1px">{{ t('changePwdTitle') }}</div>
              <div class="s-row"><label class="s-lbl">{{ t('lblOldPwd') }}</label><input type="password" class="s-inp" v-model="form.oldPwd" /></div>
              <div class="s-row"><label class="s-lbl">{{ t('lblNewPwd') }}</label><input type="password" class="s-inp" v-model="form.newPwd" /></div>
              <button class="btn btn-p" @click="changePwd">{{ t('changePwdBtn') }}</button>
            </div>
            <button class="btn btn-d" style="width:100%;margin-top:6px" @click="emit('logout')">{{ t('logoutBtn') }}</button>
          </div>

          <!-- ── 显示 ── -->
          <div v-if="activeTab === 'display'">
            <div class="s-title" v-show="!isMobile">{{ t('titleDisplay') }}</div>

            <!-- 主机名 & LOGO -->
            <div class="s-card">
              <div class="s-card-title">主机名 &amp; LOGO</div>
              <div class="s-row"><label class="s-lbl">{{ t('lblHostnameDisplay') }}</label><input class="s-inp" v-model="form.hostname" placeholder="Helm" /></div>
              <div class="s-row">
                <label class="s-lbl">LOGO</label>
                <div class="img-row" style="margin-bottom:8px">
                  <input class="s-inp" v-model="form.logo" placeholder="https://..." @input="form.logoPreview = form.logo" />
                  <label class="upbtn" for="logo-file-inp">📁</label>
                  <input type="file" id="logo-file-inp" accept="image/*" style="display:none" @change="uploadLogo" />
                </div>
                <img v-if="form.logoPreview" :src="form.logoPreview" style="width:48px;height:48px;border-radius:10px;object-fit:cover" />
              </div>
              <button class="btn btn-p" @click="savePanelCfg">{{ t('saveBtn') }}</button>
            </div>

            <!-- 样式（尺寸 + 图标） -->
            <div class="s-card-title" style="margin:20px 0 10px;">样式</div>

            <!-- 设备切换 Tab -->
            <div class="device-tab-bar">
              <button class="device-tab" :class="{ active: form.dispDevice === 'desktop' }" @click="form.dispDevice = 'desktop'">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="margin-right:5px;vertical-align:-2px"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>
                {{ t('deviceDesktop') }}
              </button>
              <button class="device-tab" :class="{ active: form.dispDevice === 'mobile' }" @click="form.dispDevice = 'mobile'">
                <svg width="12" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="margin-right:5px;vertical-align:-2px"><rect x="5" y="2" width="14" height="20" rx="2"/><circle cx="12" cy="18" r="1" fill="currentColor"/></svg>
                {{ t('deviceMobile') }}
              </button>
            </div>

            <div class="s-card">
              <div class="s-row" v-for="sl in [...sizeSliders, ...iconSliders]" :key="sl.key">
                <label class="s-lbl">{{ t(sl.label) }}</label>
                <div class="s-slider-row">
                  <input type="range" class="s-slider" v-model="curDispSet[sl.key]" :min="sl.min" :max="sl.max" />
                  <span class="s-slider-val">{{ curDispSet[sl.key] }}{{ sl.unit }}</span>
                </div>
              </div>
              <!-- 图标名称显示开关 -->
              <div class="t-row" style="margin-top:12px;padding-top:12px;border-top:1px solid #f0f4ff">
                <div>
                  <div class="t-lbl">{{ t('lblShowAppName') }}</div>
                  <div class="t-sub">{{ t('lblShowAppNameSub') }}</div>
                </div>
                <label class="sw"><input type="checkbox" v-model="form.showAppName" /><span class="sl"></span></label>
              </div>
            </div>
            <button class="btn btn-p" @click="saveDisplay" style="margin-top:12px">{{ t('saveDisplayBtn') }}</button>

            <!-- 字体 -->
            <div class="s-card-title" style="margin:20px 0 10px;">字体</div>
            <div class="s-card">
              <div class="s-row" v-for="f in fontFields" :key="f.key">
                <label class="s-lbl">{{ t(f.label) }}</label>
                <select class="s-sel" v-model="curDispSet[fontFieldMap[f.key]]">
                  <option v-for="opt in FONT_OPTIONS" :key="opt.v" :value="opt.v">{{ opt.l }}</option>
                </select>
              </div>
              <button class="btn btn-p" @click="saveDisplay" style="margin-top:4px">{{ t('saveFonts') }}</button>
            </div>
          </div>

          <!-- ── 主题色 ── -->
          <div v-if="activeTab === 'theme'">
            <div class="s-title" v-show="!isMobile">{{ t('titleTheme') }}</div>
            <div class="s-card">
              <div class="theme-grid">
                <div v-for="th in THEMES" :key="th.id" class="theme-item"
                  :class="{ sel: pendingThemeId === th.id, saved: curThemeId === th.id && pendingThemeId !== th.id }"
                  @click="selectTheme(th.id)">
                  <div class="theme-dot" :style="{ background: th.dot }"></div>
                  <div class="theme-name">{{ th.name }}</div>
                </div>
              </div>
            </div>
            <button class="btn btn-p" style="margin-top:12px" @click="saveTheme">{{ t('saveBtn') }}</button>
          </div>

          <!-- ── 时钟 ── -->
          <div v-if="activeTab === 'clock'">
            <div class="s-title" v-show="!isMobile">{{ t('titleClock') }}</div>
            <div class="s-card">
              <div class="t-row" v-for="ck in clockToggles" :key="ck.key">
                <div><div class="t-lbl">{{ t(ck.label) }}</div><div v-if="ck.sub" class="t-sub">{{ t(ck.sub) }}</div></div>
                <label class="sw"><input type="checkbox" v-model="form.clock[ck.key]" @change="saveClk" /><span class="sl"></span></label>
              </div>
            </div>
          </div>

          <!-- ── 壁纸 ── -->
          <div v-if="activeTab === 'wallpaper'">
            <div class="s-title" v-show="!isMobile">{{ t('titleWallpaper') }}</div>
            <div class="wp-grid">
              <div v-for="url in WPS" :key="url" class="wp-thumb" :class="{ sel: form.wallpaper === url }" @click="selectWp(url)">
                <img :src="url" loading="lazy" />
              </div>
            </div>
            <div class="s-row">
              <label class="s-lbl">{{ t('lblCustomUrl') }}</label>
              <div class="img-row">
                <input class="s-inp" v-model="form.wallpaper" placeholder="https://..." />
                <label class="upbtn" for="wp-file-inp">📁</label>
                <input type="file" id="wp-file-inp" accept="image/*" style="display:none" @change="uploadWp" />
              </div>
            </div>
            <button class="btn btn-p" @click="saveWp">{{ t('applyWpBtn') }}</button>
          </div>

          <!-- ── 账号管理 ── -->
          <div v-if="activeTab === 'users'">
            <div class="s-title" v-show="!isMobile">{{ t('titleUsers') }}</div>
            <div class="info-box">{{ t('infoAccounts') }}</div>
            <div class="s-card" style="overflow-x:auto;padding:0">
              <table class="u-table">
                <thead><tr><th>{{ t('thColAccount') }}</th><th>{{ t('thColNickname') }}</th><th>{{ t('thColRole') }}</th><th>{{ t('thColAction') }}</th></tr></thead>
                <tbody>
                  <tr v-for="u in users" :key="u.username">
                    <td>{{ u.username }}<span v-if="u.username === user?.username" class="badge b-cur">{{ t('badgeCurrent') }}</span></td>
                    <td>{{ u.nickname || '-' }}</td>
                    <td><span class="badge" :class="u.is_admin ? 'b-admin' : 'b-user'">{{ u.is_admin ? t('roleAdmin') : t('roleUser') }}</span></td>
                    <td><button v-if="u.username !== user?.username" class="btn btn-d" style="padding:4px 10px;font-size:12px" @click="removeUser(u.username)">{{ t('deleteUserBtn') }}</button><span v-else>-</span></td>
                  </tr>
                </tbody>
              </table>
            </div>
            <div class="s-card" style="margin-top:12px">
              <div class="s-title" style="font-size:14px;margin-bottom:12px;padding-bottom:10px;border-bottom-width:1px">{{ t('titleAddUser') }}</div>
              <div class="s-row"><label class="s-lbl">{{ t('lblNuName') }}</label><input class="s-inp" v-model="form.nuName" /></div>
              <div class="s-row"><label class="s-lbl">{{ t('lblNuPwd') }}</label><input type="password" class="s-inp" v-model="form.nuPwd" /></div>
              <div class="s-row"><label class="s-lbl">{{ t('lblNuNick') }}</label><input class="s-inp" v-model="form.nuNick" /></div>
              <button class="btn btn-p" @click="addUser">{{ t('addUserBtn') }}</button>
            </div>
          </div>


          <!-- ── 数据管理 ── -->
          <div v-if="activeTab === 'backup'">
            <div class="s-title" v-show="!isMobile">{{ t('titleBackup') }}</div>
            <div class="info-box" v-html="t('infoBackup').replace('\n','<br>')"></div>
            <div class="s-card">
              <div class="backup-btns">
                <button class="backup-btn" @click="exportData"><span class="b-ico">📤</span><span>{{ t('exportBtn') }}</span><span class="b-sub">{{ t('exportDesc') }}</span></button>
                <label class="backup-btn" for="import-file-inp"><span class="b-ico">📥</span><span>{{ t('importBtn') }}</span><span class="b-sub">{{ t('importDesc') }}</span></label>
                <input type="file" id="import-file-inp" accept=".json" style="display:none" @change="importData" />
              </div>
            </div>
            <div class="s-card" style="margin-top:12px">
              <div class="t-lbl" style="margin-bottom:10px">{{ t('lblDanger') }}</div>
              <button class="btn btn-d" @click="emit('clearData')" style="width:100%">{{ t('clearBtn') }}</button>
            </div>
          </div>

          <!-- ── 关于 ── -->
          <div v-if="activeTab === 'about'" class="about-wrap">
            <div class="about-logo-wrap">
              <img v-if="form.logoPreview || form.logo"
                :src="form.logoPreview || form.logo"
                style="width:100%;height:100%;object-fit:cover" />
              <div v-else class="about-logo-fallback">E</div>
            </div>
            <div class="about-name">Helm</div>
            <div class="about-version">{{ appVersion || '...' }}</div>
            <div class="about-desc">轻量级自托管面板</div>
            <a class="about-link" href="https://github.com/evecus/Helm" target="_blank">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12 0C5.37 0 0 5.37 0 12c0 5.3 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61-.546-1.385-1.335-1.755-1.335-1.755-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 21.795 24 17.295 24 12c0-6.63-5.37-12-12-12"/>
              </svg>
              GitHub
            </a>
          </div>

          <!-- ── 基本设置 ── -->
          <div v-if="activeTab === 'basic'">
            <div class="s-title" v-show="!isMobile">⚙️ {{ t('niBasic') }}</div>

            <!-- 语言 -->
            <div class="s-card">
              <div class="s-row">
                <label class="s-lbl">{{ t('niLanguage') }}</label>
                <div class="lang-btns">
                  <button class="lang-btn" :class="{ active: lang === 'zh' }" @click="setLang('zh')">🇨🇳 中文</button>
                  <button class="lang-btn" :class="{ active: lang === 'en' }" @click="setLang('en')">🇬🇧 English</button>
                </div>
              </div>
            </div>

            <!-- 访问控制 -->
            <div class="s-title" style="margin-top:20px">{{ t('titleAccess') }}</div>
            <div class="s-card">
              <div class="t-row">
                <div><div class="t-lbl">{{ t('lblPubMode') }}</div><div class="t-sub">{{ t('subPubMode') }}</div></div>
                <label class="sw"><input type="checkbox" v-model="form.pubMode" @change="setPubMode" /><span class="sl"></span></label>
              </div>
            </div>
            <div class="s-card" style="margin-top:12px">
              <div class="t-row">
                <div><div class="t-lbl">{{ t('lblNetMode') }}</div><div class="t-sub">{{ t('subNetMode') }}</div></div>
                <div class="net-toggle">
                  <button class="net-btn" :class="{ active: form.netMode === 'lan' }" @click="setNetMode('lan')">{{ t('netLan') }}</button>
                  <button class="net-btn" :class="{ active: form.netMode === 'wan' }" @click="setNetMode('wan')">{{ t('netWan') }}</button>
                </div>
              </div>
            </div>
            <div class="info-box" style="margin-top:8px">
              <b>{{ t('lblPrivateMode') }}</b>{{ t('descPrivateMode') }}<br>
              <b>{{ t('lblPublicMode') }}</b>{{ t('descPublicMode') }}
            </div>

            <!-- 系统功能 -->
            <div class="s-title" style="margin-top:20px">{{ t('titleFeatures') }}</div>
            <div class="s-card">
              <div class="t-row" style="padding-bottom:12px;border-bottom:1px solid #f0f4ff">
                <div>
                  <div class="t-lbl">🖥️ {{ t('featureSysInfo') }}</div>
                  <div class="t-sub">{{ t('featureSysInfoSub') }}</div>
                </div>
                <label class="sw"><input type="checkbox" v-model="form.featureSysinfo" @change="toggleFeature('featureSysinfo')" /><span class="sl"></span></label>
              </div>
              <div class="t-row" style="padding:12px 0;border-bottom:1px solid #f0f4ff">
                <div>
                  <div class="t-lbl">⚙️ {{ t('featureProcess') }}</div>
                  <div class="t-sub">{{ t('featureProcessSub') }}</div>
                </div>
                <label class="sw"><input type="checkbox" v-model="form.featureProcess" @change="toggleFeature('featureProcess')" /><span class="sl"></span></label>
              </div>
              <div class="t-row" style="padding:12px 0;border-bottom:1px solid #f0f4ff">
                <div>
                  <div class="t-lbl">🔧 {{ t('featureSystemd') }}</div>
                  <div class="t-sub">{{ t('featureSystemdSub') }}</div>
                </div>
                <label class="sw"><input type="checkbox" v-model="form.featureSystemd" @change="toggleFeature('featureSystemd')" /><span class="sl"></span></label>
              </div>
              <div class="t-row" style="padding-top:12px">
                <div>
                  <div class="t-lbl">🐳 {{ t('featureDocker') }}</div>
                  <div class="t-sub">{{ t('featureDockerSub') }}</div>
                </div>
                <label class="sw"><input type="checkbox" v-model="form.featureDocker" @change="toggleFeature('featureDocker')" /><span class="sl"></span></label>
              </div>
            </div>
          </div>

        </div>
      </div>
    </div>
  </template>
</template>

<script setup>
import { ref, reactive, watch, nextTick, computed } from 'vue'
import { useI18n, lang } from '../composables/useI18n.js'
import { apiCall } from '../composables/useApi.js'
import { THEMES, WPS, FONT_OPTIONS, curThemeId, applyThemeCss } from '../composables/useTheme.js'
import { resolveFont } from '../composables/useTheme.js'

const { t } = useI18n()
const emit = defineEmits(['toast', 'logout', 'panelUpdated', 'clearData', 'netModeChanged', 'featuresChanged', 'showAppNameChanged'])

const props = defineProps({
  user: Object,
  panelInfo: Object,
  pubModeValue: Boolean,
  netModeValue: String,
  featureSysinfo: Boolean,
  featureProcess: Boolean,
  featureSystemd: Boolean,
  featureDocker:  Boolean,
  showAppName:    { type: Boolean, default: true },
  dispSet: Object,
  fontSet: Object,
  clkCfg: Object,
  apps: Array,
  // 两套完整样式数据（来自 /api/panel）
  desktopDisp: Object,
  mobileDisp:  Object,
})

const visible = ref(false)
const activeTab = ref('account')
const users = ref([])
const appVersion = ref('')
const mobileDrawerOpen = ref(false)  // 移动端侧边抽屉
const isMobile = ref(window.innerWidth <= 700)
if (typeof window !== 'undefined') {
  const mq = window.matchMedia('(max-width:700px)')
  isMobile.value = mq.matches
  mq.addEventListener('change', e => { isMobile.value = e.matches })
}
const pendingThemeId = ref('')       // 主题色面板中临时选中但未保存的主题

watch(activeTab, async (tab) => {
  if (tab === 'about' && !appVersion.value) {
    try {
      const res = await apiCall('/api/version')
      appVersion.value = res.version || 'dev'
    } catch { appVersion.value = 'dev' }
  }
})

const form = reactive({
  nick: '', oldPwd: '', newPwd: '',
  hostname: '', logo: '', logoPreview: '', wallpaper: '',
  pubMode: false,
  netMode: 'lan',
  featureSysinfo: false,
  featureProcess: false,
  featureSystemd: false,
  featureDocker:  false,
  showAppName:    true,
  nuName: '', nuPwd: '', nuNick: '',
  display: { hostnameSize: 56, clockSize: 16, iconSize: 78, appNameSize: 12, iconRadius: 26, iconGap: 22, sidePadding: 52 },
  fonts: { hostname: 'system', clock: 'system', appname: 'system', ui: 'system' },
  clock: { show_time: true, show_date: true, show_weekday: true, show_lunar: false, show_seconds: false, show_year: false },
  // 桌面端/移动端独立样式
  dispDevice: 'desktop', // 当前正在编辑哪套: 'desktop' | 'mobile'
  desktop: { hostnameSize: 72, clockSize: 24, iconSize: 64, appNameSize: 14, iconRadius: 25, iconGap: 22, sidePadding: 49,
             fontHostname: 'system', fontClock: 'system', fontAppname: 'system', fontUI: 'system' },
  mobile:  { hostnameSize: 47, clockSize: 17, iconSize: 53, appNameSize: 11, iconRadius: 25, iconGap: 13, sidePadding: 17,
             fontHostname: 'system', fontClock: 'system', fontAppname: 'system', fontUI: 'system' },
})

// 当前正在编辑的那套样式（desktop 或 mobile）
const curDispSet = computed(() => form.dispDevice === 'mobile' ? form.mobile : form.desktop)
// 当前正在编辑的字体套（从 curDispSet 读写，独立于 form.fonts）
const curFontSet = computed(() => ({
  hostname: curDispSet.value.fontHostname,
  clock:    curDispSet.value.fontClock,
  appname:  curDispSet.value.fontAppname,
  ui:       curDispSet.value.fontUI,
}))

const TABS = [
  { id: 'account',   icon: '👤', labelKey: 'niAccount' },
  { id: 'basic',     icon: '⚙️', labelKey: 'niBasic' },
  { id: 'display',   icon: '📐', labelKey: 'niDisplay' },
  { id: 'theme',     icon: '🌈', labelKey: 'niTheme' },
  { id: 'clock',     icon: '🕐', labelKey: 'niClock' },
  { id: 'wallpaper', icon: '🖼️', labelKey: 'niWallpaper' },
  { id: 'users',     icon: '👥', labelKey: 'niUsers' },
  { id: 'backup',    icon: '💾', labelKey: 'niBackup' },
  { id: 'about',     icon: 'ℹ️',  labelKey: 'niAbout' },
]

const sizeSliders = computed(() => {
  const mob = form.dispDevice === 'mobile'
  return [
    { key: 'hostnameSize', label: 'lblHostnameSize', min: mob ? 36 : 56, max: mob ? 58 : 96, unit: 'px' },
    { key: 'clockSize',    label: 'lblClockSize',    min: 10, max: mob ? 24 : 38, unit: 'px' },
    { key: 'iconSize',     label: 'lblIconSize',     min: 44, max: mob ? 62 : 84, unit: 'px' },
    { key: 'appNameSize',  label: 'lblAppnameSize',  min: 6,  max: mob ? 16 : 22, unit: 'px' },
  ]
})
const iconSliders = computed(() => {
  const mob = form.dispDevice === 'mobile'
  return [
    { key: 'iconRadius',  label: 'iconRadius',  min: 0,  max: 50,                unit: '%'  },
    { key: 'iconGap',     label: 'iconGap',     min: 4,  max: mob ? 22 : 36,    unit: 'px' },
    { key: 'sidePadding', label: 'sidePadding', min: mob ? 8 : 32, max: mob ? 26 : 72, unit: 'px' },
  ]
})
const fontFields = [
  { key: 'hostname', label: 'fontHostname' },
  { key: 'clock',    label: 'fontClock' },
  { key: 'appname',  label: 'fontAppname' },
  { key: 'ui',       label: 'fontUi' },
]
// 字体字段在 desktop/mobile 对象里的 key 名映射
const fontFieldMap = {
  hostname: 'fontHostname',
  clock:    'fontClock',
  appname:  'fontAppname',
  ui:       'fontUI',
}
const clockToggles = [
  { key: 'show_time',    label: 'ckTime' },
  { key: 'show_seconds', label: 'ckSec' },
  { key: 'show_year',    label: 'ckYear' },
  { key: 'show_date',    label: 'ckDate', sub: 'ckDateSub' },
  { key: 'show_weekday', label: 'ckWeek' },
  { key: 'show_lunar',   label: 'ckLunar', sub: 'ckLunarSub' },
]

function buildPayload() {
  const d = form.desktop
  const m = form.mobile
  return {
    hostname: form.hostname, logo: form.logo, wallpaper: form.wallpaper,
    clock: { ...form.clock }, theme: curThemeId.value, language: lang.value,
    // 旧版顶层字段（兼容性，取桌面端值）
    hostname_size: +d.hostnameSize, clock_size: +d.clockSize,
    icon_size: +d.iconSize, app_name_size: +d.appNameSize,
    icon_radius: +d.iconRadius, icon_gap: +d.iconGap,
    side_padding: +d.sidePadding,
    font_hostname: d.fontHostname, font_clock: d.fontClock,
    font_appname: d.fontAppname, font_ui: d.fontUI,
    // 双套独立样式
    desktop: {
      hostname_size: +d.hostnameSize, clock_size: +d.clockSize,
      icon_size: +d.iconSize, app_name_size: +d.appNameSize,
      icon_radius: +d.iconRadius, icon_gap: +d.iconGap, side_padding: +d.sidePadding,
      font_hostname: d.fontHostname, font_clock: d.fontClock, font_appname: d.fontAppname, font_ui: d.fontUI,
    },
    mobile: {
      hostname_size: +m.hostnameSize, clock_size: +m.clockSize,
      icon_size: +m.iconSize, app_name_size: +m.appNameSize,
      icon_radius: +m.iconRadius, icon_gap: +m.iconGap, side_padding: +m.sidePadding,
      font_hostname: m.fontHostname, font_clock: m.fontClock, font_appname: m.fontAppname, font_ui: m.fontUI,
    },
    network_mode: form.netMode,
    feature_sysinfo: form.featureSysinfo,
    feature_process: form.featureProcess,
    feature_systemd: form.featureSystemd,
    feature_docker:  form.featureDocker,
    show_app_name:   form.showAppName,
  }
}

async function open() {
  form.nick = props.user?.nickname || ''
  form.oldPwd = ''; form.newPwd = ''
  form.hostname = props.panelInfo?.hostname || ''
  form.logo = props.panelInfo?.logo || ''
  form.logoPreview = props.panelInfo?.logo || ''
  pendingThemeId.value = curThemeId.value  // 打开时同步当前已保存主题
  form.wallpaper = props.panelInfo?.wallpaper || ''
  form.pubMode = props.pubModeValue || false
  form.netMode = props.netModeValue || 'lan'
  form.featureSysinfo = props.featureSysinfo || false
  form.featureProcess = props.featureProcess || false
  form.featureSystemd = props.featureSystemd || false
  form.featureDocker  = props.featureDocker  || false
  form.showAppName    = props.showAppName !== false
  // 初始化双套样式
  const dd = props.desktopDisp || props.dispSet || {}
  const mm = props.mobileDisp  || {}
  Object.assign(form.desktop, {
    hostnameSize: dd.hostname_size || dd.hostnameSize || 72,
    clockSize:    dd.clock_size    || dd.clockSize    || 24,
    iconSize:     dd.icon_size     || dd.iconSize     || 64,
    appNameSize:  dd.app_name_size || dd.appNameSize  || 14,
    iconRadius:   dd.icon_radius   || dd.iconRadius   || 25,
    iconGap:      dd.icon_gap      || dd.iconGap      || 22,
    sidePadding:  dd.side_padding  || dd.sidePadding  || 49,
    fontHostname: dd.font_hostname || dd.fontHostname || 'system',
    fontClock:    dd.font_clock    || dd.fontClock    || 'system',
    fontAppname:  dd.font_appname  || dd.fontAppname  || 'system',
    fontUI:       dd.font_ui       || dd.fontUI       || 'system',
  })
  Object.assign(form.mobile, {
    hostnameSize: mm.hostname_size || mm.hostnameSize || 47,
    clockSize:    mm.clock_size    || mm.clockSize    || 17,
    iconSize:     mm.icon_size     || mm.iconSize     || 53,
    appNameSize:  mm.app_name_size || mm.appNameSize  || 11,
    iconRadius:   mm.icon_radius   || mm.iconRadius   || 25,
    iconGap:      mm.icon_gap      || mm.iconGap      || 13,
    sidePadding:  mm.side_padding  || mm.sidePadding  || 17,
    fontHostname: mm.font_hostname || mm.fontHostname || 'system',
    fontClock:    mm.font_clock    || mm.fontClock    || 'system',
    fontAppname:  mm.font_appname  || mm.fontAppname  || 'system',
    fontUI:       mm.font_ui       || mm.fontUI       || 'system',
  })
  Object.assign(form.clock, props.clkCfg)
  form.nuName = ''; form.nuPwd = ''; form.nuNick = ''
  // 用 setTimeout 延迟到当前点击事件完全结束后再显示 backdrop，避免立即被关闭
  await new Promise(r => setTimeout(r, 0))
  visible.value = true
  loadUsers()
}
function close() { visible.value = false }

async function saveNick() {
  try { await apiCall('/api/me/nickname', { method: 'PUT', body: JSON.stringify({ nickname: form.nick.trim() }) }); emit('toast', t('tNickSaved')); emit('panelUpdated') }
  catch { emit('toast', t('tFailed')) }
}
async function changePwd() {
  if (!form.oldPwd || !form.newPwd) { emit('toast', t('fillPwd')); return }
  try { await apiCall('/api/me/password', { method: 'PUT', body: JSON.stringify({ old_password: form.oldPwd, new_password: form.newPwd }) }); form.oldPwd = ''; form.newPwd = ''; emit('toast', t('tPwdChanged')) }
  catch { emit('toast', t('tPwdFailed')) }
}
async function savePanelCfg() {
  const sv = buildPayload(); sv.hostname = form.hostname.trim(); sv.logo = form.logo.trim()
  try { await apiCall('/api/settings', { method: 'PUT', body: JSON.stringify(sv) }); emit('panelUpdated'); emit('toast', t('tSaved')) }
  catch { emit('toast', t('tFailed')) }
}
async function uploadLogo(e) {
  const f = e.target.files[0]; if (!f) return
  const fd = new FormData(); fd.append('logo', f)
  try { const r = await fetch('/api/upload/logo', { method: 'POST', body: fd, credentials: 'include' }); const d = await r.json(); form.logo = d.url; form.logoPreview = d.url; emit('toast', t('tUploaded')) }
  catch { emit('toast', t('tUploadFailed')) }
  e.target.value = ''
}
async function saveDisplay() {
  const sv = buildPayload()
  try { await apiCall('/api/settings', { method: 'PUT', body: JSON.stringify(sv) }); emit('showAppNameChanged', form.showAppName); emit('panelUpdated'); emit('toast', t('tSaved')) }
  catch { emit('toast', t('tFailed')) }
}
async function saveFonts() {
  const sv = buildPayload()
  try { await apiCall('/api/settings', { method: 'PUT', body: JSON.stringify(sv) }); emit('panelUpdated'); emit('toast', t('tSaved')) }
  catch { emit('toast', t('tFailed')) }
}
function selectTheme(id) {
  pendingThemeId.value = id
  applyThemeCss(THEMES.find(x => x.id === id))  // 实时预览主题色
}
async function saveTheme() {
  curThemeId.value = pendingThemeId.value
  const sv = buildPayload()
  try { await apiCall('/api/settings', { method: 'PUT', body: JSON.stringify(sv) }); emit('toast', t('tSaved')) }
  catch { emit('toast', t('tFailed')); }
}
async function applyThemeAndSave(id) {
  curThemeId.value = id
  applyThemeCss(THEMES.find(x => x.id === id))
  const sv = buildPayload()
  try { await apiCall('/api/settings', { method: 'PUT', body: JSON.stringify(sv) }) }
  catch {}
}
async function saveClk() {
  const sv = buildPayload()
  try { await apiCall('/api/settings', { method: 'PUT', body: JSON.stringify(sv) }); emit('panelUpdated') }
  catch {}
}
function selectWp(url) { form.wallpaper = url }
async function uploadWp(e) {
  const f = e.target.files[0]; if (!f) return
  const fd = new FormData(); fd.append('wallpaper', f)
  try { const r = await fetch('/api/upload/wallpaper', { method: 'POST', body: fd, credentials: 'include' }); const d = await r.json(); form.wallpaper = d.url; emit('toast', t('tUploaded')) }
  catch { emit('toast', t('tUploadFailed')) }
  e.target.value = ''
}
async function saveWp() {
  const sv = buildPayload(); sv.wallpaper = form.wallpaper.trim()
  try { await apiCall('/api/settings', { method: 'PUT', body: JSON.stringify(sv) }); emit('panelUpdated'); emit('toast', t('tSaved')) }
  catch { emit('toast', t('tFailed')) }
}
async function loadUsers() {
  try { users.value = await apiCall('/api/users') } catch {}
}
async function addUser() {
  if (!form.nuName.trim() || !form.nuPwd) { emit('toast', t('fillAccountPwd')); return }
  try { await apiCall('/api/users', { method: 'POST', body: JSON.stringify({ username: form.nuName.trim(), password: form.nuPwd, nickname: form.nuNick.trim() }) }); form.nuName = ''; form.nuPwd = ''; form.nuNick = ''; await loadUsers(); emit('toast', t('tUserAdded')) }
  catch { emit('toast', t('tUserAddFailed')) }
}
async function removeUser(username) {
  if (!confirm(`${t('confirmDeleteUser')} ${username}？`)) return
  try { await apiCall(`/api/users/${username}`, { method: 'DELETE' }); await loadUsers(); emit('toast', t('tDeleted')) }
  catch { emit('toast', t('tDeleteFailed')) }
}
async function toggleFeature(key) {
  // v-model 已将 form[key] 更新为新值，直接保存即可，失败时再回滚
  const sv = buildPayload()
  try {
    await apiCall('/api/settings', { method: 'PUT', body: JSON.stringify(sv) })
    emit('featuresChanged', {
      sysinfo: form.featureSysinfo,
      process: form.featureProcess,
      systemd: form.featureSystemd,
      docker:  form.featureDocker,
    })
  } catch { emit('toast', t('tFailed')); form[key] = !form[key] }
}
async function setNetMode(mode) {
  form.netMode = mode
  const sv = buildPayload(); sv.network_mode = mode
  try {
    await apiCall('/api/settings', { method: 'PUT', body: JSON.stringify(sv) })
    emit('netModeChanged', mode)
    emit('toast', mode === 'lan' ? t('netSwitchedLan') : t('netSwitchedWan'))
  } catch { emit('toast', t('tFailed')); form.netMode = mode === 'lan' ? 'wan' : 'lan' }
}
async function setPubMode() {
  const v = form.pubMode
  try { await apiCall('/api/publicmode', { method: 'PUT', body: JSON.stringify({ public_mode: v }) }); emit('toast', v ? t('pubOn') : t('pubOff')); emit('panelUpdated') }
  catch { emit('toast', t('tFailed')); form.pubMode = !v }
}
async function setLang(l) {
  lang.value = l; localStorage.setItem('ep_lang', l)
  const sv = buildPayload(); sv.language = l
  await apiCall('/api/settings', { method: 'PUT', body: JSON.stringify(sv) }).catch(() => {})
  emit('toast', t('tSaved'))
}
async function exportData() {
  try {
    const [appsData, settingsData] = await Promise.all([apiCall('/api/apps'), apiCall('/api/settings')])
    const backup = { version: 1, exported_at: new Date().toISOString(), apps: appsData, settings: settingsData, theme: curThemeId.value }
    const blob = new Blob([JSON.stringify(backup, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a'); a.href = url; a.download = `helm_backup_${new Date().toISOString().slice(0, 10)}.json`; a.click()
    URL.revokeObjectURL(url); emit('toast', t('tExported'))
  } catch { emit('toast', t('tExportFailed')) }
}
async function importData(e) {
  const f = e.target.files[0]; if (!f) return
  if (!confirm(t('confirmImport'))) { e.target.value = ''; return }
  try {
    const text = await f.text(); const b = JSON.parse(text)
    if (!b.version || !b.apps) { emit('toast', t('badBackup')); e.target.value = ''; return }
    if (b.settings) await apiCall('/api/settings', { method: 'PUT', body: JSON.stringify(b.settings) })
    for (const app of props.apps || []) await apiCall(`/api/apps/${app.id}`, { method: 'DELETE' }).catch(() => {})
    for (const app of b.apps) { const { id: _, order: __, ...rest } = app; await apiCall('/api/apps', { method: 'POST', body: JSON.stringify(rest) }) }
    if (b.theme) { const th = THEMES.find(x => x.id === b.theme); if (th) { curThemeId.value = th.id; applyThemeCss(th) } }
    emit('panelUpdated'); emit('toast', t('tImported'))
  } catch { emit('toast', t('tImportFailed')) }
  e.target.value = ''
}

defineExpose({ open, close })
</script>

<style scoped>
.s-backdrop { position: fixed; inset: 0; z-index: 750; background: rgba(0,0,0,.28); }
.s-panel { position: fixed; inset: 0; z-index: 760; display: flex; align-items: center; justify-content: center; padding: 20px; }
.s-box { position: relative; background: white; border-radius: 22px; width: min(940px,100%); height: min(690px,90vh); display: flex; overflow: hidden; box-shadow: 0 32px 80px rgba(168,85,247,.2); }
.s-close-top { position: absolute; top: 14px; right: 14px; width: 22px; height: 22px; border-radius: 50%; background: #ff5f57; border: none; cursor: pointer; display: flex; align-items: center; justify-content: center; transition: all var(--tr); box-shadow: 0 2px 6px rgba(255,95,87,.45); z-index: 10; }
.s-close-top:hover::after { content: '✕'; font-size: 11px; font-weight: 700; color: rgba(0,0,0,.5); }
.s-nav { width: 215px; flex-shrink: 0; background: linear-gradient(180deg,#faf5ff 0%,#fdf2f8 100%); border-right: 1px solid #ede8f5; display: flex; flex-direction: column; overflow-y: auto; position: relative; }
.s-nav-header { padding: 20px 18px 14px; display: flex; align-items: center; gap: 10px; }
.s-nav-ico { width: 32px; height: 32px; background: var(--grad); border-radius: 9px; display: flex; align-items: center; justify-content: center; flex-shrink: 0; box-shadow: 0 4px 12px color-mix(in srgb,var(--h1) 35%,transparent); }
.s-nav-t { font-size: 14px; font-weight: 800; background: var(--grad); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text; }
.s-nav-list { padding: 8px; flex: 1; display: flex; flex-direction: column; }
.s-ni { display: flex; align-items: center; gap: 9px; padding: 10px 13px; border-radius: 11px; cursor: pointer; font-size: 13px; font-weight: 600; color: #6b7280; transition: all var(--tr); margin-bottom: 3px; }
.s-ni:hover { background: rgba(168,85,247,.07); color: #1e1b2e; }
.s-ni.active { background: var(--grad); color: white; box-shadow: 0 4px 14px color-mix(in srgb,var(--h1) 30%,transparent); }
.s-ico { font-size: 16px; width: 22px; text-align: center; }
.s-content { flex: 1; overflow-y: auto; padding: 28px 30px; }
.theme-grid { display: grid; grid-template-columns: repeat(4,1fr); gap: 10px; margin-bottom: 4px; }
.theme-item { border-radius: 13px; padding: 12px 8px; cursor: pointer; transition: all var(--tr); background: white; border: 2px solid #ede8f5; text-align: center; }
.theme-item:hover { transform: scale(1.04); border-color: var(--h1); }
.theme-item.sel { border-color: var(--h1); box-shadow: 0 4px 14px color-mix(in srgb,var(--h1) 25%,transparent); }
.theme-item.saved { border-style: dashed; border-color: var(--h1); opacity: 0.6; }
.theme-dot { width: 36px; height: 36px; border-radius: 50%; margin: 0 auto 8px; box-shadow: 0 3px 10px rgba(0,0,0,.15); }
.theme-name { font-size: 11px; font-weight: 600; color: #4b5563; }
.u-table { width: 100%; border-collapse: collapse; }
.u-table th { font-size: 11px; font-weight: 700; color: #94a3b8; text-transform: uppercase; letter-spacing: .5px; padding: 8px 13px; text-align: left; background: #f5f3ff; border-bottom: 1px solid #ede8f5; }
.u-table th:first-child { border-radius: 10px 0 0 0; } .u-table th:last-child { border-radius: 0 10px 0 0; }
.u-table td { padding: 12px 13px; border-bottom: 1px solid #faf5ff; font-size: 14px; color: #1e1b2e; }
.u-table tr:last-child td { border-bottom: none; }
.wp-grid { display: grid; grid-template-columns: repeat(3,1fr); gap: 9px; margin-bottom: 14px; }
.wp-thumb { aspect-ratio: 16/9; border-radius: 11px; cursor: pointer; overflow: hidden; border: 3px solid transparent; transition: all var(--tr); }
.wp-thumb:hover, .wp-thumb.sel { border-color: var(--h1); transform: scale(1.03); box-shadow: 0 4px 14px color-mix(in srgb,var(--h1) 25%,transparent); }
.wp-thumb img { width: 100%; height: 100%; object-fit: cover; display: block; }
.backup-btns { display: flex; gap: 10px; flex-wrap: wrap; }
.backup-btn { flex: 1; min-width: 140px; padding: 13px; border-radius: 12px; border: 2px dashed #d8b4fe; background: #faf5ff; cursor: pointer; font-size: 14px; font-weight: 600; color: #7c3aed; transition: all var(--tr); display: flex; flex-direction: column; align-items: center; gap: 6px; }
.backup-btn:hover { background: #f3e8ff; border-color: var(--h1); transform: translateY(-2px); }
.b-ico { font-size: 24px; } .b-sub { font-size: 11px; color: #94a3b8; font-weight: 500; }
.net-toggle { display: flex; gap: 4px; }
.net-btn { padding: 7px 16px; border: 1.5px solid #ede8f5; border-radius: 9px; background: white; cursor: pointer; font-size: 13px; font-weight: 600; color: #6b7280; transition: all var(--tr); }
.net-btn:hover { border-color: var(--h1); color: var(--h1); }
.net-btn.active { background: var(--grad); color: white; border-color: transparent; box-shadow: 0 3px 10px color-mix(in srgb,var(--h1) 30%,transparent); }
.lang-btns { display: flex; gap: 8px; }
.lang-btn { flex: 1; padding: 9px 14px; border: 2px solid #ede8f5; border-radius: 11px; background: white; cursor: pointer; font-size: 14px; font-weight: 700; color: #6b7280; transition: all var(--tr); text-align: center; }
.lang-btn:hover { border-color: var(--h1); color: var(--h1); }
.lang-btn.active { border-color: var(--h1); color: var(--h1); background: #faf5ff; }
.about-wrap { display: flex; flex-direction: column; align-items: center; justify-content: flex-start; padding: 80px 24px 48px; gap: 10px; }
.about-logo-wrap { width: 64px; height: 64px; border-radius: 18px; overflow: hidden; box-shadow: 0 8px 28px rgba(99,102,241,.3); margin-bottom: 6px; }
.about-logo-fallback { width: 100%; height: 100%; background: var(--grad); display: flex; align-items: center; justify-content: center; font-size: 28px; font-weight: 900; color: white; }
.about-name { font-size: 22px; font-weight: 900; background: var(--grad); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text; letter-spacing: -0.5px; }
.about-version { font-size: 14px; font-weight: 700; color: white; background: var(--grad); padding: 3px 14px; border-radius: 20px; }
.about-desc { font-size: 13px; color: #94a3b8; margin-top: 2px; }
.about-link { margin-top: 18px; font-size: 13px; font-weight: 700; color: var(--h1); text-decoration: none; padding: 8px 22px; border: 1.5px solid var(--h1); border-radius: 10px; transition: all .15s; display: inline-flex; align-items: center; gap: 6px; }
.about-link:hover { background: var(--h1); color: white; }
/* 设备切换 tab */
.device-tab-bar { display: flex; gap: 0; margin-bottom: 10px; border-radius: 12px; overflow: hidden; background: #f0f0f8; border: 1.5px solid #e8e0f5; }
.device-tab { flex: 1; padding: 8px 0; font-size: 13px; font-weight: 600; border: none; background: transparent; cursor: pointer; color: #94a3b8; transition: all .18s; display: flex; align-items: center; justify-content: center; }
.device-tab.active { background: var(--grad); color: white; border-radius: 10px; box-shadow: 0 2px 8px color-mix(in srgb,var(--h1) 30%,transparent); }
.device-tab:not(.active):hover { background: rgba(168,85,247,.08); color: var(--h1); }

/* ── 移动端顶部导航栏（默认隐藏） ── */
.mob-topbar { display: none; }
.mob-drawer { display: none; }
.mob-drawer-mask { display: none; }

@media (max-width: 700px) {
  /* 整体布局：全屏竖向 */
  .s-panel { padding: 0; align-items: flex-end; }
  .s-box { border-radius: 20px 20px 0 0; width: 100%; height: 94vh; flex-direction: column; }
  .s-close-top { top: 10px; right: 12px; width: 26px; height: 26px; }

  /* 桌面侧边栏隐藏 */
  .s-nav { display: none; }

  /* 移动端顶部栏 */
  .mob-topbar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 16px 12px;
    border-bottom: 1px solid #ede8f5;
    flex-shrink: 0;
    background: linear-gradient(135deg, #faf5ff 0%, #fdf2f8 100%);
    border-radius: 20px 20px 0 0;
  }
  .mob-hamburger {
    width: 36px; height: 36px;
    border: none; border-radius: 10px;
    background: var(--grad);
    display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    gap: 4px; cursor: pointer; flex-shrink: 0;
    box-shadow: 0 3px 10px color-mix(in srgb,var(--h1) 30%,transparent);
  }
  .mob-hamburger span {
    display: block; width: 16px; height: 2px;
    background: white; border-radius: 2px;
  }
  .mob-topbar-title {
    font-size: 15px; font-weight: 800;
    background: var(--grad);
    -webkit-background-clip: text; -webkit-text-fill-color: transparent;
    background-clip: text;
    flex: 1;
  }

  /* 内容区充满剩余空间 */
  .s-content { flex: 1; padding: 18px 18px 32px; }

  /* 抽屉遮罩 */
  .mob-drawer-mask {
    display: block;
    position: absolute; inset: 0;
    background: rgba(0,0,0,.25);
    z-index: 10;
    border-radius: 20px 20px 0 0;
  }

  /* 抽屉导航 */
  .mob-drawer {
    display: flex;
    flex-direction: column;
    position: absolute;
    top: 0; left: 0; right: 0; bottom: 0;
    background: linear-gradient(180deg, #faf5ff 0%, #fdf2f8 100%);
    z-index: 20;
    border-radius: 20px 20px 0 0;
    overflow-y: auto;
    transform: translateY(-100%);
    transition: transform .25s cubic-bezier(.4,0,.2,1);
  }
  .mob-drawer.open { transform: translateY(0); }
  /* 抽屉内部专属样式 */
  .mob-drawer-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 18px 16px 14px;
    border-bottom: 1px solid #ede8f5;
    flex-shrink: 0;
  }
  .mob-drawer-ico {
    width: 32px; height: 32px;
    background: var(--grad); border-radius: 9px;
    display: flex; align-items: center; justify-content: center;
    flex-shrink: 0;
  }
  .mob-drawer-title {
    font-size: 15px; font-weight: 800;
    background: var(--grad);
    -webkit-background-clip: text; -webkit-text-fill-color: transparent;
    background-clip: text;
  }
  .mob-nav-list {
    display: flex;
    flex-direction: column;
    padding: 10px 12px 24px;
    gap: 2px;
  }
  .mob-ni {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 12px;
    padding: 13px 16px;
    border-radius: 12px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 600;
    color: #6b7280;
    transition: all .15s;
    width: 100%;
  }
  .mob-ni:hover { background: rgba(168,85,247,.07); color: #1e1b2e; }
  .mob-ni.active { background: var(--grad); color: white; }
  .mob-ni-ico { font-size: 18px; width: 24px; text-align: center; flex-shrink: 0; }
  .mob-ni-lbl { flex: 1; }
}</style>

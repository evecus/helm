<template>
  <div class="modal-overlay" v-if="visible" @click.self="close">
    <div class="modal">
      <div class="modal-header">
        <div class="modal-title">{{ editId ? t('editApp') : t('addApp') }}</div>
        <button class="close-btn" @click="close">✕</button>
      </div>

      <!-- Preview -->
      <div class="preview-box">
        <div class="preview-ico">
          <img v-if="iconType === 'image' && iconUrl" :src="iconUrl"
            style="width:100%;height:100%;object-fit:cover;border-radius:12px"
            @error="e => e.target.style.display = 'none'" />
          <template v-else>{{ previewIconText }}</template>
        </div>
        <div>
          <div class="preview-t">{{ appTitle || t('appNamePrev') }}</div>
          <div class="preview-u">{{ previewUrl || 'https://...' }}</div>
        </div>
      </div>

      <!-- Icon type tabs -->
      <div class="fr">
        <label class="fl">{{ t('iconStyle') }}</label>
        <div class="itabs">
          <button class="itab" :class="{ active: iconType === 'text' }" @click="iconType = 'text'">{{ t('textIcon') }}</button>
          <button class="itab" :class="{ active: iconType === 'image' }" @click="iconType = 'image'">{{ t('imageIcon') }}</button>
        </div>
        <div v-if="iconType === 'text'">
          <input type="text" class="fi2" v-model="iconTxt" :placeholder="t('iconPlaceholder')" maxlength="2" />
        </div>
        <div v-else class="img-row">
          <input type="text" class="fi2" v-model="iconUrl" :placeholder="t('iconUrlPlaceholder')" />
          <button class="upbtn" @click="fetchIcon" :disabled="fetchingIcon" :title="t('fetchIconBtn')">
            {{ fetchingIcon ? '...' : '🔍' }}
          </button>
          <label class="upbtn" for="icon-file-inp">📁</label>
          <input type="file" id="icon-file-inp" accept="image/*" style="display:none" @change="uploadIcon" />
        </div>
      </div>

      <div class="fr"><label class="fl">{{ t('titleLbl') }}</label><input type="text" class="fi2" v-model="appTitle" maxlength="20" /></div>

      <!-- 内网地址 -->
      <div class="fr">
        <label class="fl">{{ t('urlLanLbl') }}</label>
        <input type="text" class="fi2" v-model="appUrlLan" placeholder="10.0.0.1:8080 或 http://10.0.0.1:8080" />
      </div>

      <!-- 公网地址 -->
      <div class="fr">
        <label class="fl">{{ t('urlWanLbl') }}</label>
        <input type="text" class="fi2" v-model="appUrlWan" placeholder="example.com 或 https://example.com" />
      </div>

      <div class="url-hint">{{ t('urlHint') }}</div>

      <div class="fr">
        <label class="fl">{{ t('openLbl') }}</label>
        <select class="fi2" v-model="appOpen">
          <option value="new_tab">{{ t('newTab') }}</option>
          <option value="current">{{ t('currentTab') }}</option>
        </select>
      </div>

      <div class="mfooter">
        <button v-if="editId" class="btn btn-d" @click="deleteApp">{{ t('deleteBtn') }}</button>
        <button class="btn btn-s" @click="close">{{ t('cancelBtn') }}</button>
        <button class="btn btn-p" @click="save">{{ t('saveBtn') }}</button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { useI18n } from '../composables/useI18n.js'
import { apiCall } from '../composables/useApi.js'

const { t } = useI18n()
const emit = defineEmits(['saved', 'deleted', 'toast'])

const visible = ref(false)
const editId = ref(null)
const iconType = ref('text')
const iconTxt = ref('')
const iconUrl = ref('')
const appTitle = ref('')
const appUrlLan = ref('')
const appUrlWan = ref('')
const appOpen = ref('new_tab')
const fetchingIcon = ref(false)

// 地址处理：有协议头原样保存，无协议头用 // 前缀（继承当前页面协议）
function normalizeUrl(url) {
  const u = url.trim()
  if (!u) return ''
  if (/^https?:\/\//i.test(u)) return u   // 有 http:// 或 https://，原样
  if (u.startsWith('//')) return u         // 已经是协议相对URL
  return '//' + u                          // 裸地址，加 // 前缀
}

const previewUrl = computed(() => appUrlLan.value || appUrlWan.value)

const previewIconText = computed(() => {
  if (iconType.value === 'text') return (iconTxt.value || (appTitle.value || '?').substring(0, 2)).substring(0, 2)
  return (appTitle.value || '?').substring(0, 2)
})

function openAdd() {
  editId.value = null
  iconType.value = 'text'; iconTxt.value = ''; iconUrl.value = ''
  appTitle.value = ''; appUrlLan.value = ''; appUrlWan.value = ''; appOpen.value = 'new_tab'
  visible.value = true
}

function openEdit(app) {
  editId.value = app.id
  iconType.value = app.icon_type || 'text'
  iconTxt.value = app.icon_text || ''
  iconUrl.value = app.icon_image || ''
  appTitle.value = app.title || ''
  // 兼容旧数据：旧的 url 字段迁移到 url_lan
  appUrlLan.value = app.url_lan || app.url || ''
  appUrlWan.value = app.url_wan || ''
  appOpen.value = app.open_type || 'new_tab'
  visible.value = true
}

function close() { visible.value = false }

async function save() {
  const title = appTitle.value.trim()
  const lan = appUrlLan.value.trim()
  const wan = appUrlWan.value.trim()
  if (!title) { emit('toast', t('fillTitle')); return }
  if (!lan && !wan) { emit('toast', t('fillAtLeastOneUrl')); return }
  const data = {
    title,
    url: '',           // 清空旧字段
    url_lan: normalizeUrl(lan),
    url_wan: normalizeUrl(wan),
    icon_type: iconType.value,
    icon_text: iconTxt.value,
    icon_image: iconUrl.value,
    open_type: appOpen.value
  }
  try {
    if (editId.value) await apiCall(`/api/apps/${editId.value}`, { method: 'PUT', body: JSON.stringify(data) })
    else await apiCall('/api/apps', { method: 'POST', body: JSON.stringify(data) })
    close()
    emit('saved')
    emit('toast', t('tSaved'))
  } catch { emit('toast', t('tFailed')) }
}

async function deleteApp() {
  if (!editId.value || !confirm(t('confirmDelete'))) return
  try {
    await apiCall(`/api/apps/${editId.value}`, { method: 'DELETE' })
    close()
    emit('deleted')
    emit('toast', t('tDeleted'))
  } catch { emit('toast', t('tDeleteFailed')) }
}

async function fetchIcon() {
  // 取内网或公网地址中第一个有值的
  const addr = (appUrlLan.value || appUrlWan.value).trim()
  if (!addr) { emit('toast', t('fetchIconNoUrl')); return }
  fetchingIcon.value = true
  try {
    const res = await fetch(`/api/fetch-icon?url=${encodeURIComponent(addr)}`)
    const d = await res.json()
    if (d.icon) { iconUrl.value = d.icon; emit('toast', t('fetchIconOk')) }
    else emit('toast', t('fetchIconFail'))
  } catch { emit('toast', t('fetchIconFail')) }
  finally { fetchingIcon.value = false }
}

async function uploadIcon(e) {
  const f = e.target.files[0]; if (!f) return
  const fd = new FormData(); fd.append('image', f)
  try {
    const res = await fetch('/api/upload', { method: 'POST', body: fd, credentials: 'include' })
    const d = await res.json()
    iconUrl.value = d.url
    emit('toast', t('tUploaded'))
  } catch { emit('toast', t('tUploadFailed')) }
  e.target.value = ''
}

defineExpose({ openAdd, openEdit })
</script>

<style scoped>
.modal-overlay {
  position: fixed; inset: 0; z-index: 700;
  background: rgba(0,0,0,.4); backdrop-filter: blur(6px);
  display: flex; align-items: center; justify-content: center; padding: 16px;
}
.modal {
  background: white; border-radius: 22px; padding: 28px;
  width: 460px; max-width: 100%; max-height: 90vh; overflow-y: auto;
  box-shadow: 0 24px 64px rgba(168,85,247,.18); animation: pop-in .2s ease;
  border: 1px solid rgba(168,85,247,.1);
}
@keyframes pop-in { from { transform: scale(.93) translateY(12px); opacity: 0 } to { transform: scale(1) translateY(0); opacity: 1 } }
.modal-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 22px; }
.modal-title { font-size: 17px; font-weight: 800; background: var(--grad); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text; }
.close-btn { width: 32px; height: 32px; border: none; background: #f5f3ff; border-radius: 9px; cursor: pointer; font-size: 15px; display: flex; align-items: center; justify-content: center; color: #64748b; transition: all var(--tr); }
.close-btn:hover { background: #ede9fe; }
.preview-box { background: linear-gradient(135deg,#faf5ff,#fdf2f8); border-radius: 14px; padding: 14px; margin-bottom: 20px; display: flex; align-items: center; gap: 14px; border: 1px solid rgba(168,85,247,.1); }
.preview-ico { width: 52px; height: 52px; border-radius: 14px; overflow: hidden; background: var(--grad); display: flex; align-items: center; justify-content: center; font-size: 20px; font-weight: 800; color: white; flex-shrink: 0; }
.preview-t { font-size: 14px; font-weight: 700; color: #1e1b2e; }
.preview-u { font-size: 12px; color: #94a3b8; margin-top: 2px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 260px; }
.fl { display: block; font-size: 11px; font-weight: 700; color: #94a3b8; margin-bottom: 5px; text-transform: uppercase; letter-spacing: .6px; }
.fi2 { width: 100%; padding: 11px 13px; border: 1.5px solid #ede8f5; border-radius: 11px; font-size: 14px; color: #1e1b2e; outline: none; transition: all var(--tr); background: #faf8ff; font-family: inherit; }
.fi2:focus { border-color: var(--h1); background: white; box-shadow: 0 0 0 3px color-mix(in srgb, var(--h1) 10%, transparent); }
.fr { margin-bottom: 14px; }
.itabs { display: flex; gap: 3px; background: #f5f3ff; border-radius: 11px; padding: 3px; margin-bottom: 10px; }
.itab { flex: 1; padding: 8px; border: none; background: transparent; border-radius: 8px; cursor: pointer; font-size: 13px; font-weight: 600; color: #94a3b8; transition: all var(--tr); }
.itab.active { background: white; color: var(--h1); box-shadow: 0 2px 8px rgba(168,85,247,.15); }
.img-row { display: flex; gap: 7px; align-items: center; }
.img-row .fi2 { flex: 1; }
.upbtn { padding: 11px 13px; border: 1.5px solid #ede8f5; border-radius: 11px; background: white; cursor: pointer; font-size: 13px; font-weight: 600; color: #94a3b8; white-space: nowrap; transition: all var(--tr); display: flex; align-items: center; gap: 5px; }
.upbtn:hover { border-color: var(--h1); color: var(--h1); }
.url-hint { font-size: 11px; color: #94a3b8; margin: -8px 0 14px; line-height: 1.5; }
.mfooter { display: flex; justify-content: flex-end; gap: 8px; margin-top: 20px; padding-top: 16px; border-top: 1px solid #f5f3ff; }
</style>

<template>
  <div id="ctx-menu" v-if="visible" class="ctx"
    :style="{ left: x + 'px', top: y + 'px' }">
    <template v-if="mode === 'app'">
      <div class="ctx-item" @mousedown.stop="onEdit">
        <span class="ctx-ico">✏️</span><span>{{ t('editLabel') }}</span>
      </div>
      <div class="ctx-item danger" @mousedown.stop="onDelete">
        <span class="ctx-ico">🗑</span><span>{{ t('deleteBtn') }}</span>
      </div>
    </template>
    <template v-else>
      <div class="ctx-item" @mousedown.stop="onAdd">
        <span class="ctx-ico">✏️</span><span>{{ t('addAppShort') }}</span>
      </div>
      <div class="ctx-item" @mousedown.stop="onSort">
        <span class="ctx-ico sort-ico">⠿</span><span>{{ t('sortApps') }}</span>
      </div>
    </template>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { useI18n } from '../composables/useI18n.js'

const { t } = useI18n()
const emit = defineEmits(['edit', 'delete', 'add', 'sort'])

const visible = ref(false)
const x = ref(0), y = ref(0)
const mode = ref('app')
let currentId = null
let autoHideTimer = null

function startAutoHide() {
  clearTimeout(autoHideTimer)
  autoHideTimer = setTimeout(() => { hide() }, 5000)
}

function show(clientX, clientY, id) {
  currentId = id
  mode.value = 'app'
  _position(clientX, clientY)
  visible.value = true
  startAutoHide()
}
function showPanel(clientX, clientY) {
  currentId = null
  mode.value = 'panel'
  _position(clientX, clientY)
  visible.value = true
  startAutoHide()
}
function _position(clientX, clientY) {
  const w = 150, h = 90
  x.value = clientX + w > window.innerWidth  ? clientX - w : clientX
  y.value = clientY + h > window.innerHeight ? clientY - h : clientY
}
function hide() {
  clearTimeout(autoHideTimer)
  visible.value = false
  currentId = null
}

function onEdit()   { const id = currentId; hide(); emit('edit', id) }
function onDelete() { const id = currentId; hide(); emit('delete', id) }
function onAdd()    { hide(); emit('add') }
function onSort()   { hide(); emit('sort') }

defineExpose({ show, hide, showPanel })
</script>

<style scoped>
.ctx {
  position: fixed; z-index: 9999;
  background: rgba(255,255,255,.97); backdrop-filter: blur(20px);
  border-radius: 14px;
  box-shadow: 0 12px 40px rgba(0,0,0,.18), 0 1px 0 rgba(255,255,255,.8) inset;
  padding: 5px; min-width: 140px;
  border: 1px solid rgba(255,255,255,.6);
  animation: pop-in .14s ease;
}
.ctx-item {
  display: flex; align-items: center;
  padding: 10px 14px; border-radius: 9px;
  cursor: pointer; font-size: 14px; font-weight: 500;
  color: #1e1b2e; transition: background var(--tr); user-select: none;
}
.ctx-item:hover { background: rgba(168,85,247,.08); }
.ctx-item.danger { color: #ef4444; }
.ctx-item.danger:hover { background: #fef2f2; }
.ctx-ico {
  width: 22px; flex-shrink: 0;
  display: flex; align-items: center; justify-content: center;
  font-size: 15px; margin-right: 6px;
}
.sort-ico { font-size: 18px; letter-spacing: 0; }
</style>

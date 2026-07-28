<template>
  <div class="login-overlay" v-if="visible">
    <div class="login-card">
      <div class="login-title"><span>Helm</span></div>
      <div class="login-sub">{{ subtitle }}</div>
      <div class="fg">
        <span class="fi">👤</span>
        <input ref="userInput" type="text" v-model="username" :placeholder="t('lblUsername')"
          autocomplete="username" @keydown.enter="submit" />
      </div>
      <div class="fg">
        <span class="fi">🔒</span>
        <input type="password" v-model="password" placeholder="Password"
          autocomplete="current-password" @keydown.enter="submit" />
      </div>
      <p v-if="error" class="login-err">{{ t('loginErr') }}</p>
      <button class="login-btn" @click="submit">{{ t('loginBtn') }}</button>
      <p class="login-foot">Powered By Helm</p>
    </div>
  </div>
</template>

<script setup>
import { ref, nextTick } from 'vue'
import { useI18n } from '../composables/useI18n.js'
import { apiCall } from '../composables/useApi.js'

const { t } = useI18n()

const visible = ref(false)
const subtitle = ref('')
const username = ref('')
const password = ref('')
const error = ref(false)
const userInput = ref(null)

let resolveCb = null

function open(sub, onSuccess) {
  subtitle.value = sub || t('loginSub')
  username.value = ''
  password.value = ''
  error.value = false
  resolveCb = onSuccess || null
  visible.value = true
  nextTick(() => userInput.value?.focus())
}

function close() {
  visible.value = false
  resolveCb = null
}

async function submit() {
  error.value = false
  try {
    const res = await apiCall('/api/login', {
      method: 'POST',
      body: JSON.stringify({ username: username.value.trim(), password: password.value }),
    })
    close()
    resolveCb?.(res)
  } catch {
    error.value = true
  }
}

defineExpose({ open, close })
</script>

<style scoped>
.login-overlay {
  position: fixed; inset: 0; z-index: 800;
  display: flex; align-items: center; justify-content: center;
  background: rgba(0,0,0,.38); backdrop-filter: blur(12px);
}
.login-card {
  background: rgba(255,255,255,.9); backdrop-filter: blur(30px);
  border-radius: 24px; padding: 44px 42px 30px; width: 380px; max-width: 92vw;
  box-shadow: 0 32px 80px rgba(168,85,247,.18), 0 2px 0 rgba(255,255,255,.8) inset;
  border: 1px solid rgba(255,255,255,.7); animation: pop-in .24s ease;
}
@keyframes pop-in { from { transform: scale(.93) translateY(12px); opacity: 0 } to { transform: scale(1) translateY(0); opacity: 1 } }
.login-logo-wrap {
  width: 48px; height: 48px; background: var(--grad); border-radius: 14px;
  display: flex; align-items: center; justify-content: center; margin-bottom: 18px;
  box-shadow: 0 6px 20px color-mix(in srgb, var(--h1) 40%, transparent);
}
.login-title { font-size: 36px; font-weight: 900; text-align: center; margin-bottom: 6px; }
.login-title span { background: var(--grad); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text; }
.login-sub { font-size: 13px; color: #94a3b8; text-align: center; margin-bottom: 24px; }
.fg { position: relative; margin-bottom: 12px; }
.fg input { width: 100%; padding: 13px 14px 13px 42px; border: 1.5px solid #ede8f5; border-radius: 13px; font-size: 15px; background: rgba(250,248,255,.8); outline: none; color: #1e1b2e; transition: all var(--tr); font-family: inherit; }
.fg input:focus { border-color: var(--h1); background: white; box-shadow: 0 0 0 3px color-mix(in srgb, var(--h1) 12%, transparent); }
.fg .fi { position: absolute; left: 13px; top: 50%; transform: translateY(-50%); font-size: 16px; }
.login-btn { width: 100%; padding: 14px; background: var(--grad); border: none; border-radius: 13px; font-size: 15px; font-weight: 700; cursor: pointer; color: white; box-shadow: 0 6px 20px color-mix(in srgb, var(--h1) 35%, transparent); transition: all var(--tr); margin-top: 4px; }
.login-btn:hover { transform: translateY(-2px); }
.login-btn:active { transform: translateY(0); }
.login-err { color: #ef4444; font-size: 13px; text-align: center; margin-top: 8px; }
.login-foot { text-align: center; margin-top: 18px; font-size: 12px; color: #94a3b8; }
</style>

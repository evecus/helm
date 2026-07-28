<template>
  <div class="toast" :class="{ show: visible }">{{ message }}</div>
</template>

<script setup>
import { ref } from 'vue'

const message = ref('')
const visible = ref(false)
let timer = null

function show(msg) {
  message.value = msg
  visible.value = true
  clearTimeout(timer)
  timer = setTimeout(() => { visible.value = false }, 2400)
}

defineExpose({ show })
</script>

<style scoped>
.toast {
  position: fixed;
  bottom: 30px;
  left: 50%;
  transform: translateX(-50%) translateY(120px);
  background: rgba(15,5,30,.82);
  color: white;
  padding: 10px 22px;
  border-radius: 24px;
  font-size: 13px;
  z-index: 9999;
  transition: transform .26s;
  backdrop-filter: blur(12px);
  white-space: nowrap;
  pointer-events: none;
  box-shadow: 0 8px 24px rgba(0,0,0,.2);
}
.toast.show { transform: translateX(-50%) translateY(0); }
</style>

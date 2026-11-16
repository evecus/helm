import { ref, computed, watch } from 'vue'

export const THEMES = [
  { id: 'purple-pink',   name: '紫粉',   h1: '#a855f7', h2: '#ec4899', dot: 'linear-gradient(135deg,#a855f7,#ec4899)', wp: 'https://images.unsplash.com/photo-1579546929518-9e396f3cc809?w=1920&q=80' },
  { id: 'blue-cyan',     name: '蓝青',   h1: '#3b82f6', h2: '#06b6d4', dot: 'linear-gradient(135deg,#3b82f6,#06b6d4)', wp: 'https://images.unsplash.com/photo-1419242902214-272b3f66ee7a?w=1920&q=80' },
  { id: 'green-teal',    name: '绿青',   h1: '#22c55e', h2: '#14b8a6', dot: 'linear-gradient(135deg,#22c55e,#14b8a6)', wp: 'https://images.unsplash.com/photo-1501854140801-50d01698950b?w=1920&q=80' },
  { id: 'orange-red',    name: '橙红',   h1: '#f97316', h2: '#ef4444', dot: 'linear-gradient(135deg,#f97316,#ef4444)', wp: 'https://images.unsplash.com/photo-1470071459604-3b5ec3a7fe05?w=1920&q=80' },
  { id: 'dark-blue',     name: '深蓝',   h1: '#1d4ed8', h2: '#7c3aed', dot: 'linear-gradient(135deg,#1d4ed8,#7c3aed)', wp: 'https://images.unsplash.com/photo-1519681393784-d120267933ba?w=1920&q=80' },
  { id: 'rose-gold',     name: '玫瑰金', h1: '#f43f5e', h2: '#fb923c', dot: 'linear-gradient(135deg,#f43f5e,#fb923c)', wp: 'https://images.unsplash.com/photo-1506905925346-21bda4d32df4?w=1920&q=80' },
  { id: 'indigo-violet', name: '靛紫',   h1: '#6366f1', h2: '#8b5cf6', dot: 'linear-gradient(135deg,#6366f1,#8b5cf6)', wp: 'https://images.unsplash.com/photo-1548484352-ea579e5233a8?w=1920&q=80' },
]

export const WPS = [
  'https://images.unsplash.com/photo-1579546929518-9e396f3cc809?w=1920&q=80',
  'https://images.unsplash.com/photo-1506905925346-21bda4d32df4?w=1920&q=80',
  'https://images.unsplash.com/photo-1419242902214-272b3f66ee7a?w=1920&q=80',
  'https://images.unsplash.com/photo-1470071459604-3b5ec3a7fe05?w=1920&q=80',
  'https://images.unsplash.com/photo-1519681393784-d120267933ba?w=1920&q=80',
  'https://images.unsplash.com/photo-1548484352-ea579e5233a8?w=1920&q=80',
]

export const FONT_OPTIONS = [
  { v: 'system', l: '系统默认' },
  { v: "'PingFang SC','Microsoft YaHei',sans-serif", l: '苹方 / 微软雅黑' },
  { v: "'Noto Sans SC',sans-serif", l: 'Noto Sans' },
  { v: "Georgia,'Times New Roman',serif", l: 'Georgia（衬线）' },
  { v: "'Courier New',Courier,monospace", l: 'Courier（等宽）' },
  { v: "Impact,Haettenschweiler,'Arial Narrow Bold',sans-serif", l: 'Impact（粗体）' },
]

export const curThemeId = ref('purple-pink')
export const curTheme = computed(() => THEMES.find(x => x.id === curThemeId.value) || THEMES[0])

export function applyThemeCss(theme) {
  const r = document.documentElement.style
  r.setProperty('--h1', theme.h1)
  r.setProperty('--h2', theme.h2)
  r.setProperty('--grad', `linear-gradient(135deg,${theme.h1},${theme.h2})`)
}

export function useTheme() {
  watch(curTheme, applyThemeCss, { immediate: true })
  return { curThemeId, curTheme, THEMES, WPS, FONT_OPTIONS }
}

export function resolveFont(f) {
  return (!f || f === 'system')
    ? "-apple-system,BlinkMacSystemFont,'Segoe UI','PingFang SC','Microsoft YaHei',sans-serif"
    : f
}

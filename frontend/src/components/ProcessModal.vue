<template>
  <div class="m-overlay" v-if="visible" @click.self="close">
    <div class="m-box">
      <div class="m-head">
        <div style="display:flex;align-items:center;gap:10px">
          <div class="head-ico">⚙️</div>
          <span class="m-title">进程管理</span>
        </div>
        <div class="m-head-right">
          <input class="m-search" v-model="search" placeholder="🔍 搜索进程..." />
          <button class="m-close" @click="close">✕</button>
        </div>
      </div>

      <!-- 工具栏 -->
      <div class="m-toolbar">
        <div style="display:flex;gap:6px;flex-wrap:wrap">
          <button class="tbtn" :class="{active:sortBy==='cpu'}" @click="setSortBy('cpu')">CPU</button>
          <button class="tbtn" :class="{active:sortBy==='mem'}" @click="setSortBy('mem')">MEM</button>
          <button class="tbtn dir-btn" @click="toggleDir">
            <span :style="sortDir==='asc'?'transform:scaleY(-1);display:inline-block':''">▼</span>
            {{ sortDir==='desc'?'降序':'升序' }}
          </button>
        </div>
        <div style="display:flex;gap:8px;align-items:center">
          <button class="tbtn" @click="load">🔄 刷新</button>
          <span class="proc-count">{{ filtered.length }} / {{ processes.length }}</span>
        </div>
      </div>

      <!-- 桌面端：表格 -->
      <div class="m-body desktop-body">
        <div style="overflow-x:auto;min-width:0">
          <table class="tbl">
            <thead>
              <tr>
                <th style="width:72px">PID</th>
                <th>进程名</th>
                <th style="width:90px">用户</th>
                <th style="width:130px">CPU%</th>
                <th style="width:70px">MEM%</th>
                <th style="width:80px">RSS</th>
                <th style="width:80px">状态</th>
                <th style="width:44px"></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="p in filtered" :key="p.pid">
                <td><span class="pid">{{ p.pid }}</span></td>
                <td><span class="pname">{{ p.name }}</span></td>
                <td><span class="puser">{{ p.username }}</span></td>
                <td>
                  <div style="display:flex;align-items:center;gap:7px">
                    <div class="mini-bar-wrap">
                      <div class="mini-bar-fill" :style="`width:${Math.min(p.cpu_percent,100)}%;background:${p.cpu_percent>50?'#f43f5e':p.cpu_percent>20?'#f59e0b':'#6366f1'}`"></div>
                    </div>
                    <span class="pct" :style="`color:${p.cpu_percent>50?'#f43f5e':p.cpu_percent>20?'#f59e0b':'#6366f1'};min-width:38px`">{{ p.cpu_percent?.toFixed(1) }}%</span>
                  </div>
                </td>
                <td><span class="pct" :style="`color:${p.mem_percent>20?'#f43f5e':p.mem_percent>10?'#f59e0b':'#10b981'}`">{{ p.mem_percent?.toFixed(1) }}%</span></td>
                <td><span class="mono pct" style="color:#6b7280">{{ fmtMem(p.mem_rss) }}</span></td>
                <td><span class="stag" :class="stTag(p.status)">{{ p.status }}</span></td>
                <td><button class="kill-btn" @click="confirmKill(p)" title="终止进程">✕</button></td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- 移动端：卡片列表 -->
      <div class="m-body mobile-body">
        <div class="proc-cards">
          <div v-for="p in filtered" :key="p.pid" class="pcard">
            <div class="pcard-head">
              <div class="pcard-left">
                <span class="pid">{{ p.pid }}</span>
                <span class="pname">{{ p.name }}</span>
              </div>
              <div class="pcard-right">
                <span class="stag" :class="stTag(p.status)">{{ p.status }}</span>
                <button class="kill-btn" @click="confirmKill(p)" title="终止进程">✕</button>
              </div>
            </div>
            <div class="pcard-user">👤 {{ p.username }}</div>
            <div class="pcard-metrics">
              <div class="pm">
                <span class="pm-lbl">CPU</span>
                <div class="mini-bar-wrap">
                  <div class="mini-bar-fill" :style="`width:${Math.min(p.cpu_percent,100)}%;background:${p.cpu_percent>50?'#f43f5e':p.cpu_percent>20?'#f59e0b':'#6366f1'}`"></div>
                </div>
                <span class="pm-val" :style="`color:${p.cpu_percent>50?'#f43f5e':p.cpu_percent>20?'#f59e0b':'#6366f1'}`">{{ p.cpu_percent?.toFixed(1) }}%</span>
              </div>
              <div class="pm">
                <span class="pm-lbl">MEM</span>
                <div class="mini-bar-wrap">
                  <div class="mini-bar-fill" :style="`width:${Math.min(p.mem_percent*3,100)}%;background:${p.mem_percent>20?'#f43f5e':p.mem_percent>10?'#f59e0b':'#10b981'}`"></div>
                </div>
                <span class="pm-val" :style="`color:${p.mem_percent>20?'#f43f5e':p.mem_percent>10?'#f59e0b':'#10b981'}`">{{ p.mem_percent?.toFixed(1) }}%</span>
              </div>
              <div class="pm-rss">RSS: {{ fmtMem(p.mem_rss) }}</div>
            </div>
          </div>
          <div v-if="!filtered.length" class="empty">
            <div style="font-size:36px">⚙️</div>
            <div style="color:#94a3b8;margin-top:10px;font-size:14px">暂无进程数据</div>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- 确认终止 -->
  <div class="m-overlay" v-if="killTarget" style="z-index:910" @click.self="killTarget=null">
    <div class="confirm-box">
      <div style="font-size:18px;font-weight:800;color:#1e1b2e;margin-bottom:10px">⚠️ 终止进程</div>
      <div style="font-size:14px;color:#4b5563;margin-bottom:22px;line-height:1.6">
        确认终止进程 <strong style="color:#f43f5e;font-size:16px">{{ killTarget.name }}</strong><br>
        <span style="color:#94a3b8;font-size:13px">PID: {{ killTarget.pid }}</span>
      </div>
      <div style="display:flex;gap:8px;justify-content:flex-end">
        <button class="tbtn" @click="killTarget=null">取消</button>
        <button class="tbtn danger" @click="doKill">终止</button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { apiCall } from '../composables/useApi.js'
const visible = ref(false)
const processes = ref([])
const search = ref('')
const sortBy = ref('cpu')
const sortDir = ref('desc')
const killTarget = ref(null)
const filtered = computed(() => processes.value.filter(p => p.name?.toLowerCase().includes(search.value.toLowerCase())))
function fmtMem(b) { if(!b) return '0'; if(b<1048576) return (b/1024).toFixed(0)+'KB'; return (b/1048576).toFixed(1)+'MB' }
function stTag(s) { if(s==='R'||s==='running') return 'green'; if(s==='Z'||s==='zombie') return 'red'; return 'gray' }
function setSortBy(s) { sortBy.value=s; load() }
function toggleDir() { sortDir.value=sortDir.value==='desc'?'asc':'desc'; load() }
async function load() { try { processes.value = await apiCall(`/api/monitor/processes?sort=${sortBy.value}&dir=${sortDir.value}&limit=100`) } catch {} }
function confirmKill(p) { killTarget.value=p }
async function doKill() {
  try { await apiCall(`/api/monitor/processes/${killTarget.value.pid}`,{method:'DELETE'}); killTarget.value=null; load() }
  catch(e) { alert('失败: '+e.message) }
}
async function open() { visible.value=true; await load() }
function close() { visible.value=false }
defineExpose({ open, close })
</script>

<style scoped>
.m-overlay { position:fixed;inset:0;z-index:900;background:rgba(15,10,40,.55);backdrop-filter:blur(10px);display:flex;align-items:center;justify-content:center;padding:16px }
.m-box { background:#fff;border-radius:22px;width:960px;max-width:96vw;max-height:90vh;display:flex;flex-direction:column;box-shadow:0 40px 100px rgba(99,102,241,.2) }
.m-head { display:flex;align-items:center;justify-content:space-between;padding:20px 28px;border-bottom:2px solid #f0f4ff }
.m-head-right { display:flex;gap:8px;align-items:center }
.head-ico { font-size:20px;width:40px;height:40px;display:flex;align-items:center;justify-content:center;background:rgba(99,102,241,.1);border-radius:12px }
.m-title { font-size:20px;font-weight:800;background:var(--grad);-webkit-background-clip:text;-webkit-text-fill-color:transparent;background-clip:text }
.m-search { padding:8px 14px;border:1.5px solid #ede8f5;border-radius:10px;font-size:14px;outline:none;background:#faf8ff;font-family:inherit;width:200px }
.m-search:focus { border-color:var(--h1) }
.m-close { border:none;background:#f0f4ff;border-radius:10px;width:36px;height:36px;cursor:pointer;font-size:15px;color:#64748b;font-weight:700 }
.m-close:hover { background:#e0e7ff }
.m-toolbar { display:flex;align-items:center;justify-content:space-between;gap:10px;padding:14px 28px;border-bottom:1px solid #f0f4ff;background:#fdfcff;flex-wrap:wrap }
.proc-count { font-size:13px;color:#94a3b8;font-weight:600 }
.tbtn { padding:7px 14px;border:1.5px solid #ede8f5;border-radius:9px;background:#fff;cursor:pointer;font-size:13px;font-weight:600;color:#6b7280;transition:all .15s;display:inline-flex;align-items:center;gap:5px }
.tbtn:hover { border-color:var(--h1);color:var(--h1);background:#faf5ff }
.tbtn.active { border-color:var(--h1);color:var(--h1);background:linear-gradient(135deg,rgba(99,102,241,.1),rgba(139,92,246,.08)) }
.tbtn.danger { background:#fee2e2;color:#dc2626;border-color:#fca5a5 }
.tbtn.danger:hover { background:#fecaca }
.dir-btn { gap:6px }
.m-body { flex:1;overflow-y:auto;overflow-x:hidden;min-height:0 }
.tbl { width:100%;border-collapse:collapse }
.tbl th { padding:12px 16px;background:#f8faff;font-size:11px;font-weight:700;color:#7c3aed;text-transform:uppercase;letter-spacing:.5px;white-space:nowrap;position:sticky;top:0;z-index:1;border-bottom:1.5px solid #ede8f5;text-align:left }
.tbl td { padding:13px 16px;border-bottom:1px solid #f5f3ff;vertical-align:middle }
.tbl tr:hover td { background:#faf8ff }
.tbl tr:last-child td { border-bottom:none }
.pid { font-family:monospace;font-size:14px;color:#6366f1;font-weight:700 }
.pname { font-size:14px;font-weight:700;color:#1e1b2e }
.puser { font-size:13px;color:#94a3b8 }
.pct { font-size:14px;font-weight:700 }
.mono { font-family:monospace }
.mini-bar-wrap { width:50px;height:5px;background:#eef2ff;border-radius:3px;overflow:hidden }
.mini-bar-fill { height:100%;border-radius:3px;transition:width .3s }
.stag { display:inline-flex;align-items:center;padding:4px 10px;border-radius:9px;font-size:12px;font-weight:700 }
.stag.green { background:#d1fae5;color:#059669 }
.stag.red   { background:#fee2e2;color:#dc2626 }
.stag.gray  { background:#f3f4f6;color:#6b7280 }
.kill-btn { border:none;background:#fee2e2;border-radius:8px;width:28px;height:28px;cursor:pointer;color:#dc2626;font-size:12px;font-weight:700;transition:background .15s }
.kill-btn:hover { background:#fecaca }
.confirm-box { background:#fff;border-radius:18px;padding:28px;width:400px;max-width:92vw;box-shadow:0 32px 80px rgba(0,0,0,.2) }
.empty { text-align:center;padding:60px 20px }

/* 移动端卡片（默认隐藏） */
.mobile-body { display:none }
.proc-cards { display:flex;flex-direction:column;gap:10px;padding:14px }
.pcard { background:#fff;border:1.5px solid rgba(99,102,241,.1);border-radius:16px;padding:14px 16px;box-shadow:0 2px 10px rgba(99,102,241,.06) }
.pcard-head { display:flex;align-items:center;justify-content:space-between;margin-bottom:6px }
.pcard-left { display:flex;align-items:center;gap:8px;min-width:0 }
.pcard-right { display:flex;align-items:center;gap:8px;flex-shrink:0 }
.pcard-user { font-size:12px;color:#94a3b8;margin-bottom:10px }
.pcard-metrics { display:flex;flex-direction:column;gap:6px;background:rgba(99,102,241,.04);border-radius:10px;padding:10px }
.pm { display:flex;align-items:center;gap:8px }
.pm-lbl { font-size:11px;color:#94a3b8;font-weight:700;width:30px }
.pm-val { font-size:13px;font-weight:700;width:42px;text-align:right }
.pm-rss { font-size:12px;color:#6b7280;font-family:monospace;text-align:right;margin-top:2px }

@media(max-width:700px) {
  .m-head { padding:10px 12px;flex-wrap:nowrap;gap:6px }
  .m-head-right { flex-wrap:nowrap;gap:5px;flex:1;min-width:0 }
  .head-ico { width:28px;height:28px;font-size:15px;border-radius:8px;flex-shrink:0 }
  .m-title { font-size:13px;white-space:nowrap }
  .m-search { width:0;flex:1;min-width:0;padding:6px 10px;font-size:12px }
  .m-close { width:28px;height:28px;font-size:12px;border-radius:8px;flex-shrink:0 }
  .m-toolbar { padding:8px 12px;gap:6px;flex-wrap:nowrap }
  .m-toolbar > div { gap:4px }
  .tbtn { padding:5px 9px;font-size:11px;border-radius:7px;gap:3px }
  .proc-count { font-size:11px }
  .desktop-body { display:none }
  .mobile-body { display:flex;flex:1;overflow-y:auto;min-height:0 }
  .mobile-body > .proc-cards { flex:1 }
  .kill-btn { width:32px;height:32px;font-size:14px;border-radius:10px }
}
</style>

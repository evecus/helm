<template>
  <div class="m-overlay" v-if="visible" @click.self="close">
    <div class="m-box">
      <div class="m-head">
        <div style="display:flex;align-items:center;gap:10px">
          <div class="head-ico">🔧</div>
          <span class="m-title">Systemd 管理</span>
        </div>
        <div class="m-head-right">
          <input class="m-search" v-model="search" placeholder="🔍 搜索服务..." />
          <select class="m-sel" v-model="filterState">
            <option value="">全部状态</option>
            <option value="active">Active</option>
            <option value="inactive">Inactive</option>
            <option value="failed">Failed</option>
          </select>
          <button class="m-close" @click="close">✕</button>
        </div>
      </div>

      <!-- 工具栏 -->
      <div class="m-toolbar">
        <div style="display:flex;gap:6px;align-items:center;flex-wrap:wrap">
          <span style="font-size:12px;color:#94a3b8;font-weight:600">排序</span>
          <button class="tbtn" :class="{active:sortBy===''}" @click="setSort('')">默认</button>
          <button class="tbtn" :class="{active:sortBy==='memory'}" @click="setSort('memory')">内存</button>
          <button class="tbtn" :class="{active:sortBy==='cpu'}" @click="setSort('cpu')">CPU</button>
          <button v-if="sortBy" class="tbtn dir-btn" @click="toggleDir">
            <span :style="sortDir==='asc'?'transform:scaleY(-1);display:inline-block':''">▼</span>
            {{ sortDir==='desc'?'降序':'升序' }}
          </button>
        </div>
        <div style="display:flex;gap:8px;align-items:center">
          <button class="tbtn" @click="load(true)">🔄 刷新</button>
          <span class="svc-count">{{ filtered.length }} 个服务</span>
        </div>
      </div>

      <!-- 桌面端：表格 -->
      <div class="m-body desktop-body">
        <div v-if="loading" class="empty">
          <div style="font-size:36px">⚙️</div>
          <div style="color:#94a3b8;margin-top:10px;font-size:14px">加载中...</div>
        </div>
        <div v-else style="overflow-x:auto;min-width:0">
          <table class="tbl">
            <thead>
              <tr>
                <th style="width:36%">服务名</th>
                <th style="width:100px">状态</th>
                <th style="width:90px">内存</th>
                <th style="width:100px">文件状态</th>
                <th style="width:180px">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="svc in filtered" :key="svc.unit" class="svc-row" @click="showDetail(svc)">
                <td>
                  <div class="svc-name">{{ svc.unit }}</div>
                  <div class="svc-desc">{{ svc.description }}</div>
                  <div v-if="svc.main_pid&&svc.main_pid!=='0'" class="svc-pid">PID {{ svc.main_pid }}</div>
                </td>
                <td>
                  <span class="stag" :class="stateTag(svc.active)">{{ svc.active }}</span>
                  <span v-if="svc.sub" class="svc-sub">({{ svc.sub }})</span>
                </td>
                <td>
                  <span v-if="svc.memory" class="mem-lbl">{{ svc.memory }}</span>
                  <span v-else style="color:#d1d5db;font-size:13px">—</span>
                </td>
                <td>
                  <span v-if="svc.unit_file_state" class="stag sm" :class="fileTag(svc.unit_file_state)">{{ svc.unit_file_state }}</span>
                  <span v-else style="color:#d1d5db;font-size:13px">—</span>
                </td>
                <td @click.stop>
                  <div class="acts">
                    <button class="abtn cyan"  v-if="svc.active!=='active'" @click="action(svc,'start')"   title="启动">▶</button>
                    <button class="abtn ghost" v-if="svc.active==='active'" @click="action(svc,'stop')"    title="停止">⏹</button>
                    <button class="abtn ghost" @click="action(svc,'restart')" title="重启">↺</button>
                    <button class="abtn green" v-if="svc.unit_file_state!=='enabled'"  @click="action(svc,'enable')"  title="开机启动">✓</button>
                    <button class="abtn red"   v-if="svc.unit_file_state==='enabled'"  @click="action(svc,'disable')" title="禁用启动">✗</button>
                    <button class="abtn ghost" @click="showLogs(svc)" title="查看日志">📋</button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
          <div v-if="filtered.length===0&&!loading" class="empty">
            <div style="font-size:36px">🔧</div>
            <div style="color:#94a3b8;margin-top:10px;font-size:14px">{{ errorMsg || '未发现服务' }}</div>
          </div>
        </div>
      </div>

      <!-- 移动端：卡片列表 -->
      <div class="m-body mobile-body">
        <div v-if="loading" class="empty">
          <div style="font-size:36px">⚙️</div>
          <div style="color:#94a3b8;margin-top:10px;font-size:14px">加载中...</div>
        </div>
        <div v-else class="svc-cards">
          <div v-for="svc in filtered" :key="svc.unit" class="scard" @click="showDetail(svc)">
            <!-- 服务名 + 状态 -->
            <div class="scard-head">
              <div class="scard-name-wrap">
                <div class="svc-name">{{ svc.unit }}</div>
                <div class="svc-desc">{{ svc.description }}</div>
              </div>
              <span class="stag" :class="stateTag(svc.active)">{{ svc.active }}</span>
            </div>
            <!-- 信息行 -->
            <div class="scard-info">
              <span v-if="svc.sub" class="scard-sub">({{ svc.sub }})</span>
              <span v-if="svc.memory" class="mem-lbl">💾 {{ svc.memory }}</span>
              <span v-if="svc.unit_file_state" class="stag sm" :class="fileTag(svc.unit_file_state)">{{ svc.unit_file_state }}</span>
            </div>
            <!-- 操作按钮 -->
            <div class="scard-acts" @click.stop>
              <button class="sabtn cyan"  v-if="svc.active!=='active'" @click="action(svc,'start')" title="启动">
                <span>▶</span><span class="sabtn-lbl">启动</span>
              </button>
              <button class="sabtn ghost" v-if="svc.active==='active'" @click="action(svc,'stop')" title="停止">
                <span>⏹</span><span class="sabtn-lbl">停止</span>
              </button>
              <button class="sabtn ghost" @click="action(svc,'restart')" title="重启">
                <span>↺</span><span class="sabtn-lbl">重启</span>
              </button>
              <button class="sabtn green" v-if="svc.unit_file_state!=='enabled'" @click="action(svc,'enable')" title="开机启动">
                <span>✓</span><span class="sabtn-lbl">自启</span>
              </button>
              <button class="sabtn red" v-if="svc.unit_file_state==='enabled'" @click="action(svc,'disable')" title="禁用启动">
                <span>✗</span><span class="sabtn-lbl">禁用</span>
              </button>
              <button class="sabtn ghost" @click="showLogs(svc)" title="查看日志">
                <span>📋</span><span class="sabtn-lbl">日志</span>
              </button>
            </div>
          </div>
          <div v-if="filtered.length===0&&!loading" class="empty">
            <div style="font-size:36px">🔧</div>
            <div style="color:#94a3b8;margin-top:10px;font-size:14px">{{ errorMsg || '未发现服务' }}</div>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- 服务详情 modal -->
  <div class="m-overlay" v-if="detailModal" style="z-index:910" @click.self="detailModal=null">
    <div class="sub-modal" style="width:620px">
      <div class="sub-head">
        <div>
          <div style="font-size:16px;font-weight:800;color:#1e1b4b">🔧 {{ detailModal.unit }}</div>
          <div style="font-size:12px;color:#6b7280;margin-top:3px">{{ detailModal.description }}</div>
        </div>
        <button class="m-close" @click="detailModal=null">✕</button>
      </div>
      <div class="detail-grid">
        <div class="detail-item"><div class="dl">运行状态</div><span class="stag" :class="stateTag(detailModal.active)">{{ detailModal.active }} ({{ detailModal.sub }})</span></div>
        <div class="detail-item"><div class="dl">文件状态</div><span class="stag" :class="fileTag(detailModal.unit_file_state)">{{ detailModal.unit_file_state||'—' }}</span></div>
        <div class="detail-item"><div class="dl">主进程 PID</div><span class="dv mono">{{ detailModal.main_pid||'—' }}</span></div>
        <div class="detail-item"><div class="dl">内存占用</div><span class="dv" style="color:#06b6d4;font-weight:700">{{ detailModal.memory||'—' }}</span></div>
        <div class="detail-item"><div class="dl">CPU 累计时间</div><span class="dv" style="color:#7c3aed;font-weight:700">{{ detailModal.cpu_time||'—' }}</span></div>
        <div class="detail-item"><div class="dl">任务数</div><span class="dv mono">{{ detailModal.tasks||'—' }}</span></div>
        <div class="detail-item" style="grid-column:1/-1"><div class="dl">启动命令</div><span class="dv mono" style="font-size:11px;word-break:break-all">{{ detailModal.exec_start||'—' }}</span></div>
        <div class="detail-item" style="grid-column:1/-1"><div class="dl">Unit 路径</div><span class="dv mono" style="font-size:11px;word-break:break-all">{{ detailModal.fragment_path||'—' }}</span></div>
        <div class="detail-item" style="grid-column:1/-1"><div class="dl">启动时间</div><span class="dv mono" style="font-size:11px">{{ detailModal.started_at||'—' }}</span></div>
      </div>
      <div style="display:flex;gap:8px;margin-top:20px;flex-wrap:wrap">
        <button class="tbtn cyan"  v-if="detailModal.active!=='active'" @click="action(detailModal,'start');detailModal=null">▶ 启动</button>
        <button class="tbtn ghost" v-if="detailModal.active==='active'" @click="action(detailModal,'stop');detailModal=null">⏹ 停止</button>
        <button class="tbtn ghost" @click="action(detailModal,'restart');detailModal=null">↺ 重启</button>
        <button class="tbtn green" v-if="detailModal.unit_file_state!=='enabled'" @click="action(detailModal,'enable');detailModal=null">✓ 开机启动</button>
        <button class="tbtn red"   v-if="detailModal.unit_file_state==='enabled'"  @click="action(detailModal,'disable');detailModal=null">✗ 禁用启动</button>
        <button class="tbtn ghost" @click="showLogs(detailModal);detailModal=null">📋 查看日志</button>
      </div>
    </div>
  </div>

  <!-- 日志 modal -->
  <div class="m-overlay" v-if="logModal" style="z-index:910" @click.self="logModal=null">
    <div class="sub-modal" style="width:780px">
      <div class="sub-head">
        <span style="font-size:15px;font-weight:800;color:#1e1b4b">📋 {{ logModal.unit }} 日志</span>
        <button class="m-close" @click="logModal=null">✕</button>
      </div>
      <pre class="log-pre">{{ logContent || '加载中...' }}</pre>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { apiCall } from '../composables/useApi.js'
const visible = ref(false)
const services = ref([])
const search = ref('')
const filterState = ref('')
const sortBy = ref('')
const sortDir = ref('desc')
const loading = ref(false)
const errorMsg = ref('')
const logModal = ref(null)
const logContent = ref('')
const detailModal = ref(null)

const filtered = computed(() => services.value.filter(s => {
  const q = search.value.toLowerCase()
  const ms = s.unit?.toLowerCase().includes(q) || s.description?.toLowerCase().includes(q)
  const mf = !filterState.value || s.active === filterState.value
  return ms && mf
}))

function stateTag(s) { return s==='active'?'green':s==='failed'?'red':'gray' }
function fileTag(s)  { return s==='enabled'?'green':s==='masked'?'red':'gray' }
function setSort(s) { sortBy.value=s; load(true) }
function toggleDir() { sortDir.value=sortDir.value==='desc'?'asc':'desc'; load(true) }

async function load(force) {
  if (!force && services.value.length) return
  loading.value=true; errorMsg.value=''
  try {
    const params = sortBy.value ? `?sort=${sortBy.value}&dir=${sortDir.value}` : ''
    services.value = await apiCall('/api/monitor/services'+params)
    if (!services.value?.length) errorMsg.value='未返回服务数据'
  } catch(e) { services.value=[]; errorMsg.value=e.message }
  finally { loading.value=false }
}

async function action(svc, act) {
  try { await apiCall(`/api/monitor/services/${svc.unit}/${act}`,{method:'POST'}); setTimeout(()=>load(true),900) }
  catch(e) { alert('操作失败: '+e.message) }
}

function showDetail(svc) { detailModal.value=svc }

async function showLogs(svc) {
  logModal.value=svc; logContent.value=''
  try { const d=await apiCall(`/api/monitor/services/${svc.unit}/logs`); logContent.value=d.logs||'' }
  catch { logContent.value='获取日志失败' }
}

async function open() { visible.value=true; await load(false) }
function close() { visible.value=false }
defineExpose({ open, close })
</script>

<style scoped>
.m-overlay { position:fixed;inset:0;z-index:900;background:rgba(15,10,40,.55);backdrop-filter:blur(10px);display:flex;align-items:center;justify-content:center;padding:16px }
.m-box { background:#fff;border-radius:22px;width:1000px;max-width:100%;min-width:0;max-height:90vh;display:flex;flex-direction:column;box-shadow:0 40px 100px rgba(99,102,241,.2) }
.m-head { display:flex;align-items:center;justify-content:space-between;padding:20px 28px;border-bottom:2px solid #f0f4ff;flex-wrap:wrap;gap:8px;min-width:0 }
.m-head-right { display:flex;gap:8px;align-items:center;flex-wrap:wrap;min-width:0 }
.head-ico { font-size:20px;width:40px;height:40px;display:flex;align-items:center;justify-content:center;background:rgba(99,102,241,.1);border-radius:12px }
.m-title { font-size:20px;font-weight:800;background:var(--grad);-webkit-background-clip:text;-webkit-text-fill-color:transparent;background-clip:text }
.m-search { padding:8px 14px;border:1.5px solid #ede8f5;border-radius:10px;font-size:13px;outline:none;background:#faf8ff;font-family:inherit;width:160px }
.m-search:focus { border-color:var(--h1) }
.m-sel { padding:8px 10px;border:1.5px solid #ede8f5;border-radius:10px;font-size:13px;background:#faf8ff;outline:none;font-family:inherit;color:#374151 }
.m-close { border:none;background:#f0f4ff;border-radius:10px;width:36px;height:36px;cursor:pointer;font-size:15px;color:#64748b;font-weight:700 }
.m-close:hover { background:#e0e7ff }
.m-toolbar { display:flex;align-items:center;justify-content:space-between;gap:10px;padding:14px 28px;border-bottom:1px solid #f0f4ff;background:#fdfcff;flex-wrap:wrap;min-width:0 }
.svc-count { font-size:13px;color:#94a3b8;font-weight:600;white-space:nowrap }
.m-body { flex:1;overflow-y:auto;overflow-x:hidden;min-height:0;min-width:0;width:100% }
.tbtn { padding:7px 14px;border:1.5px solid #ede8f5;border-radius:9px;background:#fff;cursor:pointer;font-size:13px;font-weight:600;color:#6b7280;transition:all .15s;display:inline-flex;align-items:center;gap:5px }
.tbtn:hover { border-color:var(--h1);color:var(--h1);background:#faf5ff }
.tbtn.active,.tbtn.cyan { border-color:var(--h1);color:var(--h1);background:rgba(99,102,241,.1) }
.tbtn.ghost { background:#fff;border-color:#ede8f5;color:#6b7280 }
.tbtn.ghost:hover { background:#faf5ff;border-color:var(--h1);color:var(--h1) }
.tbtn.green { background:#d1fae5;color:#059669;border-color:#a7f3d0 }
.tbtn.red   { background:#fee2e2;color:#dc2626;border-color:#fca5a5 }
.tbl { width:100%;border-collapse:collapse;table-layout:fixed }
.tbl th { padding:12px 16px;background:#f8faff;font-size:11px;font-weight:700;color:#7c3aed;text-transform:uppercase;letter-spacing:.5px;position:sticky;top:0;z-index:1;border-bottom:1.5px solid #ede8f5;text-align:left;white-space:nowrap }
.tbl td { padding:13px 16px;border-bottom:1px solid #f5f3ff;vertical-align:middle }
.svc-row { cursor:pointer }
.svc-row:hover td { background:#faf8ff }
.svc-name { font-size:14px;font-weight:700;color:#1e1b4b;min-width:0;overflow-wrap:anywhere;word-break:break-word;white-space:normal }
.svc-desc { font-size:11px;color:#94a3b8;margin-top:2px;min-width:0;overflow-wrap:anywhere;word-break:break-word;white-space:normal }
.svc-pid  { font-size:11px;color:#c4b5fd;margin-top:1px }
.svc-sub  { font-size:11px;color:#94a3b8;margin-left:4px }
.stag { display:inline-flex;align-items:center;padding:4px 10px;border-radius:9px;font-size:12px;font-weight:700 }
.stag.sm { font-size:11px;padding:3px 8px }
.stag.green { background:#d1fae5;color:#059669 }
.stag.red   { background:#fee2e2;color:#dc2626 }
.stag.gray  { background:#f3f4f6;color:#6b7280 }
.mem-lbl { font-size:13px;font-weight:700;color:#06b6d4;font-family:monospace }
.acts { display:flex;gap:4px;align-items:center;flex-wrap:nowrap }
.abtn { border:none;border-radius:8px;width:30px;height:30px;cursor:pointer;font-size:13px;font-weight:700;display:flex;align-items:center;justify-content:center;transition:all .15s }
.abtn.cyan  { background:#e0f7fa;color:#0891b2 } .abtn.cyan:hover  { background:#b2ebf2 }
.abtn.ghost { background:#f5f3ff;color:#6366f1 } .abtn.ghost:hover { background:#ede9fe }
.abtn.green { background:#d1fae5;color:#059669 } .abtn.green:hover { background:#a7f3d0 }
.abtn.red   { background:#fee2e2;color:#dc2626 } .abtn.red:hover   { background:#fecaca }
.dir-btn { gap:5px }
.empty { text-align:center;padding:60px 20px }
.sub-modal { background:#fff;border-radius:20px;padding:28px;max-width:100%;max-height:88vh;overflow-y:auto;box-shadow:0 32px 80px rgba(0,0,0,.2) }
.sub-head { display:flex;align-items:flex-start;justify-content:space-between;margin-bottom:20px;gap:12px;min-width:0 }
.detail-grid { display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:10px }
.detail-item { background:#f8faff;border:1px solid rgba(99,102,241,.1);border-radius:12px;padding:12px 16px;display:flex;flex-direction:column;gap:5px }
.dl { font-size:10px;font-weight:700;text-transform:uppercase;letter-spacing:.6px;color:#94a3b8 }
.dv { font-size:14px;color:#1e1b4b;font-weight:600 }
.mono { font-family:monospace }
.log-pre { background:#0f172a;color:#e2e8f0;border-radius:12px;padding:18px;font-size:13px;font-family:monospace;overflow:auto;max-height:60vh;white-space:pre-wrap;word-break:break-all;margin-top:14px }

/* 移动端卡片（默认隐藏） */
.mobile-body { display:none }
.svc-cards { display:flex;flex-direction:column;gap:10px;padding:14px;min-width:0 }
.scard { background:#fff;border:1.5px solid rgba(99,102,241,.1);border-radius:16px;padding:14px 16px;box-shadow:0 2px 10px rgba(99,102,241,.06);cursor:pointer;min-width:0;overflow:hidden }
.scard:active { background:#faf8ff }
.scard-head { display:flex;align-items:flex-start;justify-content:space-between;gap:8px;margin-bottom:6px;min-width:0 }
.scard-name-wrap { flex:1;min-width:0 }
.scard-info { display:flex;align-items:center;gap:8px;flex-wrap:wrap;margin-bottom:10px;margin-top:4px;min-width:0 }
.scard-sub { font-size:12px;color:#94a3b8 }
.scard-acts { display:flex;gap:6px;flex-wrap:wrap;min-width:0 }
.sabtn { display:flex;flex-direction:column;align-items:center;justify-content:center;gap:2px;border:none;border-radius:12px;padding:8px 12px;min-width:52px;cursor:pointer;font-size:16px;font-weight:700;transition:all .15s }
.sabtn-lbl { font-size:10px;font-weight:600;line-height:1 }
.sabtn.cyan  { background:#e0f7fa;color:#0891b2 } .sabtn.cyan:hover  { background:#b2ebf2 }
.sabtn.ghost { background:#f5f3ff;color:#6366f1 } .sabtn.ghost:hover { background:#ede9fe }
.sabtn.green { background:#d1fae5;color:#059669 } .sabtn.green:hover { background:#a7f3d0 }
.sabtn.red   { background:#fee2e2;color:#dc2626 } .sabtn.red:hover   { background:#fecaca }

@media(max-width:700px) {
  .m-head { padding:10px 12px;flex-wrap:nowrap;gap:6px }
  .m-head-right { flex-wrap:nowrap;gap:5px;flex:1;min-width:0 }
  .head-ico { width:28px;height:28px;font-size:15px;border-radius:8px;flex-shrink:0 }
  .m-title { font-size:13px;white-space:nowrap }
  .m-search { width:0;flex:1;min-width:0;padding:6px 10px;font-size:12px }
  .m-sel { padding:6px 6px;font-size:11px;flex-shrink:0 }
  .m-close { width:28px;height:28px;font-size:12px;border-radius:8px;flex-shrink:0 }
  .m-toolbar { padding:8px 12px;gap:6px;flex-wrap:nowrap;min-width:0 }
  .m-toolbar > div { gap:4px;min-width:0 }
  .tbtn { padding:5px 9px;font-size:11px;border-radius:7px;gap:3px }
  .svc-count { font-size:11px;min-width:0;overflow:hidden;text-overflow:ellipsis }
  .desktop-body { display:none }
  .mobile-body { display:flex;flex:1;overflow-y:auto;min-height:0 }
  .mobile-body > .svc-cards { flex:1;min-width:0 }
}
</style>

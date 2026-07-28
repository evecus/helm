<template>
  <div class="m-overlay" v-if="visible" @click.self="close">
    <div class="m-box">
      <div class="m-head">
        <div style="display:flex;align-items:center;gap:10px">
          <div class="head-ico">🐳</div>
          <span class="m-title">Docker 管理</span>
        </div>
        <div style="display:flex;gap:8px;align-items:center">
          <input class="m-search" v-model="search" placeholder="🔍 搜索容器..." />
          <button class="tbtn" @click="load">🔄</button>
          <button class="m-close" @click="close">✕</button>
        </div>
      </div>

      <div class="m-body">
        <div v-if="filtered.length" class="cg-wrap">
          <!-- running / paused 组 -->
          <div v-if="filteredRunning.length" class="cg">
            <div v-for="c in filteredRunning" :key="c.id" class="ccard">
              <div class="cc-head">
                <div class="cc-dot" :class="c.state==='running'?'run':c.state==='paused'?'pause':'stop'"></div>
                <div class="cc-name" :title="c.name">{{ c.name }}</div>
                <span class="stag" :class="stateTag(c.state)">{{ c.state }}</span>
              </div>
              <div class="cc-img">🐳 {{ c.image }}</div>
              <!-- 端口 -->
              <div class="cc-ports-wrap">
                <div class="cc-port-row" v-for="(port,pi) in parsedPorts(c.ports).slice(0,2)" :key="pi">
                  <span class="port-tag" :class="isClickablePort(port)?'clickable':''"
                    @click="openPort(port)" :title="isClickablePort(port)?`打开 ${port}`:port">{{ port }}</span>
                </div>
                <div v-if="parsedPorts(c.ports).length===0" class="cc-port-row" style="height:22px"></div>
                <div v-if="parsedPorts(c.ports).length<=1" class="cc-port-row" style="height:22px"></div>
              </div>
              <!-- CPU/MEM -->
              <div class="cc-metrics" v-if="c.state==='running'">
                <div class="cm">
                  <span class="cm-lbl">CPU</span>
                  <div class="mini-bar"><div class="mini-fill" :style="`width:${c.cpu_percent||0}%;background:#6366f1`"></div></div>
                  <span class="cm-val" style="color:#6366f1">{{ c.cpu_percent?.toFixed(1) }}%</span>
                </div>
                <div class="cm">
                  <span class="cm-lbl">MEM</span>
                  <div class="mini-bar"><div class="mini-fill" :style="`width:${c.mem_percent||0}%;background:#06b6d4`"></div></div>
                  <span class="cm-val" style="color:#06b6d4">{{ c.mem_percent?.toFixed(1) }}%</span>
                </div>
              </div>
              <div v-else class="cc-metrics-placeholder"></div>
              <!-- 操作 -->
              <div class="cc-actions">
                <button class="abtn cyan"  v-if="c.state!=='running'" @click="action(c,'start')" title="启动">▶</button>
                <button class="abtn ghost" v-if="c.state==='running'" @click="action(c,'stop')" title="停止">⏹</button>
                <button class="abtn ghost" @click="action(c,'restart')" title="重启">↺</button>
                <button class="abtn purple" @click="showInspect(c)" title="容器参数">⚙️</button>
                <button class="abtn ghost" @click="pullUpdate(c)" :disabled="updating===c.id" :title="updating===c.id?'更新中...':'一键更新镜像'">{{ updating===c.id?'⏳':'⬆️' }}</button>
                <button class="abtn ghost" @click="showLogs(c)" title="查看日志">📋</button>
              </div>
            </div>
          </div>
          <!-- stopped 组 -->
          <div v-if="filteredStopped.length" class="cg">
            <div v-for="c in filteredStopped" :key="c.id" class="ccard">
              <div class="cc-head">
                <div class="cc-dot" :class="c.state==='running'?'run':c.state==='paused'?'pause':'stop'"></div>
                <div class="cc-name" :title="c.name">{{ c.name }}</div>
                <span class="stag" :class="stateTag(c.state)">{{ c.state }}</span>
              </div>
              <div class="cc-img">🐳 {{ c.image }}</div>
              <!-- 端口 -->
              <div class="cc-ports-wrap">
                <div class="cc-port-row" v-for="(port,pi) in parsedPorts(c.ports).slice(0,2)" :key="pi">
                  <span class="port-tag" :class="isClickablePort(port)?'clickable':''"
                    @click="openPort(port)" :title="isClickablePort(port)?`打开 ${port}`:port">{{ port }}</span>
                </div>
                <div v-if="parsedPorts(c.ports).length===0" class="cc-port-row" style="height:22px"></div>
                <div v-if="parsedPorts(c.ports).length<=1" class="cc-port-row" style="height:22px"></div>
              </div>
              <!-- CPU/MEM -->
              <div class="cc-metrics" v-if="c.state==='running'">
                <div class="cm">
                  <span class="cm-lbl">CPU</span>
                  <div class="mini-bar"><div class="mini-fill" :style="`width:${c.cpu_percent||0}%;background:#6366f1`"></div></div>
                  <span class="cm-val" style="color:#6366f1">{{ c.cpu_percent?.toFixed(1) }}%</span>
                </div>
                <div class="cm">
                  <span class="cm-lbl">MEM</span>
                  <div class="mini-bar"><div class="mini-fill" :style="`width:${c.mem_percent||0}%;background:#06b6d4`"></div></div>
                  <span class="cm-val" style="color:#06b6d4">{{ c.mem_percent?.toFixed(1) }}%</span>
                </div>
              </div>
              <div v-else class="cc-metrics-placeholder"></div>
              <!-- 操作 -->
              <div class="cc-actions">
                <button class="abtn cyan"  v-if="c.state!=='running'" @click="action(c,'start')" title="启动">▶</button>
                <button class="abtn ghost" v-if="c.state==='running'" @click="action(c,'stop')" title="停止">⏹</button>
                <button class="abtn ghost" @click="action(c,'restart')" title="重启">↺</button>
                <button class="abtn purple" @click="showInspect(c)" title="容器参数">⚙️</button>
                <button class="abtn ghost" @click="pullUpdate(c)" :disabled="updating===c.id" :title="updating===c.id?'更新中...':'一键更新镜像'">{{ updating===c.id?'⏳':'⬆️' }}</button>
                <button class="abtn ghost" @click="showLogs(c)" title="查看日志">📋</button>
              </div>
            </div>
          </div>
        </div>
        <div v-else class="empty">
          <div style="font-size:48px">🐳</div>
          <div style="font-size:16px;font-weight:700;color:#1e1b4b;margin-top:14px">暂无容器</div>
          <div style="font-size:13px;color:#94a3b8;margin-top:6px">确认 Docker 正在运行</div>
        </div>
      </div>
    </div>
  </div>

  <!-- 容器参数 modal -->
  <div class="m-overlay" v-if="inspectModal" style="z-index:910" @click.self="closeInspect">
    <div class="sub-modal" style="width:780px">
      <div class="sub-head">
        <div>
          <div style="font-size:16px;font-weight:800;color:#1e1b4b">⚙️ {{ inspectModal.name }} 参数</div>
          <div v-if="inspectData?.compose_file" style="font-size:12px;color:#10b981;margin-top:3px">📦 docker-compose 容器</div>
        </div>
        <button class="m-close" @click="closeInspect">✕</button>
      </div>
      <div class="tab-bar" v-if="inspectData?.compose_file">
        <button class="tab-btn" :class="{active:inspectTab==='info'}" @click="inspectTab='info'">容器信息</button>
        <button class="tab-btn" :class="{active:inspectTab==='compose'}" @click="inspectTab='compose'">编辑 Compose</button>
      </div>
      <div v-if="inspectTab==='info'||!inspectData?.compose_file">
        <div v-if="!inspectData" style="text-align:center;padding:40px;color:#94a3b8;font-size:14px">加载中...</div>
        <div v-else class="inspect-grid">
          <div class="inspect-item"><div class="il">镜像</div><span class="iv mono">{{ inspectData.image }}</span></div>
          <div class="inspect-item"><div class="il">状态</div><span class="iv">{{ inspectData.status }}</span></div>
          <div class="inspect-item"><div class="il">创建时间</div><span class="iv mono" style="font-size:12px">{{ inspectData.created }}</span></div>
          <div class="inspect-item"><div class="il">重启策略</div><span class="iv">{{ inspectData.restart_policy||'—' }}</span></div>
          <div class="inspect-item" style="grid-column:1/-1"><div class="il">端口映射</div><span class="iv mono">{{ inspectData.ports||'—' }}</span></div>
          <div class="inspect-item" style="grid-column:1/-1"><div class="il">环境变量</div><pre class="mini-pre">{{ (inspectData.env||[]).join('\n')||'—' }}</pre></div>
          <div class="inspect-item" style="grid-column:1/-1"><div class="il">挂载卷</div><pre class="mini-pre">{{ (inspectData.mounts||[]).join('\n')||'—' }}</pre></div>
          <div class="inspect-item" style="grid-column:1/-1"><div class="il">网络</div><span class="iv">{{ (inspectData.networks||[]).join(', ')||'—' }}</span></div>
          <div class="inspect-item" style="grid-column:1/-1"><div class="il">启动命令</div><pre class="mini-pre">{{ (inspectData.cmd||[]).join(' ')||'—' }}</pre></div>
        </div>
        <div style="margin-top:16px;border-top:1px solid #f0f4ff;padding-top:14px">
          <button class="tbtn ghost" @click="pullUpdate(inspectModal)" :disabled="updating===inspectModal.id">
            {{ updating===inspectModal.id?'⏳ 更新中...':'⬆️ 一键更新镜像' }}
          </button>
        </div>
      </div>
      <div v-if="inspectTab==='compose'&&inspectData?.compose_file">
        <div style="font-size:11px;color:#94a3b8;font-family:monospace;margin-bottom:8px">{{ inspectData.compose_file }}</div>
        <div v-if="composeErr" class="alert-err">{{ composeErr }}</div>
        <div v-if="composeOk" class="alert-ok">✓ {{ composeOk }}</div>
        <textarea class="editor-box" v-model="composeContent" spellcheck="false" placeholder="加载中..."></textarea>
        <div style="display:flex;gap:8px;margin-top:12px;align-items:center;flex-wrap:wrap">
          <button class="tbtn danger" @click="applyCompose" :disabled="composeSaving">{{ composeSaving?'处理中...':'💥 销毁并重建容器' }}</button>
          <span style="font-size:11px;color:#94a3b8">⚠️ 将销毁当前容器并用新 compose 文件重建</span>
        </div>
      </div>
    </div>
  </div>

  <!-- 日志 modal -->
  <div class="m-overlay" v-if="logModal" style="z-index:910" @click.self="logModal=null">
    <div class="sub-modal" style="width:740px">
      <div class="sub-head">
        <span style="font-size:15px;font-weight:800;color:#1e1b4b">📋 {{ logModal.name }} 日志</span>
        <button class="m-close" @click="logModal=null">✕</button>
      </div>
      <pre class="log-pre">{{ logContent||'加载中...' }}</pre>
    </div>
  </div>

  <!-- 更新结果 modal -->
  <div class="m-overlay" v-if="updateLog" style="z-index:910" @click.self="updateLog=null">
    <div class="sub-modal" style="width:660px">
      <div class="sub-head">
        <span style="font-size:15px;font-weight:800;color:#1e1b4b">⬆️ 镜像更新结果</span>
        <button class="m-close" @click="updateLog=null">✕</button>
      </div>
      <pre class="log-pre">{{ updateLog }}</pre>
      <button class="tbtn ghost" style="margin-top:12px" @click="updateLog=null;load()">关闭并刷新</button>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { apiCall } from '../composables/useApi.js'

const visible = ref(false)
const containers = ref([])
const search = ref('')
const logModal = ref(null)
const logContent = ref('')
const inspectModal = ref(null)
const inspectData = ref(null)
const inspectTab = ref('info')
const composeContent = ref('')
const composeErr = ref('')
const composeOk = ref('')
const composeSaving = ref(false)
const updating = ref(null)
const updateLog = ref(null)

const filtered = computed(() => {
  const stateOrder = s => s==='running'?0:s==='paused'?1:s==='exited'?2:3
  return containers.value
    .filter(c =>
      c.name?.toLowerCase().includes(search.value.toLowerCase()) ||
      c.image?.toLowerCase().includes(search.value.toLowerCase())
    )
    .sort((a,b) => stateOrder(a.state)-stateOrder(b.state))
})
const filteredRunning = computed(() => filtered.value.filter(c => c.state==='running' || c.state==='paused'))
const filteredStopped = computed(() => filtered.value.filter(c => c.state!=='running' && c.state!=='paused'))

function stateTag(s) { return s==='running'?'green':s==='paused'?'yellow':'gray' }

async function load() {
  try { containers.value = await apiCall('/api/monitor/containers') }
  catch { if (!containers.value.length) containers.value=[] }
}

async function action(c, act) {
  try { await apiCall(`/api/monitor/containers/${c.id}/${act}`,{method:'POST'}); await load() }
  catch(e) { alert(e.message) }
}

async function showLogs(c) {
  logModal.value=c; logContent.value=''
  try { const d=await apiCall(`/api/monitor/containers/${c.id}/logs`); logContent.value=d.logs||'' }
  catch { logContent.value='获取日志失败' }
}

function parsedPorts(portsStr) {
  if (!portsStr) return []
  let s = portsStr
  s = s.replace(/(\/(tcp|udp|sctp))(\[)/g,'$1,$3')
  s = s.replace(/(\/(tcp|udp|sctp))(\d)/g,'$1,$3')
  const tokens = s.split(/[,\\s]+/).map(t=>t.trim()).filter(Boolean)
  const seen=new Set(), result=[]
  for (const tok of tokens) { if(tok&&!seen.has(tok)){seen.add(tok);result.push(tok)} }
  return result
}
function isClickablePort(p) { return /^[0-9.]+:\d+->/.test(p) || /^\[::]:/.test(p) }
function openPort(p) {
  const m=p.match(/^[0-9.]+:(\d+)->/);
  if(m) window.open(`http://${window.location.hostname}:${m[1]}`,'_blank')
}

async function showInspect(c) {
  inspectModal.value=c; inspectData.value=null; inspectTab.value='info'
  composeContent.value=''; composeErr.value=''; composeOk.value=''
  try {
    const d=await apiCall(`/api/monitor/containers/${c.id}/inspect`)
    inspectData.value=d
    if (d.compose_file) {
      try {
        const r2=await apiCall(`/api/monitor/compose/file?path=${encodeURIComponent(d.compose_file)}`)
        composeContent.value=r2.content||''
      } catch { composeContent.value='' }
    }
  } catch(e) {
    inspectData.value={ image:'获取失败', status:e.message, env:[], mounts:[], networks:[], cmd:[] }
  }
}
function closeInspect() { inspectModal.value=null; inspectData.value=null }

async function applyCompose() {
  if (!confirm('确定要销毁当前容器并用修改后的 compose 文件重建吗？')) return
  composeSaving.value=true; composeErr.value=''; composeOk.value=''
  try {
    const d=await apiCall('/api/monitor/compose/apply',{
      method:'POST',
      body: JSON.stringify({ path:inspectData.value.compose_file, content:composeContent.value, container_id:inspectModal.value.id })
    })
    composeOk.value=d.message||'重建成功'; setTimeout(()=>load(),2000)
  } catch(e) { composeErr.value=e.message }
  finally { composeSaving.value=false }
}

async function pullUpdate(c) {
  if (!c) return
  if (!confirm(`确定要拉取最新镜像并重建容器 ${c.name} 吗？`)) return
  updating.value=c.id
  try {
    const d=await apiCall(`/api/monitor/containers/${c.id}/update`,{method:'POST'})
    updateLog.value=d.log||'更新完成'; load()
  } catch(e) { updateLog.value='错误: '+e.message }
  finally { updating.value=null }
}

async function open() { visible.value=true; await load() }
function close() { visible.value=false }
defineExpose({ open, close })
</script>

<style scoped>
.m-overlay { position:fixed;inset:0;z-index:900;background:rgba(15,10,40,.55);backdrop-filter:blur(10px);display:flex;align-items:center;justify-content:center;padding:16px }
.m-box { background:#fff;border-radius:22px;width:1000px;max-width:96vw;max-height:90vh;display:flex;flex-direction:column;box-shadow:0 40px 100px rgba(99,102,241,.2) }
.m-head { display:flex;align-items:center;justify-content:space-between;padding:20px 28px;border-bottom:2px solid #f0f4ff }
.head-ico { font-size:22px;width:44px;height:44px;display:flex;align-items:center;justify-content:center;background:rgba(6,182,212,.1);border-radius:12px }
.m-title { font-size:20px;font-weight:800;background:var(--grad);-webkit-background-clip:text;-webkit-text-fill-color:transparent;background-clip:text }
.m-search { padding:8px 14px;border:1.5px solid #ede8f5;border-radius:10px;font-size:14px;outline:none;background:#faf8ff;font-family:inherit;width:180px }
.m-search:focus { border-color:var(--h1) }
.m-close { border:none;background:#f0f4ff;border-radius:10px;width:36px;height:36px;cursor:pointer;font-size:15px;color:#64748b;font-weight:700 }
.m-close:hover { background:#e0e7ff }
.tbtn { padding:7px 14px;border:1.5px solid #ede8f5;border-radius:9px;background:#fff;cursor:pointer;font-size:13px;font-weight:600;color:#6b7280;transition:all .15s;display:inline-flex;align-items:center;gap:5px }
.tbtn:hover { border-color:var(--h1);color:var(--h1);background:#faf5ff }
.tbtn:disabled { opacity:.5;cursor:not-allowed }
.tbtn.ghost { border-color:#ede8f5 }
.tbtn.danger { background:#fee2e2;color:#dc2626;border-color:#fca5a5 }
.tbtn.danger:hover { background:#fecaca }
.m-body { padding:22px 28px;overflow-y:auto;flex:1 }

/* 容器卡片 */
.cg-wrap { display:flex;flex-direction:column;gap:8px }
.cg-wrap { display:flex;flex-direction:column;gap:8px }
.cg { display:grid;grid-template-columns:repeat(auto-fill,minmax(290px,1fr));gap:16px }
.ccard { background:#fff;border:1.5px solid rgba(99,102,241,.1);border-radius:18px;padding:18px;box-shadow:0 2px 14px rgba(99,102,241,.07);transition:transform .2s,box-shadow .2s }
.ccard:hover { transform:translateY(-3px);box-shadow:0 8px 28px rgba(99,102,241,.14) }
.cc-head { display:flex;align-items:center;gap:8px;margin-bottom:9px }
.cc-dot { width:11px;height:11px;border-radius:50%;flex-shrink:0 }
.cc-dot.run   { background:#10b981;box-shadow:0 0 8px rgba(16,185,129,.5);animation:pulse 2s infinite }
.cc-dot.pause { background:#f59e0b }
.cc-dot.stop  { background:#9ca3af }
.cc-name { font-size:15px;font-weight:700;color:#1e1b4b;flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap }
.cc-img { font-size:12px;color:#94a3b8;margin-bottom:9px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap }
.stag { display:inline-flex;align-items:center;padding:3px 10px;border-radius:9px;font-size:11px;font-weight:700;flex-shrink:0 }
.stag.green  { background:#d1fae5;color:#059669 }
.stag.yellow { background:#fef3c7;color:#d97706 }
.stag.gray   { background:#f3f4f6;color:#6b7280 }
.cc-ports-wrap { margin-bottom:9px;min-height:48px }
.cc-port-row { height:22px;display:flex;align-items:center;margin-bottom:2px }
.port-tag { display:inline-flex;align-items:center;padding:2px 8px;border-radius:6px;font-size:11px;font-weight:600;background:rgba(99,102,241,.08);color:#6366f1;font-family:monospace }
.port-tag.clickable { cursor:pointer;transition:all .15s }
.port-tag.clickable:hover { background:rgba(6,182,212,.15);color:#0e7490;transform:scale(1.04) }
.cc-metrics { display:flex;flex-direction:column;gap:7px;margin-bottom:14px;background:rgba(99,102,241,.04);border-radius:10px;padding:11px }
.cc-metrics-placeholder { height:54px;margin-bottom:14px }
.cm { display:flex;align-items:center;gap:7px }
.cm-lbl { font-size:10px;color:#94a3b8;font-weight:700;width:28px }
.mini-bar { flex:1;height:5px;background:#eef2ff;border-radius:3px;overflow:hidden }
.mini-fill { height:100%;border-radius:3px;transition:width .5s }
.cm-val { font-size:12px;font-weight:700;width:36px;text-align:right }
/* 操作按钮 - 桌面和移动通用大尺寸 */
.cc-actions { display:flex;gap:7px;align-items:center;flex-wrap:nowrap }
.abtn { border:none;border-radius:10px;width:36px;height:36px;cursor:pointer;font-size:15px;font-weight:700;display:flex;align-items:center;justify-content:center;transition:all .15s }
.abtn:disabled { opacity:.5;cursor:not-allowed }
.abtn.cyan   { background:#e0f7fa;color:#0891b2 } .abtn.cyan:hover   { background:#b2ebf2 }
.abtn.ghost  { background:#f5f3ff;color:#6366f1 } .abtn.ghost:hover  { background:#ede9fe }
.abtn.purple { background:rgba(124,58,237,.1);color:#7c3aed } .abtn.purple:hover { background:rgba(124,58,237,.18) }
.empty { text-align:center;padding:60px 20px }
.sub-modal { background:#fff;border-radius:20px;padding:28px;max-width:96vw;max-height:90vh;overflow-y:auto;box-shadow:0 32px 80px rgba(0,0,0,.2) }
.sub-head { display:flex;align-items:flex-start;justify-content:space-between;margin-bottom:18px;gap:12px }
.tab-bar { display:flex;gap:4px;margin-bottom:16px;border-bottom:1.5px solid #f0f4ff;padding-bottom:8px }
.tab-btn { padding:6px 16px;border-radius:9px 9px 0 0;font-size:13px;font-weight:600;cursor:pointer;border:none;background:transparent;color:#94a3b8;transition:all .2s }
.tab-btn.active { background:rgba(99,102,241,.1);color:#6366f1 }
.inspect-grid { display:grid;grid-template-columns:1fr 1fr;gap:10px }
.inspect-item { background:#f8faff;border:1px solid rgba(99,102,241,.1);border-radius:12px;padding:12px 16px;display:flex;flex-direction:column;gap:5px }
.il { font-size:10px;font-weight:700;text-transform:uppercase;letter-spacing:.6px;color:#94a3b8 }
.iv { font-size:14px;color:#1e1b4b;font-weight:600;word-break:break-all }
.mono { font-family:monospace }
.mini-pre { margin:4px 0 0;font-family:monospace;font-size:11px;color:#374151;background:#f0f4ff;border-radius:8px;padding:8px;max-height:90px;overflow-y:auto;white-space:pre-wrap;word-break:break-all }
.editor-box { width:100%;height:42vh;background:#0f172a;color:#e2e8f0;border:1px solid rgba(99,102,241,.2);border-radius:12px;padding:16px;font-family:monospace;font-size:12px;line-height:1.6;resize:vertical;outline:none;box-sizing:border-box }
.editor-box:focus { border-color:#7c3aed }
.alert-err { background:rgba(244,63,94,.1);border:1px solid rgba(244,63,94,.3);border-radius:8px;padding:10px;margin-bottom:10px;font-size:12px;color:#f43f5e }
.alert-ok  { background:rgba(16,185,129,.1);border:1px solid rgba(16,185,129,.3);border-radius:8px;padding:10px;margin-bottom:10px;font-size:12px;color:#10b981 }
.log-pre { background:#0f172a;color:#e2e8f0;border-radius:12px;padding:18px;font-size:13px;font-family:monospace;overflow:auto;max-height:60vh;white-space:pre-wrap;word-break:break-all;margin-top:14px }
@keyframes pulse { 0%,100%{opacity:1}50%{opacity:.5} }

/* 移动端：全屏底部弹出，卡片全宽单列 */
@media(max-width:700px) {
  .m-head { padding:16px 18px }
  .m-search { display:none }
  .m-body { padding:14px 14px 16px }
  .cg { grid-template-columns:1fr;gap:12px }
  .abtn { width:42px;height:42px;font-size:17px;border-radius:12px }
  .cc-actions { gap:8px }
}
</style>

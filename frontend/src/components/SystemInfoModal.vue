<template>
  <div class="m-overlay" v-if="visible" @click.self="close">
    <div class="m-box">
      <div class="m-head">
        <div style="display:flex;align-items:center;gap:10px">
          <div class="head-ico">🖥️</div>
          <span class="m-title">本机信息</span>
        </div>
        <button class="m-close" @click="close">✕</button>
      </div>
      <div class="m-body">
        <!-- 系统信息横幅 -->
        <div class="sys-banner">
          <div class="si" v-for="item in sysItems" :key="item.l">
            <div class="si-l">{{ item.l }}</div>
            <div class="si-v" :title="item.v">{{ item.v || '—' }}</div>
          </div>
        </div>
        <!-- 四指标卡片 -->
        <div class="cards-row">
          <div class="mcard" style="--acc:#6366f1">
            <div class="mc-top"><span class="mc-ico" style="background:rgba(99,102,241,.12)">⚡</span><span class="mc-t">CPU</span></div>
            <div class="mc-val" style="color:#6366f1">{{ snap.cpu?.usage_percent?.toFixed(1)||'0.0' }}%</div>
            <div class="bar"><div class="bf" :style="`width:${snap.cpu?.usage_percent||0}%;background:#6366f1`"></div></div>
            <div class="mc-sub">{{ snap.cpu?.cpu_threads||0 }} 线程 · 负载 {{ snap.cpu?.load_avg_1?.toFixed(2)||'—' }}</div>
          </div>
          <div class="mcard" style="--acc:#06b6d4">
            <div class="mc-top"><span class="mc-ico" style="background:rgba(6,182,212,.12)">💾</span><span class="mc-t">内存</span></div>
            <div class="mc-val" style="color:#06b6d4">{{ snap.memory?.used_percent?.toFixed(1)||'0.0' }}%</div>
            <div class="bar"><div class="bf" :style="`width:${snap.memory?.used_percent||0}%;background:#06b6d4`"></div></div>
            <div class="mc-sub">{{ fmt(snap.memory?.used) }} / {{ fmt(snap.memory?.total) }}</div>
          </div>
          <div class="mcard" style="--acc:#10b981">
            <div class="mc-top"><span class="mc-ico" style="background:rgba(16,185,129,.12)">💽</span><span class="mc-t">磁盘</span></div>
            <div class="mc-val" style="color:#10b981">{{ maxDisk.toFixed(1) }}%</div>
            <div class="bar"><div class="bf" :style="`width:${maxDisk}%;background:#10b981`"></div></div>
            <div class="mc-sub">{{ snap.disk?.partitions?.length||0 }} 个分区</div>
          </div>
          <div class="mcard" style="--acc:#f59e0b">
            <div class="mc-top"><span class="mc-ico" style="background:rgba(245,158,11,.12)">🌐</span><span class="mc-t">网络</span></div>
            <div class="mc-val" style="color:#f59e0b;font-size:20px">{{ fmtSpeed(totalUp) }} ↑</div>
            <div class="mc-sub">↓ {{ fmtSpeed(totalDown) }} · {{ snap.network?.connections }} 连接</div>
          </div>
        </div>
        <!-- 中间三栏 -->
        <div class="mid-row">
          <!-- 内存详情 -->
          <div class="card">
            <div class="card-h">内存详情</div>
            <div style="display:flex;flex-direction:column;gap:14px">
              <div>
                <div class="mb-hd"><span>RAM</span><span style="color:#6366f1;font-weight:700;font-size:15px">{{ snap.memory?.used_percent?.toFixed(1) }}%</span></div>
                <div class="mb-sub">{{ fmt(snap.memory?.used) }} / {{ fmt(snap.memory?.total) }}</div>
                <div class="bar" style="height:8px;margin-top:6px"><div class="bf" :style="`width:${snap.memory?.used_percent||0}%;background:#6366f1`"></div></div>
              </div>
              <div v-if="snap.memory?.swap_total>0">
                <div class="mb-hd"><span>Swap</span><span style="color:#06b6d4;font-weight:700;font-size:15px">{{ snap.memory?.swap_percent?.toFixed(1) }}%</span></div>
                <div class="mb-sub">{{ fmt(snap.memory?.swap_used) }} / {{ fmt(snap.memory?.swap_total) }}</div>
                <div class="bar" style="height:8px;margin-top:6px"><div class="bf" :style="`width:${snap.memory?.swap_percent||0}%;background:#06b6d4`"></div></div>
              </div>
              <div class="mem-stats">
                <div class="mem-stat"><div class="ms-lbl">缓存</div><div class="ms-val" style="color:#6366f1">{{ fmt(snap.memory?.cached) }}</div></div>
                <div class="mem-stat"><div class="ms-lbl">缓冲</div><div class="ms-val" style="color:#6366f1">{{ fmt(snap.memory?.buffers) }}</div></div>
                <div class="mem-stat"><div class="ms-lbl">可用</div><div class="ms-val" style="color:#10b981">{{ fmt(snap.memory?.available) }}</div></div>
              </div>
            </div>
          </div>
          <!-- CPU核心 -->
          <div class="card">
            <div class="card-h">CPU · {{ snap.cpu?.frequency_mhz?.toFixed(0)||'—' }} MHz</div>
            <div style="display:flex;flex-direction:column;gap:7px;overflow-y:auto;max-height:200px">
              <div v-for="(pct,i) in snap.cpu?.per_core_usage" :key="i" class="core-row">
                <span class="core-lbl">C{{ i }}</span>
                <div class="bar" style="flex:1;height:7px"><div class="bf" :style="`width:${pct||0}%;background:${pct>80?'#f43f5e':pct>50?'#f59e0b':'#6366f1'}`"></div></div>
                <span class="core-val" :style="pct>80?'color:#f43f5e':pct>50?'color:#f59e0b':'color:#6366f1'">{{ pct?.toFixed(0) }}%</span>
              </div>
              <div v-if="!snap.cpu?.per_core_usage?.length" style="color:#94a3b8;text-align:center;padding:20px 0;font-size:13px">暂无核心数据</div>
            </div>
          </div>
          <!-- 负载+温度 -->
          <div class="card">
            <div class="card-h">系统负载</div>
            <div class="loads">
              <div v-for="(v,k) in loads" :key="k" class="load-item">
                <div class="load-val">{{ v }}</div>
                <div class="load-lbl">{{ k }}</div>
              </div>
            </div>
            <template v-if="snap.temps?.length">
              <div class="card-h" style="font-size:13px;margin:14px 0 10px">温度</div>
              <div style="display:flex;flex-direction:column;gap:8px">
                <div v-for="tp in snap.temps" :key="tp.sensor" class="temp-row">
                  <span class="temp-lbl">{{ tp.sensor }}</span>
                  <div class="bar" style="flex:1;height:7px"><div class="bf" :style="`width:${Math.min(tp.temperature/1.2,100)}%;background:${tp.temperature>80?'#f43f5e':tp.temperature>60?'#f59e0b':'#10b981'}`"></div></div>
                  <span class="temp-val" :style="tp.temperature>80?'color:#f43f5e':tp.temperature>60?'color:#f59e0b':'color:#10b981'">{{ tp.temperature?.toFixed(1) }}°C</span>
                </div>
              </div>
            </template>
            <div v-else style="margin-top:14px;text-align:center;padding:12px 0">
              <div style="font-size:28px">🌡️</div>
              <div style="font-size:12px;color:#94a3b8;margin-top:6px">暂无温度数据</div>
            </div>
          </div>
        </div>
        <!-- 磁盘分区 -->
        <div class="section-hd">磁盘分区</div>
        <div class="disk-grid">
          <div class="dk" v-for="p in snap.disk?.partitions" :key="p.mountpoint">
            <div class="dk-head">
              <span class="dk-mp">{{ p.mountpoint }}</span>
              <span class="tag" :class="p.used_percent>85?'tag-red':p.used_percent>70?'tag-yellow':'tag-green'">{{ p.used_percent?.toFixed(1) }}%</span>
            </div>
            <div class="dk-dev">{{ p.device }} · {{ p.fstype }}</div>
            <div class="bar" style="height:7px"><div class="bf" :style="`width:${p.used_percent||0}%;background:${p.used_percent>85?'#f43f5e':p.used_percent>70?'#f59e0b':'#10b981'}`"></div></div>
            <div class="dk-size"><span>已用 {{ fmt(p.used) }}</span><span>{{ fmt(p.free) }} 可用 / {{ fmt(p.total) }}</span></div>
          </div>
        </div>
        <!-- 网络接口 -->
        <div class="section-hd">网络接口</div>
        <div class="net-grid">
          <div class="ncard" v-for="iface in snap.network?.interfaces" :key="iface.name">
            <div class="nc-head">
              <div class="nc-dot" :class="(iface.speed_up||iface.speed_down)?'active':'idle'"></div>
              <span class="nc-name">{{ iface.name }}</span>
            </div>
            <div v-for="addr in (iface.addrs||[]).slice(0,2)" :key="addr" class="nc-addr">{{ addr }}</div>
            <div class="nc-speeds">
              <div class="nc-sp" style="color:#10b981">↑ {{ fmtSpeed(iface.speed_up) }}</div>
              <div class="nc-sp" style="color:#6366f1">↓ {{ fmtSpeed(iface.speed_down) }}</div>
              <div class="nc-sp" style="color:#f59e0b">{{ fmt((iface.bytes_sent||0)+(iface.bytes_recv||0)) }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
<script setup>
import { ref, computed, onUnmounted } from 'vue'
import { apiCall } from '../composables/useApi.js'
const visible = ref(false)
const snap = ref({ cpu:{}, memory:{}, disk:{partitions:[]}, network:{interfaces:[]}, temps:[], system:{} })
let timer = null
const sysItems = computed(() => {
  const s = snap.value.system || {}
  return [
    { l:'主机名',  v: s.hostname },
    { l:'系统',    v: `${s.platform||''} ${s.platform_version||''}`.trim() },
    { l:'内核',    v: s.kernel_version },
    { l:'架构',    v: s.arch },
    { l:'CPU型号', v: s.cpu_model },
    { l:'CPU核心', v: s.cpu_threads ? `${s.cpu_cores||1}核 ${s.cpu_threads}线程` : '—' },
    { l:'运行时间',v: s.uptime_str },
    { l:'本地IP',  v: s.local_ipv4 },
  ]
})
const maxDisk = computed(() => Math.max(0,...(snap.value.disk?.partitions?.map(p=>p.used_percent)||[0])))
const totalUp   = computed(() => (snap.value.network?.interfaces||[]).reduce((s,i)=>s+(i.speed_up||0),0))
const totalDown = computed(() => (snap.value.network?.interfaces||[]).reduce((s,i)=>s+(i.speed_down||0),0))
const loads = computed(() => ({ '1m':snap.value.cpu?.load_avg_1?.toFixed(2)||'—', '5m':snap.value.cpu?.load_avg_5?.toFixed(2)||'—', '15m':snap.value.cpu?.load_avg_15?.toFixed(2)||'—' }))
function fmt(b) { if(!b) return '0 B'; const u=['B','KB','MB','GB','TB'],i=Math.min(Math.floor(Math.log(b)/Math.log(1024)),4); return (b/Math.pow(1024,i)).toFixed(1)+' '+u[i] }
function fmtSpeed(b) { if(!b) return '0 B/s'; if(b<1024) return b+' B/s'; if(b<1048576) return (b/1024).toFixed(1)+' KB/s'; return (b/1048576).toFixed(2)+' MB/s' }
async function load() {
  try {
    const d = await apiCall('/api/monitor/metrics')
    snap.value = { cpu:d.cpu||{}, memory:d.memory||{}, disk:d.disk||{partitions:[]}, network:d.network||{interfaces:[]}, temps:d.temperatures||[], system:d.system||{} }
  } catch {}
}
async function open() { visible.value=true; await load(); timer=setInterval(load,3000) }
function close() { visible.value=false; clearInterval(timer); timer=null }
onUnmounted(() => clearInterval(timer))
defineExpose({ open, close })
</script>
<style scoped>
.m-overlay { position:fixed;inset:0;z-index:900;background:rgba(15,10,40,.55);backdrop-filter:blur(10px);display:flex;align-items:center;justify-content:center;padding:16px }
.m-box { background:#fff;border-radius:22px;width:1020px;max-width:96vw;max-height:90vh;display:flex;flex-direction:column;box-shadow:0 40px 100px rgba(99,102,241,.2) }
.m-head { display:flex;align-items:center;justify-content:space-between;padding:20px 28px;border-bottom:2px solid #f0f4ff }
.head-ico { font-size:20px;width:40px;height:40px;display:flex;align-items:center;justify-content:center;background:rgba(99,102,241,.1);border-radius:12px }
.m-title { font-size:20px;font-weight:800;background:var(--grad);-webkit-background-clip:text;-webkit-text-fill-color:transparent;background-clip:text }
.m-close { border:none;background:#f0f4ff;border-radius:10px;width:36px;height:36px;cursor:pointer;font-size:15px;color:#64748b;font-weight:700 }
.m-close:hover { background:#e0e7ff }
.m-body { padding:24px 28px;overflow-y:auto;display:flex;flex-direction:column;gap:20px;flex:1 }
.sys-banner { display:grid;grid-template-columns:repeat(4,1fr);gap:10px }
.si { background:linear-gradient(135deg,rgba(99,102,241,.05),rgba(139,92,246,.04));border:1px solid rgba(99,102,241,.1);border-radius:12px;padding:12px 16px }
.si-l { font-size:10px;color:#94a3b8;font-weight:700;text-transform:uppercase;letter-spacing:.6px;margin-bottom:5px }
.si-v { font-size:14px;color:#1e1b2e;font-weight:600;overflow:hidden;text-overflow:ellipsis;white-space:nowrap }
.cards-row { display:grid;grid-template-columns:repeat(4,1fr);gap:12px }
.mcard { background:#f8faff;border-radius:16px;padding:18px;border-top:3px solid var(--acc);box-shadow:0 2px 12px rgba(99,102,241,.06) }
.mc-top { display:flex;align-items:center;gap:8px;margin-bottom:10px }
.mc-ico { width:34px;height:34px;border-radius:10px;display:flex;align-items:center;justify-content:center;font-size:16px }
.mc-t { font-size:13px;font-weight:700;color:#4b5563 }
.mc-val { font-size:28px;font-weight:800;margin-bottom:8px;font-family:monospace;letter-spacing:-1px }
.mc-sub { font-size:12px;color:#94a3b8;margin-top:6px }
.mid-row { display:grid;grid-template-columns:1fr 1fr 1fr;gap:14px }
.card { background:#fff;border:1px solid rgba(99,102,241,.1);border-radius:16px;padding:20px;box-shadow:0 2px 12px rgba(99,102,241,.05) }
.card-h { font-size:14px;font-weight:700;color:#1e1b4b;margin-bottom:14px;padding-left:10px;border-left:3px solid #6366f1 }
.mb-hd { display:flex;justify-content:space-between;align-items:center;font-size:13px;font-weight:600;color:#374151;margin-bottom:3px }
.mb-sub { font-size:12px;color:#94a3b8 }
.mem-stats { display:grid;grid-template-columns:repeat(3,1fr);gap:6px;margin-top:12px;padding-top:12px;border-top:1px solid #f0f4ff }
.mem-stat { text-align:center;background:#f8faff;border-radius:10px;padding:10px 4px }
.ms-lbl { font-size:10px;color:#94a3b8;font-weight:700;text-transform:uppercase;margin-bottom:4px }
.ms-val { font-size:14px;font-weight:700 }
.core-row { display:grid;grid-template-columns:24px 1fr 36px;align-items:center;gap:8px }
.core-lbl { font-size:11px;color:#94a3b8;font-weight:600 }
.core-val { font-size:12px;font-weight:700;text-align:right }
.loads { display:flex;justify-content:space-around }
.load-item { text-align:center;padding:12px 0 }
.load-val { font-size:30px;font-weight:800;font-family:monospace;background:linear-gradient(135deg,#6366f1,#06b6d4);-webkit-background-clip:text;-webkit-text-fill-color:transparent;background-clip:text }
.load-lbl { font-size:12px;color:#94a3b8;margin-top:4px }
.temp-row { display:flex;align-items:center;gap:8px }
.temp-lbl { font-size:12px;color:#4b5563;width:64px;flex-shrink:0;overflow:hidden;text-overflow:ellipsis }
.temp-val { font-size:13px;font-weight:700;width:46px;text-align:right;font-family:monospace }
.section-hd { font-size:15px;font-weight:800;color:#6366f1;display:flex;align-items:center;gap:8px }
.section-hd::after { content:'';flex:1;height:1.5px;background:linear-gradient(90deg,rgba(99,102,241,.25),transparent) }
.disk-grid { display:grid;grid-template-columns:repeat(3,1fr);gap:10px }
.dk { background:rgba(99,102,241,.04);border:1px solid rgba(99,102,241,.1);border-radius:12px;padding:14px }
.dk-head { display:flex;align-items:center;justify-content:space-between;margin-bottom:3px }
.dk-mp { font-size:15px;font-weight:700;color:#1e1b4b }
.dk-dev { font-size:11px;color:#94a3b8;margin-bottom:8px }
.dk-size { display:flex;justify-content:space-between;font-size:11px;color:#6b7280;margin-top:6px }
.net-grid { display:grid;grid-template-columns:repeat(auto-fill,minmax(200px,1fr));gap:10px }
.ncard { background:rgba(99,102,241,.04);border:1px solid rgba(99,102,241,.1);border-radius:12px;padding:14px }
.nc-head { display:flex;align-items:center;gap:7px;margin-bottom:6px }
.nc-dot { width:8px;height:8px;border-radius:50%;flex-shrink:0 }
.nc-dot.active { background:#10b981;box-shadow:0 0 7px rgba(16,185,129,.5);animation:pulse 2s infinite }
.nc-dot.idle { background:#94a3b8 }
.nc-name { font-size:14px;font-weight:700;color:#1e1b4b }
.nc-addr { font-size:10px;color:#6366f1;font-family:monospace;margin-bottom:2px;word-break:break-all }
.nc-speeds { display:grid;grid-template-columns:repeat(3,1fr);gap:4px;margin-top:8px;font-size:11px;font-weight:600;font-family:monospace }
.nc-sp { text-align:center;background:rgba(255,255,255,.7);border-radius:6px;padding:4px 2px }
.bar { height:5px;background:#eef2ff;border-radius:3px;overflow:hidden }
.bf  { height:100%;border-radius:3px;transition:width .5s ease }
.tag { display:inline-flex;align-items:center;padding:3px 8px;border-radius:9px;font-size:11px;font-weight:700 }
.tag-green  { background:#d1fae5;color:#059669 }
.tag-yellow { background:#fef3c7;color:#d97706 }
.tag-red    { background:#fee2e2;color:#dc2626 }
@keyframes pulse { 0%,100%{opacity:1}50%{opacity:.5} }
@media(max-width:780px) {
  .sys-banner,.cards-row { grid-template-columns:repeat(2,1fr) }
  .mid-row,.disk-grid { grid-template-columns:1fr }
}
</style>

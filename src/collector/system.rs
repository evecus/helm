//! 系统指标采集，对应 Go 版 internal/collector/system.go。
//! gopsutil 的等价能力全部由 `sysinfo` crate 提供；sysinfo 未覆盖的少量字段
//! （发行版名、网卡 IP、连接数、磁盘/内存补充字段）直接读 /proc、/sys 补齐。

use libc::{sockaddr_in, sockaddr_in6};
use serde::Serialize;
use std::collections::HashMap;
use std::ffi::CStr;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use sysinfo::{Disks, Networks, System};

// ── 数据结构（json 字段与 Go 版完全一致） ────────────────────────

#[derive(Clone, Debug, Default, Serialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub os: String,
    pub platform: String,
    pub platform_version: String,
    pub kernel_version: String,
    pub arch: String,
    pub uptime: u64,
    pub uptime_str: String,
    pub boot_time: u64,
    pub cpu_model: String,
    pub cpu_cores: i32,
    pub cpu_threads: i32,
    pub public_ip: String,
    pub local_ipv4: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CPUStats {
    pub usage_percent: f64,
    pub per_core_usage: Vec<f64>,
    pub load_avg_1: f64,
    pub load_avg_5: f64,
    pub load_avg_15: f64,
    pub frequency_mhz: f64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct MemoryStats {
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub available: u64,
    pub used_percent: f64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub swap_free: u64,
    pub swap_percent: f64,
    pub cached: u64,
    pub buffers: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct DiskPartition {
    pub device: String,
    pub mountpoint: String,
    pub fstype: String,
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub used_percent: f64,
    pub read_bytes: u64,
    pub write_bytes: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct DiskStats {
    pub partitions: Vec<DiskPartition>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct NetworkInterface {
    pub name: String,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub packets_sent: u64,
    pub packets_recv: u64,
    pub speed_up: u64,
    pub speed_down: u64,
    pub addrs: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct NetworkStats {
    pub interfaces: Vec<NetworkInterface>,
    pub total_sent: u64,
    pub total_recv: u64,
    pub connections: i32,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Temperature {
    pub sensor: String,
    pub temperature: f64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct MetricsSnapshot {
    pub timestamp: i64,
    pub system: SystemInfo,
    pub cpu: CPUStats,
    pub memory: MemoryStats,
    pub disk: DiskStats,
    pub network: NetworkStats,
    pub temperatures: Vec<Temperature>,
}

// ── 小工具 ────────────────────────────────────────────────────────

fn read_file(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

fn format_uptime(secs: u64) -> String {
    let d = secs / 86400;
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    if d > 0 {
        format!("{}d {}h {}m", d, h, m)
    } else {
        format!("{}h {}m", h, m)
    }
}

/// Go 版 host.Info() 的 Platform / PlatformVersion 来自发行版信息。
/// sysinfo 不单独暴露发行版名，这里读 /etc/os-release 补齐。
fn os_release() -> (String, String) {
    let text = read_file("/etc/os-release");
    let text = if text.is_empty() {
        read_file("/usr/lib/os-release")
    } else {
        text
    };
    let mut name = String::new();
    let mut version = String::new();
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("NAME=") {
            name = v.trim_matches('"').to_string();
        } else if let Some(v) = line.strip_prefix("VERSION_ID=") {
            version = v.trim_matches('"').to_string();
        }
    }
    (name, version)
}

/// sysinfo 0.38 不暴露 Cached / Buffers，从 /proc/meminfo 补齐这两个字段。
fn meminfo_cached_buffers() -> (u64, u64) {
    let mut cached = 0u64;
    let mut buffers = 0u64;
    for line in read_file("/proc/meminfo").lines() {
        if let Some(v) = line.strip_prefix("Cached:") {
            cached = v
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0)
                * 1024;
        } else if let Some(v) = line.strip_prefix("Buffers:") {
            buffers = v
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0)
                * 1024;
        }
    }
    (cached, buffers)
}

/// 枚举网络接口及其 CIDR 地址，保持系统顺序（等价 net.Interfaces()）。
/// sysinfo 的 Networks 不含 IP 地址，这里用 libc getifaddrs 补齐。
fn interfaces_with_addrs() -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    unsafe {
        let mut ifap: *mut libc::ifaddrs = std::ptr::null_mut();
        if libc::getifaddrs(&mut ifap) != 0 {
            return out;
        }
        let mut cur = ifap;
        while !cur.is_null() {
            let ifa = &*cur;
            let name = if ifa.ifa_name.is_null() {
                String::new()
            } else {
                CStr::from_ptr(ifa.ifa_name).to_string_lossy().into_owned()
            };
            let addr_ptr = ifa.ifa_addr;
            let mask_ptr = ifa.ifa_netmask;
            if !addr_ptr.is_null() {
                let family = (*addr_ptr).sa_family as i32;
                let mut cidr = String::new();
                if family == libc::AF_INET {
                    let sin = &*(addr_ptr as *const sockaddr_in);
                    // s_addr 内存字节序即网络序，直接转成 Ipv4Addr
                    let ip = std::net::Ipv4Addr::from(sin.sin_addr.s_addr.to_ne_bytes());
                    let prefix = if mask_ptr.is_null() {
                        32
                    } else {
                        let m = &*(mask_ptr as *const sockaddr_in);
                        m.sin_addr.s_addr.count_ones()
                    };
                    cidr = format!("{}/{}", ip, prefix);
                } else if family == libc::AF_INET6 {
                    let sin6 = &*(addr_ptr as *const sockaddr_in6);
                    let ip = std::net::Ipv6Addr::from(sin6.sin6_addr.s6_addr);
                    let prefix = if mask_ptr.is_null() {
                        128
                    } else {
                        let m = &*(mask_ptr as *const sockaddr_in6);
                        m.sin6_addr.s6_addr.iter().map(|b| b.count_ones()).sum::<u32>()
                    };
                    cidr = format!("{}/{}", ip, prefix);
                }
                if !cidr.is_empty() {
                    if let Some(entry) = out.iter_mut().find(|e| e.0 == name) {
                        entry.1.push(cidr);
                    } else {
                        out.push((name, vec![cidr]));
                    }
                }
            } else if !out.iter().any(|e| e.0 == name) {
                out.push((name, Vec::new()));
            }
            cur = ifa.ifa_next;
        }
        libc::freeifaddrs(ifap);
    }
    out
}

/// 对应 gopsutil 的 exec.LookPath("docker")。
fn docker_installed() -> bool {
    let path = std::env::var("PATH").unwrap_or_default();
    for dir in path.split(':') {
        let p = std::path::Path::new(dir).join("docker");
        if let Ok(md) = std::fs::metadata(&p) {
            use std::os::unix::fs::PermissionsExt;
            if md.is_file() && md.permissions().mode() & 0o111 != 0 {
                return true;
            }
        }
    }
    false
}

/// Go 版 isRealInterface：只保留物理以太网卡 / 已装 docker 时的网桥。
fn is_real_interface(name: &str) -> bool {
    let base = format!("/sys/class/net/{}", name);
    let ty = read_file(&format!("{}/type", base));
    if ty.trim() != "1" {
        return false;
    }
    let dest = match std::fs::read_link(&base) {
        Ok(p) => p.to_string_lossy().into_owned(),
        Err(_) => return false,
    };
    if !dest.contains("/virtual/") {
        return true;
    }
    if std::path::Path::new(&format!("{}/peer_ifindex", base)).exists() {
        return false;
    }
    if std::path::Path::new(&format!("{}/bridge", base)).exists() {
        return docker_installed();
    }
    false
}

/// gopsutil psnet.Connections("all") 的数量，sysinfo 不提供，统计 /proc/net 连接表行数。
fn connection_count() -> i32 {
    let mut n = 0i32;
    for f in ["/proc/net/tcp", "/proc/net/tcp6", "/proc/net/udp", "/proc/net/udp6"] {
        n += read_file(f).lines().count().saturating_sub(1) as i32;
    }
    n
}

// ── 采集入口 ──────────────────────────────────────────────────────

fn build_system_info(sys: &System) -> SystemInfo {
    let (platform, platform_version) = os_release();
    let uptime = System::uptime();
    let cpu_arch = System::cpu_arch();
    let arch = match cpu_arch.as_str() {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "arm" => "arm",
        other => other,
    };
    // 物理核心数不会变化，缓存避免每次采集都重新解析 /proc/cpuinfo
    static PHYS_CORES: OnceLock<usize> = OnceLock::new();
    let cores = *PHYS_CORES.get_or_init(|| System::physical_core_count().unwrap_or(0));

    let mut si = SystemInfo {
        hostname: System::host_name().unwrap_or_default(),
        os: "linux".into(),
        platform,
        platform_version,
        kernel_version: System::kernel_version().unwrap_or_default(),
        arch: arch.to_string(),
        uptime,
        uptime_str: format_uptime(uptime),
        boot_time: System::boot_time(),
        cpu_model: sys
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_default(),
        cpu_cores: cores as i32,
        cpu_threads: sys.cpus().len() as i32,
        ..Default::default()
    };

    // Local IPv4：只取第一个私网 IPv4 地址（带 CIDR）
    for (_name, addrs) in interfaces_with_addrs() {
        for a in addrs {
            if a.contains(':') {
                continue; // 跳过 IPv6
            }
            let ip = a.split('/').next().unwrap_or("");
            if ip.starts_with("127.") || ip.starts_with("169.254.") {
                continue;
            }
            if si.local_ipv4.is_empty() {
                si.local_ipv4 = a;
            }
        }
        if !si.local_ipv4.is_empty() {
            break;
        }
    }
    si
}

fn build_cpu_stats(sys: &System) -> CPUStats {
    let load = System::load_average();
    CPUStats {
        usage_percent: sys.global_cpu_usage() as f64,
        per_core_usage: sys.cpus().iter().map(|c| c.cpu_usage() as f64).collect(),
        load_avg_1: load.one as f64,
        load_avg_5: load.five as f64,
        load_avg_15: load.fifteen as f64,
        frequency_mhz: sys
            .cpus()
            .first()
            .map(|c| c.frequency() as f64)
            .unwrap_or(0.0),
    }
}

fn build_memory_stats(sys: &System) -> MemoryStats {
    let (cached, buffers) = meminfo_cached_buffers();

    let total = sys.total_memory();
    let used = sys.used_memory();
    let swap_total = sys.total_swap();
    let swap_used = sys.used_swap();
    MemoryStats {
        total,
        used,
        free: sys.free_memory(),
        available: sys.available_memory(),
        used_percent: if total > 0 {
            used as f64 / total as f64 * 100.0
        } else {
            0.0
        },
        swap_total,
        swap_used,
        swap_free: sys.free_swap(),
        swap_percent: if swap_total > 0 {
            swap_used as f64 / swap_total as f64 * 100.0
        } else {
            0.0
        },
        cached,
        buffers,
    }
}

// 保留 Go 版的独立采集函数（当前未注册路由），供单独调用。
#[allow(dead_code)]
pub fn get_system_info() -> SystemInfo {
    let mut sys = System::new();
    sys.refresh_cpu_all();
    build_system_info(&sys)
}

#[allow(dead_code)]
pub fn get_cpu_stats() -> CPUStats {
    // cpu.Percent(300ms, false) + cpu.Percent(0, true)：两次采样间隔 300ms
    let mut sys = System::new();
    sys.refresh_cpu_all();
    std::thread::sleep(Duration::from_millis(300));
    sys.refresh_cpu_all();
    build_cpu_stats(&sys)
}

#[allow(dead_code)]
pub fn get_memory_stats() -> MemoryStats {
    let mut sys = System::new();
    sys.refresh_memory();
    build_memory_stats(&sys)
}

pub fn get_disk_stats() -> DiskStats {
    let disks = Disks::new_with_refreshed_list();
    let mut partitions = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for d in disks.list() {
        let mountpoint = d.mount_point().to_string_lossy().into_owned();
        if seen.contains(&mountpoint) || !mountpoint.starts_with('/') {
            continue;
        }
        let total = d.total_space();
        if total == 0 {
            continue;
        }
        seen.insert(mountpoint.clone());
        let free = d.available_space();
        let used = total.saturating_sub(free);
        let usage = d.usage();
        partitions.push(DiskPartition {
            device: d.name().to_string_lossy().into_owned(),
            mountpoint,
            fstype: d.file_system().to_string_lossy().into_owned(),
            total,
            used,
            free,
            used_percent: used as f64 / total as f64 * 100.0,
            read_bytes: usage.total_read_bytes,
            write_bytes: usage.total_written_bytes,
        });
    }
    DiskStats { partitions }
}

// ── 网络 ──────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct NetIo {
    bytes_recv: u64,
    packets_recv: u64,
    bytes_sent: u64,
    packets_sent: u64,
}

struct NetPrev {
    stats: HashMap<String, NetIo>,
    at: Instant,
}

static NET_PREV: OnceLock<Mutex<NetPrev>> = OnceLock::new();

fn net_prev() -> &'static Mutex<NetPrev> {
    NET_PREV.get_or_init(|| {
        Mutex::new(NetPrev {
            stats: HashMap::new(),
            at: Instant::now(),
        })
    })
}

pub fn get_network_stats() -> NetworkStats {
    let nets = Networks::new_with_refreshed_list();
    let iface_addrs = interfaces_with_addrs();
    let now = Instant::now();

    let mut prev = net_prev().lock().unwrap_or_else(|p| p.into_inner());
    let elapsed = now.duration_since(prev.at).as_secs_f64();
    let elapsed = if elapsed < 0.1 { 1.0 } else { elapsed };
    let old = prev.stats.clone();

    let mut interfaces = Vec::new();
    let mut total_sent = 0u64;
    let mut total_recv = 0u64;
    let mut current: HashMap<String, NetIo> = HashMap::new();

    for (name, data) in nets.list() {
        let io = NetIo {
            bytes_recv: data.total_received(),
            packets_recv: data.total_packets_received(),
            bytes_sent: data.total_transmitted(),
            packets_sent: data.total_packets_transmitted(),
        };
        current.insert(name.clone(), io.clone());

        if !is_real_interface(name) {
            continue;
        }
        let addrs = iface_addrs
            .iter()
            .find(|e| &e.0 == name)
            .map(|e| e.1.clone())
            .unwrap_or_default();
        let mut ni = NetworkInterface {
            name: name.clone(),
            addrs,
            bytes_sent: io.bytes_sent,
            bytes_recv: io.bytes_recv,
            packets_sent: io.packets_sent,
            packets_recv: io.packets_recv,
            ..Default::default()
        };
        total_sent += io.bytes_sent;
        total_recv += io.bytes_recv;
        if let Some(p) = old.get(name) {
            // Go 里 uint64 减法会回绕，这里保持一致
            let up = (io.bytes_sent.wrapping_sub(p.bytes_sent) as f64) / elapsed;
            let dn = (io.bytes_recv.wrapping_sub(p.bytes_recv) as f64) / elapsed;
            if up > 0.0 {
                ni.speed_up = up as u64;
            }
            if dn > 0.0 {
                ni.speed_down = dn as u64;
            }
        }
        interfaces.push(ni);
    }

    prev.stats = current;
    prev.at = now;
    drop(prev);

    NetworkStats {
        interfaces,
        total_sent,
        total_recv,
        connections: connection_count(),
    }
}

/// 对应 Go 版 GetTemperatures()：读 /sys/class/thermal/thermal_zone0..9。
/// ARM 板子（RK3566 等）的温度只暴露在 thermal_zone，sysinfo 的 Components
/// 读的是 hwmon，在这类设备上恒为空，因此这里按 Go 版原样实现。
/// root 参数化仅为单元测试可注入模拟目录，生产固定传 /sys/class/thermal。
fn temperatures_from(root: &str) -> Vec<Temperature> {
    let mut temps = Vec::new();
    for i in 0..10 {
        let base = format!("{}/thermal_zone{}", root, i);
        let data = match std::fs::read_to_string(format!("{}/temp", base)) {
            Ok(s) => s,
            Err(_) => break, // 与 Go 版一致：读不到就停止枚举
        };
        let millideg: f64 = data.trim().parse().unwrap_or(0.0);
        if millideg == 0.0 {
            continue;
        }
        let sensor = std::fs::read_to_string(format!("{}/type", base))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let sensor = if sensor.is_empty() {
            format!("zone{}", i)
        } else {
            sensor
        };
        temps.push(Temperature {
            sensor,
            temperature: millideg / 1000.0,
        });
    }
    temps
}

pub fn get_temperatures() -> Vec<Temperature> {
    temperatures_from("/sys/class/thermal")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thermal_zone_parsing_matches_go() {
        let root = std::env::temp_dir().join(format!("helm-thermal-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);

        let mk = |i: u32, temp: &str, ty: Option<&str>| {
            let z = root.join(format!("thermal_zone{}", i));
            std::fs::create_dir_all(&z).unwrap();
            std::fs::write(z.join("temp"), temp).unwrap();
            if let Some(t) = ty {
                std::fs::write(z.join("type"), t).unwrap();
            }
        };
        mk(0, "45123\n", Some("cpu-thermal\n")); // 正常：毫度 → 度，去空白
        mk(1, "38000", None); // 无 type → 回退 zone1
        mk(2, "0", None); // 0 → 跳过
        // zone3 不存在 → break，后续不再枚举

        let t = temperatures_from(root.to_str().unwrap());
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].sensor, "cpu-thermal");
        assert!((t[0].temperature - 45.123).abs() < 1e-6);
        assert_eq!(t[1].sensor, "zone1");
        assert!((t[1].temperature - 38.0).abs() < 1e-6);

        let _ = std::fs::remove_dir_all(&root);
    }
}

/// Go 版额外提供但未注册路由的辅助函数，保留等价实现。
#[allow(dead_code)]
pub fn get_crontabs() -> Vec<String> {
    let mut lines = Vec::new();
    let push_file = |p: &str, prefix: &str, lines: &mut Vec<String>| {
        for l in read_file(p).lines() {
            let l = l.trim();
            if !l.is_empty() && !l.starts_with('#') {
                lines.push(format!("{}{}", prefix, l));
            }
        }
    };
    push_file("/etc/crontab", "", &mut lines);
    if let Ok(entries) = std::fs::read_dir("/etc/cron.d") {
        for e in entries.flatten() {
            let p = e.path().to_string_lossy().into_owned();
            let name = e.file_name().to_string_lossy().into_owned();
            push_file(&p, &format!("[{}] ", name), &mut lines);
        }
    }
    if let Ok((out, _)) = crate::cmd::combined("crontab", &["-l"], Duration::from_secs(3)) {
        for l in String::from_utf8_lossy(&out).lines() {
            let l = l.trim();
            if !l.is_empty() && !l.starts_with('#') {
                lines.push(format!("[user] {}", l));
            }
        }
    }
    lines
}

pub fn collect_all() -> MetricsSnapshot {
    // 整个快照复用同一个 System：只做一次 /proc/meminfo 与一次 300ms CPU 采样窗口，
    // 避免各子函数各自 System::new() + refresh_cpu_all() 带来的重复 /proc/cpuinfo 解析。
    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu_all();
    std::thread::sleep(Duration::from_millis(300));
    sys.refresh_cpu_all();

    MetricsSnapshot {
        timestamp: crate::auth::now_unix(),
        system: build_system_info(&sys),
        cpu: build_cpu_stats(&sys),
        memory: build_memory_stats(&sys),
        disk: get_disk_stats(),
        network: get_network_stats(),
        temperatures: get_temperatures(),
    }
}

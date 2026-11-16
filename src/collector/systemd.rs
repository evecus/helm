//! 服务采集与操作。
//!
//! - systemd 分支：逐字对应 Go 版 internal/collector/systemd.go。
//! - OpenRC 分支：新增，支持 Alpine Linux 的 /etc/init.d 服务（Go 版没有）。
//!   复用同一个 SystemdService 结构，前端无需改动。
//!
//! 运行时自动检测 init 系统：/run/systemd/system 存在 → systemd；
//! 否则若 /etc/init.d 存在且带有 OpenRC 特征 → OpenRC；其余保持 systemd
//! （与 Go 版行为一致，采集失败时 handler 返回空数组）。
//! 可用环境变量 HELM_INIT_SYSTEM=systemd|openrc 强制指定。

use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use crate::cmd;

#[derive(Clone, Debug, Default, Serialize)]
pub struct SystemdService {
    pub unit: String,
    pub load: String,
    pub active: String,
    pub sub: String,
    pub description: String,
    // 详细信息（systemd 通过 systemctl show 获取；OpenRC 通过 /proc 获取）
    pub pid: String,
    pub memory_bytes: u64,
    pub memory: String,
    pub cpu_ns: u64,
    pub cpu_time: String,
    pub unit_file_state: String,
    pub main_pid: String,
    pub exec_start: String,
    pub fragment_path: String,
    pub started_at: String,
    pub tasks: String,
}

fn lossy(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn parse_kv(s: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    for line in s.lines() {
        if let Some(idx) = line.find('=') {
            if idx > 0 {
                m.insert(line[..idx].to_string(), line[idx + 1..].to_string());
            }
        }
    }
    m
}

/// ExecStart 只取 path= 之后、遇到空格或分号之前的部分。
fn exec_start_path(es: &str) -> String {
    if let Some(idx) = es.find("path=") {
        let rest = &es[idx + 5..];
        if let Some(end) = rest.find(|c| c == ' ' || c == ';') {
            return rest[..end].to_string();
        }
        return rest.to_string();
    }
    String::new()
}

fn format_bytes(b: u64) -> String {
    if b >= 1 << 30 {
        format!("{:.1} GB", b as f64 / (1u64 << 30) as f64)
    } else if b >= 1 << 20 {
        format!("{:.1} MB", b as f64 / (1u64 << 20) as f64)
    } else if b >= 1 << 10 {
        format!("{:.1} KB", b as f64 / (1u64 << 10) as f64)
    } else {
        format!("{} B", b)
    }
}

fn format_nsec(ns: u64) -> String {
    let secs = ns as f64 / 1_000_000_000.0;
    if secs >= 3600.0 {
        format!("{:.1}h", secs / 3600.0)
    } else if secs >= 60.0 {
        format!("{:.1}m", secs / 60.0)
    } else {
        format!("{:.1}s", secs)
    }
}

// ══════════════════════════════════════════════════════════════════
// init 系统检测
// ══════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InitSystem {
    Systemd,
    OpenRc,
}

fn which(cmd: &str) -> bool {
    let path = std::env::var("PATH").unwrap_or_default();
    path.split(':').any(|d| Path::new(d).join(cmd).is_file())
}

fn detect_init() -> InitSystem {
    // 显式覆盖：便于容器/特殊环境，也便于本地测试 OpenRC 分支。
    if let Ok(v) = std::env::var("HELM_INIT_SYSTEM") {
        match v.to_ascii_lowercase().as_str() {
            "openrc" | "init.d" | "alpine" => return InitSystem::OpenRc,
            "systemd" => return InitSystem::Systemd,
            _ => {}
        }
    }

    // systemd 明确在跑（PID 1 是 systemd 时会创建该目录）
    if Path::new("/run/systemd/system").is_dir() {
        return InitSystem::Systemd;
    }

    // OpenRC 特征：/etc/init.d 目录 + 任一 OpenRC 组件
    if Path::new(OPENRC_INIT_DIR).is_dir()
        && (which("rc-service")
            || which("rc-update")
            || Path::new("/sbin/openrc-run").exists()
            || Path::new("/libexec/rc/bin/openrc-run").exists()
            || Path::new("/run/openrc").is_dir())
    {
        return InitSystem::OpenRc;
    }

    // 其余保持 Go 版行为（例如普通 Docker 容器：systemctl 不存在 → 采集失败 → 空列表）
    InitSystem::Systemd
}

fn init_system() -> InitSystem {
    static INIT: OnceLock<InitSystem> = OnceLock::new();
    *INIT.get_or_init(detect_init)
}

// ══════════════════════════════════════════════════════════════════
// systemd 分支（与 Go 版一致）
// ══════════════════════════════════════════════════════════════════

fn enrich_service(svc: &mut SystemdService) {
    let props = [
        "MainPID",
        "MemoryCurrent",
        "CPUUsageNSec",
        "UnitFileState",
        "ExecStart",
        "FragmentPath",
        "TasksCurrent",
        "ActiveEnterTimestamp",
    ];
    let mut args: Vec<String> = vec!["show".into(), svc.unit.clone(), "--no-pager".into()];
    for p in props {
        args.push(format!("--property={}", p));
    }
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let out = match cmd::output("systemctl", &refs, Duration::from_secs(3)) {
        Ok(v) => v,
        Err(_) => return,
    };
    let kv = parse_kv(&lossy(&out));
    let get = |k: &str| kv.get(k).cloned().unwrap_or_default();

    svc.main_pid = get("MainPID");
    svc.unit_file_state = get("UnitFileState");
    svc.fragment_path = get("FragmentPath");
    svc.tasks = get("TasksCurrent");
    svc.exec_start = exec_start_path(&get("ExecStart"));

    // 内存
    let mem = get("MemoryCurrent");
    if !mem.is_empty() && mem != "[not set]" && mem != "18446744073709551615" {
        if let Ok(v) = mem.parse::<u64>() {
            svc.memory_bytes = v;
            svc.memory = format_bytes(v);
        }
    }
    // CPU 时间
    let cpu = get("CPUUsageNSec");
    if !cpu.is_empty() && cpu != "[not set]" {
        if let Ok(v) = cpu.parse::<u64>() {
            svc.cpu_ns = v;
            svc.cpu_time = format_nsec(v);
        }
    }
    // 启动时间
    let ts = get("ActiveEnterTimestamp");
    if !ts.is_empty() && ts != "0" {
        svc.started_at = ts;
    }
}

fn get_unit_file_state(svc: &mut SystemdService) {
    let out = match cmd::output(
        "systemctl",
        &[
            "show",
            &svc.unit,
            "--no-pager",
            "--property=UnitFileState",
            "--property=FragmentPath",
            "--property=ExecStart",
        ],
        Duration::from_secs(2),
    ) {
        Ok(v) => v,
        Err(_) => return,
    };
    let kv = parse_kv(&lossy(&out));
    svc.unit_file_state = kv.get("UnitFileState").cloned().unwrap_or_default();
    svc.fragment_path = kv.get("FragmentPath").cloned().unwrap_or_default();
    svc.exec_start = exec_start_path(&kv.get("ExecStart").cloned().unwrap_or_default());
}

fn systemd_services() -> Result<Vec<SystemdService>, String> {
    let out = cmd::output(
        "systemctl",
        &[
            "list-units",
            "--type=service",
            "--all",
            "--no-pager",
            "--plain",
            "--no-legend",
        ],
        Duration::from_secs(10),
    )?;

    let mut services = Vec::new();
    for line in lossy(&out).lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 4 {
            continue;
        }
        let mut svc = SystemdService {
            unit: fields[0].to_string(),
            load: fields[1].to_string(),
            active: fields[2].to_string(),
            sub: fields[3].to_string(),
            description: if fields.len() > 4 {
                fields[4..].join(" ")
            } else {
                String::new()
            },
            ..Default::default()
        };
        // 只对 active 服务获取详细信息（避免超时）
        if svc.active == "active" {
            enrich_service(&mut svc);
        } else {
            get_unit_file_state(&mut svc);
        }
        services.push(svc);
    }
    Ok(services)
}

// ══════════════════════════════════════════════════════════════════
// OpenRC / Alpine init.d 分支（新增）
//
// OpenRC 没有 systemctl，状态全部落在 RC_SVCDIR(/run/openrc) 下的
// 状态目录里：started / starting / stopping / inactive / wasinactive /
// failed / scheduled / hotplugged（见 openrc src/librc/librc.h dirnames）。
// 服务脚本在 /etc/init.d/，开机自启由 /etc/runlevels/<level>/<name>
// 的符号链接决定。这里全部用文件系统读取，不依赖外部命令，
// 对 musl 静态二进制最友好。
// ══════════════════════════════════════════════════════════════════

const OPENRC_INIT_DIR: &str = "/etc/init.d";
const OPENRC_RUNLEVEL_DIR: &str = "/etc/runlevels";

/// OpenRC 状态目录根；新版在 /run/openrc，老版可能在 /var/run/openrc。
fn openrc_svcdir() -> &'static str {
    if Path::new("/run/openrc").is_dir() {
        "/run/openrc"
    } else if Path::new("/var/run/openrc").is_dir() {
        "/var/run/openrc"
    } else {
        // 目录还不存在时按默认路径判断（全部服务视为 stopped）
        "/run/openrc"
    }
}

/// 返回 (active, sub)，取值与前端 SystemdModal 的 stateTag / filterState 对齐。
fn openrc_state(svcdir: &str, name: &str) -> (&'static str, &'static str) {
    let has = |sub: &str| Path::new(&format!("{}/{}/{}", svcdir, sub, name)).exists();
    if has("failed") {
        ("failed", "crashed")
    } else if has("started") {
        ("active", "running")
    } else if has("starting") {
        ("activating", "starting")
    } else if has("stopping") {
        ("deactivating", "stopping")
    } else if has("inactive") || has("wasinactive") {
        ("inactive", "inactive")
    } else if has("scheduled") {
        ("active", "scheduled")
    } else {
        ("inactive", "stopped")
    }
}

/// /etc/runlevels/<任意 runlevel>/<name> 存在即视为 enabled。
fn openrc_enabled(name: &str) -> bool {
    let Ok(rd) = std::fs::read_dir(OPENRC_RUNLEVEL_DIR) else {
        return false;
    };
    for e in rd.flatten() {
        if Path::new(&e.path().join(name)).exists() {
            return true;
        }
    }
    false
}

fn valid_service_name(n: &str) -> bool {
    if n.is_empty() || n.starts_with('.') || n.ends_with('~') || n.starts_with("README") {
        return false;
    }
    const SKIP_EXT: &[&str] = &[
        "sh", "conf", "bak", "old", "new", "orig", "sample", "example", "swp",
        "dpkg-old", "dpkg-new", "dpkg-dist",
    ];
    if let Some((_, ext)) = n.rsplit_once('.') {
        return !SKIP_EXT.contains(&ext);
    }
    true
}

#[derive(Default, Debug)]
struct ScriptMeta {
    description: String,
    command: String,
    command_args: String,
    pidfile: String,
}

/// 展开 init.d 脚本里最常见的变量（主要是 RC_SVCNAME）。
fn expand_shell_vars(s: &str, name: &str) -> String {
    let mut out = s.to_string();
    for var in ["RC_SVCNAME", "SVCNAME", "name"] {
        out = out.replace(&format!("${{{}}}", var), name);
        out = out.replace(&format!("${}", var), name);
    }
    out
}

/// 解析 `key=value` 右侧：去掉引号、行尾注释，再展开变量。
fn parse_shell_value(raw: &str, name: &str) -> String {
    let s = raw.trim();
    let s = if s.starts_with('"') {
        s[1..].split('"').next().unwrap_or("")
    } else if s.starts_with('\'') {
        s[1..].split('\'').next().unwrap_or("")
    } else {
        s.split('#').next().unwrap_or("").trim()
    };
    expand_shell_vars(s.trim(), name)
}

/// 从 OpenRC init.d 脚本里提取 description / command / command_args / pidfile。
fn parse_init_script(content: &str, name: &str) -> ScriptMeta {
    let mut m = ScriptMeta::default();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some(eq) = line.find('=') else { continue };
        let key = line[..eq].trim();
        if !matches!(key, "description" | "command" | "command_args" | "pidfile" | "name") {
            continue;
        }
        let val = parse_shell_value(&line[eq + 1..], name);
        match key {
            "description" => m.description = val,
            "command" => m.command = val,
            "command_args" => m.command_args = val,
            "pidfile" => m.pidfile = val,
            // name 只用于显示，description 已兜底，这里忽略
            _ => {}
        }
    }
    m
}

fn first_uint(s: &str) -> Option<u64> {
    let mut cur = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            cur.push(c);
        } else if !cur.is_empty() {
            break;
        }
    }
    if cur.is_empty() { None } else { cur.parse().ok() }
}

fn proc_exists(pid: u32) -> bool {
    pid > 1 && Path::new(&format!("/proc/{}", pid)).is_dir()
}

/// 依次尝试：脚本 pidfile → 常见 pid 路径 → OpenRC daemons 目录 → /proc 按命令名匹配。
fn openrc_find_pid(meta: &ScriptMeta, name: &str, svcdir: &str) -> Option<u32> {
    let mut candidates = Vec::new();
    if !meta.pidfile.is_empty() {
        candidates.push(meta.pidfile.clone());
    }
    candidates.push(format!("/run/{}.pid", name));
    candidates.push(format!("/var/run/{}.pid", name));
    candidates.push(format!("/run/{}/{}.pid", name, name));

    for p in candidates {
        if let Ok(s) = std::fs::read_to_string(&p) {
            if let Some(pid) = first_uint(&s) {
                if pid > 1 && pid < u32::MAX as u64 && proc_exists(pid as u32) {
                    return Some(pid as u32);
                }
            }
        }
    }

    // OpenRC 用 start-stop-daemon 保存的 daemon 信息
    let daemon_dir = format!("{}/daemons/{}", svcdir, name);
    if let Ok(rd) = std::fs::read_dir(&daemon_dir) {
        for e in rd.flatten() {
            if let Ok(s) = std::fs::read_to_string(e.path()) {
                if let Some(pid) = first_uint(&s) {
                    if pid > 1 && pid < u32::MAX as u64 && proc_exists(pid as u32) {
                        return Some(pid as u32);
                    }
                }
            }
        }
    }

    // 兜底：扫描 /proc，按 command 的 basename 匹配 argv[0]
    if !meta.command.is_empty() {
        let cmd_name = Path::new(&meta.command)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !cmd_name.is_empty() {
            if let Ok(rd) = std::fs::read_dir("/proc") {
                for e in rd.flatten() {
                    let pid = match e.file_name().to_string_lossy().parse::<u32>() {
                        Ok(p) => p,
                        Err(_) => continue,
                    };
                    let Ok(cmdline) = std::fs::read(format!("/proc/{}/cmdline", pid)) else {
                        continue;
                    };
                    let argv0 = cmdline.split(|&b| b == 0).next().unwrap_or(&[]);
                    let argv0_str = String::from_utf8_lossy(argv0);
                    let base = Path::new(argv0_str.as_ref())
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    if base == cmd_name {
                        return Some(pid);
                    }
                }
            }
        }
    }
    None
}

/// 从 /proc/<pid>/status 取 (VmRSS 字节数, Threads)。
fn parse_proc_status(s: &str) -> (u64, String) {
    let mut rss = 0u64;
    let mut threads = String::new();
    for line in s.lines() {
        if let Some(v) = line.strip_prefix("VmRSS:") {
            rss = first_uint(v).unwrap_or(0) * 1024;
        } else if let Some(v) = line.strip_prefix("Threads:") {
            threads = v.trim().to_string();
        }
    }
    (rss, threads)
}

/// 从 /proc/<pid>/stat 取 (utime, stime, starttime)，单位均为 clock ticks。
/// 注意 comm 字段带括号且可能含空格，先从最后一个 ')' 之后开始切分。
fn parse_proc_stat(s: &str) -> Option<(u64, u64, u64)> {
    let rp = s.rfind(')')?;
    let f: Vec<&str> = s[rp + 1..].split_whitespace().collect();
    // f[0] = state(字段3)，因此 字段N → f[N-3]
    // utime=14 → f[11], stime=15 → f[12], starttime=22 → f[19]
    Some((
        f.get(11)?.parse().ok()?,
        f.get(12)?.parse().ok()?,
        f.get(19)?.parse().ok()?,
    ))
}

struct ProcInfo {
    rss_bytes: u64,
    threads: String,
    cpu_ticks: u64,
    start_ticks: u64,
}

fn read_proc_info(pid: u32) -> Option<ProcInfo> {
    let status = std::fs::read_to_string(format!("/proc/{}/status", pid)).ok()?;
    let stat = std::fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
    let (rss_bytes, threads) = parse_proc_status(&status);
    let (utime, stime, start_ticks) = parse_proc_stat(&stat).unwrap_or((0, 0, 0));
    Some(ProcInfo {
        rss_bytes,
        threads,
        cpu_ticks: utime.saturating_add(stime),
        start_ticks,
    })
}

fn clk_tck() -> f64 {
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if hz > 0 { hz as f64 } else { 100.0 }
}

fn system_boot_time() -> u64 {
    let s = std::fs::read_to_string("/proc/stat").unwrap_or_default();
    for line in s.lines() {
        if let Some(v) = line.strip_prefix("btime ") {
            return v.trim().parse().unwrap_or(0);
        }
    }
    0
}

fn format_started_at(ts: i64) -> String {
    use time::format_description::well_known::Rfc3339;
    match time::OffsetDateTime::from_unix_timestamp(ts) {
        Ok(dt) => dt.format(&Rfc3339).unwrap_or_default(),
        Err(_) => String::new(),
    }
}

fn openrc_services() -> Result<Vec<SystemdService>, String> {
    let rd = std::fs::read_dir(OPENRC_INIT_DIR)
        .map_err(|e| format!("openrc init.d unavailable: {}", e))?;
    let mut names = Vec::new();
    for e in rd.flatten() {
        let Ok(ft) = e.file_type() else { continue };
        if !ft.is_file() && !ft.is_symlink() {
            continue;
        }
        let n = e.file_name().to_string_lossy().into_owned();
        if valid_service_name(&n) {
            names.push(n);
        }
    }
    names.sort();

    let svcdir = openrc_svcdir();
    let boot = system_boot_time();
    let hz = clk_tck();
    let mut out = Vec::with_capacity(names.len());

    for name in names {
        let script_path = format!("{}/{}", OPENRC_INIT_DIR, name);
        let content = std::fs::read_to_string(&script_path).unwrap_or_default();
        let meta = parse_init_script(&content, &name);
        let (active, sub) = openrc_state(svcdir, &name);

        let exec_start = if meta.command.is_empty() {
            String::new()
        } else if meta.command_args.is_empty() {
            meta.command.clone()
        } else {
            format!("{} {}", meta.command, meta.command_args)
        };

        let mut svc = SystemdService {
            unit: name.clone(),
            load: "loaded".into(),
            active: active.into(),
            sub: sub.into(),
            description: if meta.description.is_empty() {
                name.clone()
            } else {
                meta.description.clone()
            },
            unit_file_state: if openrc_enabled(&name) {
                "enabled".into()
            } else {
                "disabled".into()
            },
            exec_start,
            fragment_path: script_path,
            ..Default::default()
        };

        // 只有运行中/崩溃的服务才有 PID 与资源数据
        if active == "active" || active == "failed" {
            if let Some(pid) = openrc_find_pid(&meta, &name, svcdir) {
                svc.main_pid = pid.to_string();
                if let Some(info) = read_proc_info(pid) {
                    svc.memory_bytes = info.rss_bytes;
                    svc.memory = format_bytes(info.rss_bytes);
                    svc.cpu_ns = (info.cpu_ticks as f64 * 1_000_000_000.0 / hz) as u64;
                    svc.cpu_time = format_nsec(svc.cpu_ns);
                    svc.tasks = info.threads;
                    if boot > 0 && info.start_ticks > 0 {
                        let ts = boot as i64 + (info.start_ticks as f64 / hz) as i64;
                        svc.started_at = format_started_at(ts);
                    }
                }
            }
        }
        out.push(svc);
    }
    Ok(out)
}

fn tail_lines(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

/// Alpine 没有 journalctl，尝试常见 syslog 文件；找不到时给出可读提示而非报错。
fn openrc_logs(unit: &str, lines: i32) -> Result<String, String> {
    let n = if lines > 0 { lines as usize } else { 200 };
    let candidates = [
        format!("/var/log/{}.log", unit),
        format!("/var/log/{}/{}.log", unit, unit),
        "/var/log/messages".to_string(),
        "/var/log/syslog".to_string(),
    ];
    for p in &candidates {
        if let Ok(s) = std::fs::read_to_string(p) {
            return Ok(tail_lines(&s, n));
        }
    }
    Ok(format!(
        "OpenRC 服务日志由系统日志服务管理，未找到 {} 的日志文件。\n已尝试：{}",
        unit,
        candidates.join("、")
    ))
}

// ══════════════════════════════════════════════════════════════════
// 公开 API（按 init 系统分发）
// ══════════════════════════════════════════════════════════════════

pub fn get_services() -> Result<Vec<SystemdService>, String> {
    match init_system() {
        InitSystem::Systemd => systemd_services(),
        InitSystem::OpenRc => openrc_services(),
    }
}

pub fn service_action(unit: &str, action: &str) -> Result<(), String> {
    match init_system() {
        InitSystem::Systemd => cmd::run("systemctl", &[action, unit], Duration::from_secs(30)),
        InitSystem::OpenRc => match action {
            "start" | "stop" | "restart" => {
                cmd::run("rc-service", &[unit, action], Duration::from_secs(30))
            }
            "enable" => cmd::run("rc-update", &["add", unit, "default"], Duration::from_secs(30)),
            "disable" => cmd::run("rc-update", &["del", unit, "default"], Duration::from_secs(30)),
            _ => Err(format!("unsupported action: {}", action)),
        },
    }
}

pub fn get_service_logs(unit: &str, lines: i32) -> Result<String, String> {
    if init_system() == InitSystem::OpenRc {
        return openrc_logs(unit, lines);
    }
    let n = lines.to_string();
    let (out, err) = cmd::combined(
        "journalctl",
        &["-u", unit, "-n", &n, "--no-pager", "--output=short-iso"],
        Duration::from_secs(10),
    )?;
    match err {
        Some(e) => Err(e),
        None => Ok(lossy(&out)),
    }
}

/// 排序：sortBy 为空不排序；"memory"/"cpu" 按对应数值；其它键保持原序。
pub fn sort_services(svcs: &mut [SystemdService], sort_by: &str, dir: &str) {
    if sort_by.is_empty() {
        return;
    }
    if sort_by != "memory" && sort_by != "cpu" {
        return;
    }
    let asc = dir == "asc";
    svcs.sort_by(|a, b| {
        let (vi, vj) = match sort_by {
            "memory" => (a.memory_bytes, b.memory_bytes),
            "cpu" => (a.cpu_ns, b.cpu_ns),
            _ => (0, 0),
        };
        let ord = vi.cmp(&vj);
        if asc { ord } else { ord.reverse() }
    });
}

/// Go 版提供但未注册路由的辅助函数，保留等价实现。
/// OpenRC 下映射到 /etc/init.d/<unit>。
#[allow(dead_code)]
pub fn read_service_file(unit: &str) -> Result<(String, String), String> {
    if init_system() == InitSystem::OpenRc {
        let path = format!("{}/{}", OPENRC_INIT_DIR, unit);
        let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        return Ok((content, path));
    }
    let out = cmd::output(
        "systemctl",
        &["show", unit, "--no-pager", "--property=FragmentPath"],
        Duration::from_secs(3),
    )?;
    let kv = parse_kv(&lossy(&out));
    let path = kv.get("FragmentPath").cloned().unwrap_or_default();
    if path.is_empty() {
        return Err(format!("no FragmentPath for unit {}", unit));
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok((content, path))
}

#[allow(dead_code)]
pub fn write_service_file(unit: &str, content: &str) -> Result<(), String> {
    if init_system() == InitSystem::OpenRc {
        let path = format!("{}/{}", OPENRC_INIT_DIR, unit);
        return std::fs::write(&path, content).map_err(|e| e.to_string());
    }
    let out = cmd::output(
        "systemctl",
        &[
            "show",
            unit,
            "--no-pager",
            "--property=FragmentPath,ActiveState",
        ],
        Duration::from_secs(3),
    )?;
    let kv = parse_kv(&lossy(&out));
    let path = kv.get("FragmentPath").cloned().unwrap_or_default();
    if path.is_empty() {
        return Err(format!("no FragmentPath for {}", unit));
    }
    let was_active = kv.get("ActiveState").map(|s| s.as_str()) == Some("active");
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    cmd::run("systemctl", &["daemon-reload"], Duration::from_secs(15))?;
    if was_active {
        let _ = cmd::run("systemctl", &["restart", unit], Duration::from_secs(30));
    }
    Ok(())
}

// ══════════════════════════════════════════════════════════════════
// 单元测试（OpenRC 解析逻辑，不依赖真实 Alpine 环境）
// ══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_init_script() {
        let script = r#"#!/sbin/openrc-run
# comment
name="chronyd"
description="Chrony time daemon"
command=/usr/sbin/chronyd
command_args="-f /etc/chrony/chrony.conf"
pidfile="/run/${RC_SVCNAME}.pid"

depend() {
    need net
}

start_pre() {
    checkpath --directory /run/chrony
}
"#;
        let m = parse_init_script(script, "chronyd");
        assert_eq!(m.description, "Chrony time daemon");
        assert_eq!(m.command, "/usr/sbin/chronyd");
        assert_eq!(m.command_args, "-f /etc/chrony/chrony.conf");
        assert_eq!(m.pidfile, "/run/chronyd.pid");
    }

    #[test]
    fn test_parse_init_script_single_quote_and_comment() {
        let script = "description='SSH daemon'\ncommand=/usr/sbin/sshd  # inline comment\n";
        let m = parse_init_script(script, "sshd");
        assert_eq!(m.description, "SSH daemon");
        assert_eq!(m.command, "/usr/sbin/sshd");
    }

    #[test]
    fn test_parse_proc_status() {
        let s = "Name:\tfoo\nVmRSS:\t  20480 kB\nThreads:\t7\nVmSize:\t 100000 kB\n";
        let (rss, threads) = parse_proc_status(s);
        assert_eq!(rss, 20480 * 1024);
        assert_eq!(threads, "7");
    }

    #[test]
    fn test_parse_proc_stat_with_spaces_in_comm() {
        // /proc/<pid>/stat：字段 3=state，14=utime，15=stime，22=starttime。
        // comm 用括号包住且含空格，验证从最后一个 ')' 之后切分的正确性。
        let s = "1234 (my daemon) S 1 2 3 4 5 6 7 8 9 10 500 250 16 17 18 19 20 21 987654 23 24";
        let (u, st, start) = parse_proc_stat(s).unwrap();
        assert_eq!(u, 500);
        assert_eq!(st, 250);
        assert_eq!(start, 987654);
    }

    #[test]
    fn test_valid_service_name() {
        assert!(valid_service_name("chronyd"));
        assert!(valid_service_name("sshd"));
        assert!(!valid_service_name(".hidden"));
        assert!(!valid_service_name("foo.bak"));
        assert!(!valid_service_name("README"));
        assert!(!valid_service_name("bar~"));
    }

    #[test]
    fn test_tail_lines() {
        let s = "a\nb\nc\nd";
        assert_eq!(tail_lines(s, 2), "c\nd");
        assert_eq!(tail_lines(s, 10), "a\nb\nc\nd");
    }

    #[test]
    fn test_openrc_state_with_temp_dir() {
        let base = std::env::temp_dir().join(format!("helm-openrc-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(base.join("started"));
        let _ = std::fs::create_dir_all(base.join("failed"));
        let _ = std::fs::write(base.join("started/chronyd"), "");
        let _ = std::fs::write(base.join("failed/crond"), "");
        let d = base.to_string_lossy().into_owned();
        assert_eq!(openrc_state(&d, "chronyd"), ("active", "running"));
        assert_eq!(openrc_state(&d, "crond"), ("failed", "crashed"));
        assert_eq!(openrc_state(&d, "sshd"), ("inactive", "stopped"));
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn test_format_helpers() {
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_nsec(90_000_000_000), "1.5m");
        assert_eq!(format_nsec(5_000_000_000), "5.0s");
    }
}

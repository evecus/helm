//! systemd 采集与操作，对应 Go 版 internal/collector/systemd.go。

use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

use crate::cmd;

#[derive(Clone, Debug, Default, Serialize)]
pub struct SystemdService {
    pub unit: String,
    pub load: String,
    pub active: String,
    pub sub: String,
    pub description: String,
    // 详细信息（通过 systemctl show 获取）
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

pub fn get_services() -> Result<Vec<SystemdService>, String> {
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

pub fn service_action(unit: &str, action: &str) -> Result<(), String> {
    cmd::run("systemctl", &[action, unit], Duration::from_secs(30))
}

pub fn get_service_logs(unit: &str, lines: i32) -> Result<String, String> {
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
#[allow(dead_code)]
pub fn read_service_file(unit: &str) -> Result<(String, String), String> {
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

//! 进程采集，对应 Go 版 internal/collector/process.go（gopsutil/process → sysinfo）。

use serde::Serialize;
use sysinfo::{Pid, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, System, Users};

#[derive(Clone, Debug, Default, Serialize)]
pub struct ProcessInfo {
    pub pid: i32,
    pub name: String,
    pub username: String,
    pub cpu_percent: f64,
    pub mem_percent: f32,
    pub mem_rss: u64,
    pub status: String,
    pub cmdline: String,
}

/// 映射 sysinfo 的状态到 gopsutil convertStatusChar 的字符串。
fn status_str(s: ProcessStatus) -> &'static str {
    match s {
        ProcessStatus::Idle => "idle",
        ProcessStatus::Run => "running",
        ProcessStatus::Sleep => "sleep",
        ProcessStatus::Stop | ProcessStatus::Tracing | ProcessStatus::Suspended => "stop",
        ProcessStatus::Zombie => "zombie",
        ProcessStatus::Dead | ProcessStatus::UninterruptibleDiskSleep => "blocked",
        ProcessStatus::Wakekill => "wait",
        ProcessStatus::Waking => "system",
        ProcessStatus::Parked | ProcessStatus::LockBlocked => "lock",
        _ => "",
    }
}

pub fn get_processes(sort_by: &str, sort_dir: &str, limit: i32) -> Result<Vec<ProcessInfo>, String> {
    let mut sys = System::new();
    sys.refresh_memory();
    // gopsutil 会列出主进程（不含线程），且需要 user/cmd 字段
    let kind = ProcessRefreshKind::everything().without_tasks();
    // sysinfo 的 cpu_usage 需要两次刷新之间的间隔才有意义
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, kind.clone());
    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);

    let total_mem = sys.total_memory() as f64;
    let users = Users::new_with_refreshed_list();

    let mut infos: Vec<ProcessInfo> = sys
        .processes()
        .values()
        .map(|p| {
            let username = p
                .user_id()
                .and_then(|uid| users.list().iter().find(|u| u.id() == uid))
                .map(|u| u.name().to_string())
                .unwrap_or_default();
            let rss = p.memory();
            let cmdline = p
                .cmd()
                .iter()
                .map(|s| s.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ");
            let mem_percent = if total_mem > 0.0 {
                (rss as f64 / total_mem * 100.0) as f32
            } else {
                0.0
            };
            ProcessInfo {
                pid: p.pid().as_u32() as i32,
                name: p.name().to_string_lossy().into_owned(),
                username,
                cpu_percent: p.cpu_usage() as f64,
                mem_percent,
                mem_rss: rss,
                status: status_str(p.status()).to_string(),
                cmdline,
            }
        })
        .collect();

    let asc = sort_dir == "asc";
    if sort_by == "mem" {
        infos.sort_by(|a, b| {
            let ord = a
                .mem_percent
                .partial_cmp(&b.mem_percent)
                .unwrap_or(std::cmp::Ordering::Equal);
            if asc { ord } else { ord.reverse() }
        });
    } else {
        infos.sort_by(|a, b| {
            let ord = a
                .cpu_percent
                .partial_cmp(&b.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal);
            if asc { ord } else { ord.reverse() }
        });
    }

    if limit > 0 && (limit as usize) < infos.len() {
        infos.truncate(limit as usize);
    }
    Ok(infos)
}

/// 对应 gopsutil process.NewProcess(pid).Kill()：SIGKILL。
pub fn kill_process(pid: i32) -> Result<(), String> {
    let target = Pid::from_u32(pid as u32);
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::Some(&[target]), true);
    match sys.process(target) {
        Some(p) => {
            if p.kill() {
                Ok(())
            } else {
                Err("kill failed".into())
            }
        }
        None => Err("process does not exist".into()),
    }
}

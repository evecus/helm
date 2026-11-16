//! 进程采集，对应 Go 版 internal/collector/process.go（gopsutil/process → sysinfo）。

use serde::Serialize;
use sysinfo::{Pid, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, System, UpdateKind, Users};

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
    // 只取 Go 版实际用到的字段（累计 CPU 时间 / RSS / user / cmdline）。
    // everything() 还会额外读 environ、exe、cwd、root、disk_usage，
    // 在进程数多的机器上开销大得多。
    let kind = ProcessRefreshKind::nothing()
        .with_cpu()
        .with_memory()
        .with_user(UpdateKind::OnlyIfNotSet)
        .with_cmd(UpdateKind::OnlyIfNotSet)
        .without_tasks();
    // gopsutil 的 p.CPUPercent() 是「累计 CPU 时间 / 进程运行时长」，
    // 属生命周期平均值，不需要两次刷新 + sleep，一次刷新即可。
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
                // 对应 gopsutil p.CPUPercent()：累计 CPU 秒数 / 已运行墙钟秒数。
                // 不再用 sysinfo 的瞬时窗口，避免采集时把 helm 自己算成高占用。
                cpu_percent: {
                    let run = p.run_time() as f64;
                    if run > 0.0 {
                        (p.accumulated_cpu_time() as f64 / 1000.0) / run * 100.0
                    } else {
                        0.0
                    }
                },
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

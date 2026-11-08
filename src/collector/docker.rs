//! Docker 采集与操作，对应 Go 版 internal/collector/docker.go。
//! 全部通过 docker CLI 子进程完成，命令、参数、超时与输出格式保持一致。

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::cmd;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub state: String,
    pub ports: String,
    pub created: String,
    pub cpu_percent: f64,
    pub mem_percent: f64,
    pub mem_used: u64,
    pub mem_limit: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct InspectResult {
    pub image: String,
    pub status: String,
    pub created: String,
    pub restart_policy: String,
    pub env: Vec<String>,
    pub mounts: Vec<String>,
    pub networks: Vec<String>,
    pub ports: String,
    pub cmd: Vec<String>,
    pub compose_file: String,
}

fn lossy(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// 解析 docker stats 的 "128MiB / 2GiB" 之类字符串。
/// Go 版用 map 遍历后缀匹配（顺序不稳定）；这里按后缀长度降序，结果稳定正确。
fn parse_mem_str(s: &str) -> u64 {
    let s = s.trim();
    let multipliers: [(&str, f64); 7] = [
        ("KiB", 1024.0),
        ("MiB", 1024.0 * 1024.0),
        ("GiB", 1024.0 * 1024.0 * 1024.0),
        ("KB", 1000.0),
        ("MB", 1000.0 * 1000.0),
        ("GB", 1000.0 * 1000.0 * 1000.0),
        ("B", 1.0),
    ];
    for (suffix, mult) in multipliers {
        if let Some(v) = s.strip_suffix(suffix) {
            return (v.trim().parse::<f64>().unwrap_or(0.0) * mult) as u64;
        }
    }
    s.parse::<u64>().unwrap_or(0)
}

pub fn get_containers() -> Result<Vec<Container>, String> {
    // docker ps -a --format '{...}'（每行一个 JSON）
    let fmt = r#"{"id":"{{.ID}}","name":"{{.Names}}","image":"{{.Image}}","status":"{{.Status}}","state":"{{.State}}","ports":"{{.Ports}}","created":"{{.CreatedAt}}"}"#;
    let out = cmd::output(
        "docker",
        &["ps", "-a", "--format", fmt],
        Duration::from_secs(10),
    )
    .map_err(|e| format!("docker not available: {}", e))?;

    let mut containers: Vec<Container> = Vec::new();
    for line in lossy(&out).trim().lines() {
        if line.is_empty() {
            continue;
        }
        if let Ok(c) = serde_json::from_str::<Container>(line) {
            containers.push(c);
        }
    }

    // Enrich with stats (non-blocking)：stats 失败时忽略，不影响容器列表
    let stats_fmt = r"{{.ID}}\t{{.CPUPerc}}\t{{.MemPerc}}\t{{.MemUsage}}";
    if let Ok(stats_out) = cmd::output(
        "docker",
        &["stats", "--no-stream", "--format", stats_fmt],
        Duration::from_secs(8),
    ) {
        let mut stats_map = std::collections::HashMap::new();
        for line in lossy(&stats_out).trim().lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 4 {
                continue;
            }
            let cpu = parts[1].trim_end_matches('%').trim().parse::<f64>().unwrap_or(0.0);
            let mem_pct = parts[2].trim_end_matches('%').trim().parse::<f64>().unwrap_or(0.0);
            let mem_parts: Vec<&str> = parts[3].split(" / ").collect();
            let (used, lim) = if mem_parts.len() == 2 {
                (parse_mem_str(mem_parts[0]), parse_mem_str(mem_parts[1]))
            } else {
                (0, 0)
            };
            stats_map.insert(parts[0].to_string(), (cpu, mem_pct, used, lim));
        }
        for c in containers.iter_mut() {
            if let Some((cpu, mem_pct, used, lim)) = stats_map.get(&c.id) {
                c.cpu_percent = *cpu;
                c.mem_percent = *mem_pct;
                c.mem_used = *used;
                c.mem_limit = *lim;
            }
        }
    }
    Ok(containers)
}

pub fn container_action(id: &str, action: &str) -> Result<(), String> {
    cmd::run("docker", &[action, id], Duration::from_secs(30))
}

pub fn get_container_logs(id: &str, lines: i32) -> Result<String, String> {
    let n = lines.to_string();
    let (out, err) = cmd::combined(
        "docker",
        &["logs", "--tail", &n, id],
        Duration::from_secs(10),
    )?;
    match err {
        Some(e) => Err(e),
        None => Ok(lossy(&out)),
    }
}

fn file_exists(path: &str) -> bool {
    std::path::Path::new(path).is_file()
}

pub fn inspect_container(id: &str) -> Result<InspectResult, String> {
    let out = cmd::output("docker", &["inspect", id], Duration::from_secs(10))
        .map_err(|e| format!("docker inspect failed: {}", e))?;

    let raw: Vec<serde_json::Value> = serde_json::from_slice(&out)
        .map_err(|e| format!("parse error: {} (output: {:.300})", e, lossy(&out)))?;
    if raw.is_empty() {
        return Err(format!("parse error: (output: {:.300})", lossy(&out)));
    }
    let c = &raw[0];
    let mut r = InspectResult::default();

    if let Some(cfg) = c.get("Config") {
        r.image = cfg.get("Image").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if let Some(env) = cfg.get("Env").and_then(|v| v.as_array()) {
            for e in env {
                if let Some(s) = e.as_str() {
                    r.env.push(s.to_string());
                }
            }
        }
        if let Some(cmdv) = cfg.get("Cmd").and_then(|v| v.as_array()) {
            for v in cmdv {
                if let Some(s) = v.as_str() {
                    r.cmd.push(s.to_string());
                }
            }
        }
        if let Some(wdir) = cfg
            .get("Labels")
            .and_then(|v| v.get("com.docker.compose.project.working_dir"))
            .and_then(|v| v.as_str())
        {
            if !wdir.is_empty() {
                for name in ["docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"] {
                    let p = format!("{}/{}", wdir, name);
                    if file_exists(&p) {
                        r.compose_file = p;
                        break;
                    }
                }
                if r.compose_file.is_empty() {
                    r.compose_file = format!("{}/docker-compose.yml", wdir);
                }
            }
        }
    }

    r.status = c
        .get("State")
        .and_then(|v| v.get("Status"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    r.created = c.get("Created").and_then(|v| v.as_str()).unwrap_or("").to_string();

    if let Some(hc) = c.get("HostConfig") {
        r.restart_policy = hc
            .get("RestartPolicy")
            .and_then(|v| v.get("Name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if let Some(pb) = hc.get("PortBindings").and_then(|v| v.as_object()) {
            let mut parts = Vec::new();
            for (container_port, bindings) in pb {
                if let Some(list) = bindings.as_array() {
                    for b in list {
                        let host_port = b.get("HostPort").and_then(|v| v.as_str()).unwrap_or("");
                        let mut host_ip = b.get("HostIp").and_then(|v| v.as_str()).unwrap_or("");
                        if host_ip.is_empty() {
                            host_ip = "0.0.0.0";
                        }
                        parts.push(format!("{}:{}->{}", host_ip, host_port, container_port));
                    }
                }
            }
            r.ports = parts.join(", ");
        }
    }

    if let Some(mounts) = c.get("Mounts").and_then(|v| v.as_array()) {
        for m in mounts {
            let src = m.get("Source").and_then(|v| v.as_str()).unwrap_or("");
            let dst = m.get("Destination").and_then(|v| v.as_str()).unwrap_or("");
            r.mounts.push(format!("{}->{}", src, dst));
        }
    }

    if let Some(networks) = c
        .get("NetworkSettings")
        .and_then(|v| v.get("Networks"))
        .and_then(|v| v.as_object())
    {
        for name in networks.keys() {
            r.networks.push(name.clone());
        }
    }

    Ok(r)
}

pub fn read_compose_file(path: &str) -> Result<String, String> {
    let out = cmd::output("cat", &[path], Duration::from_secs(5))
        .map_err(|_| "cannot read file".to_string())?;
    Ok(lossy(&out))
}

/// 写回 compose 文件并 docker compose down/up。返回 (日志, 结果)。
pub fn write_and_apply_compose(
    path: &str,
    content: &str,
    _container_id: &str,
) -> (String, Result<(), String>) {
    // Go 版：exec.Command("bash", "-c", "cat > <path>")，stdin = content
    let write = (|| -> Result<Vec<u8>, String> {
        let mut child = Command::new("bash")
            .arg("-c")
            .arg(format!("cat > {}", path))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        if let Some(mut sin) = child.stdin.take() {
            sin.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
        }
        let out = child.wait_with_output().map_err(|e| e.to_string())?;
        let mut buf = out.stdout;
        buf.extend_from_slice(&out.stderr);
        if !out.status.success() {
            return Err(format!("write failed: {}", lossy(&buf)));
        }
        Ok(buf)
    })();
    if let Err(e) = write {
        return (String::new(), Err(e));
    }

    let dir = match path.rfind('/') {
        Some(i) => &path[..i],
        None => ".",
    };
    let dir = if dir.is_empty() { "." } else { dir };

    let timeout = Duration::from_secs(60);
    let (down_out, _) =
        cmd::combined("docker", &["compose", "-f", path, "down"], timeout).unwrap_or((Vec::new(), None));
    let (up_out, up_err) =
        cmd::combined("docker", &["compose", "-f", path, "up", "-d"], timeout).unwrap_or((Vec::new(), None));

    let log = format!(
        "=== Working dir: {} ===\n\n=== docker compose down ===\n{}\n=== docker compose up -d ===\n{}",
        dir,
        lossy(&down_out),
        lossy(&up_out)
    );
    match up_err {
        Some(_) => (log.clone(), Err(format!("compose up failed: {}", lossy(&up_out)))),
        None => (log, Ok(())),
    }
}

/// 拉取镜像并更新容器，返回 (日志, 结果)。
pub fn pull_and_update_container(id: &str) -> (String, Result<(), String>) {
    let timeout = Duration::from_secs(300);
    let img_out = match cmd::output(
        "docker",
        &["inspect", "--format", "{{.Config.Image}}", id],
        timeout,
    ) {
        Ok(v) => v,
        Err(_) => return (String::new(), Err("cannot get image".into())),
    };
    let image = lossy(&img_out).trim().to_string();
    let mut log = format!("=== Pulling image: {} ===\n", image);

    let (pull_out, _) =
        cmd::combined("docker", &["pull", &image], timeout).unwrap_or((Vec::new(), None));
    log.push_str(&lossy(&pull_out));

    let label_fmt = r#"{{index .Config.Labels "com.docker.compose.project.working_dir"}}"#;
    let (labels_out, _) = cmd::combined(
        "docker",
        &["inspect", "--format", label_fmt, id],
        timeout,
    )
    .unwrap_or((Vec::new(), None));
    let wdir = lossy(&labels_out).trim().to_string();

    if !wdir.is_empty() {
        log.push_str(&format!("\n=== Detected docker-compose project at: {} ===\n", wdir));
        let mut compose_path = String::new();
        for name in ["docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"] {
            let p = format!("{}/{}", wdir, name);
            if file_exists(&p) {
                compose_path = p;
                break;
            }
        }
        if compose_path.is_empty() {
            compose_path = format!("{}/docker-compose.yml", wdir);
        }
        log.push_str(&format!("Compose file: {}\n\n", compose_path));

        let (down_out, _) = cmd::combined(
            "docker",
            &["compose", "-f", &compose_path, "down"],
            timeout,
        )
        .unwrap_or((Vec::new(), None));
        log.push_str("=== docker compose down ===\n");
        log.push_str(&lossy(&down_out));

        let (up_out, up_err) = cmd::combined(
            "docker",
            &["compose", "-f", &compose_path, "up", "-d"],
            timeout,
        )
        .unwrap_or((Vec::new(), None));
        log.push_str("\n=== docker compose up -d ===\n");
        log.push_str(&lossy(&up_out));
        if up_err.is_some() {
            return (log, Err("compose up failed".into()));
        }
    } else {
        log.push_str(&format!("\n=== Recreating container: {} ===\n", id));
        let (stop_out, _) =
            cmd::combined("docker", &["stop", id], timeout).unwrap_or((Vec::new(), None));
        log.push_str(&lossy(&stop_out));
        let (rm_out, _) = cmd::combined("docker", &["rm", id], timeout).unwrap_or((Vec::new(), None));
        log.push_str(&lossy(&rm_out));
        log.push_str("Container stopped and removed. Please restart manually or redeploy with original run command.");
    }
    (log, Ok(()))
}
